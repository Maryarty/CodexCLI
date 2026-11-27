//! Compile a spritesheet once into the immutable PNG/APNG sources consumed by rtwk.
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use codex_config::types::TuiPetAnchor;
use codex_core::config::Config;
use codex_terminal_detection::terminal_info;
use codex_tui_core::app::effects::ImageSize;
use codex_tui_core::screens::pets::{Ambient, Anchor, Clip, DISABLED, Motion};
use codex_tui_rtwk::services::agents::workspace::blocking;
use image::{RgbaImage, imageops};

use super::{BUILTIN, Grid, Manifest, ensure_builtin, installed_asset, support};

const MAX_FRAMES: usize = 256;
const MAX_EDGE: u32 = 256;
const MAX_ENCODED: usize = 16 * 1024 * 1024;

pub(super) async fn load(
    config: Option<Arc<Config>>,
    selector: Option<String>,
) -> Result<Option<Ambient>> {
    let Some(config) = config else {
        return Ok(None);
    };
    let Some(selector) = selector.or_else(|| config.tui_pet.clone()) else {
        return Ok(None);
    };
    if selector == DISABLED || support(&terminal_info(), true).is_err() {
        return Ok(None);
    }
    let anchor = match config.tui_pet_anchor {
        TuiPetAnchor::Composer => Anchor::Composer,
        TuiPetAnchor::ScreenBottom => Anchor::ScreenBottom,
    };
    let animations = config.animations;
    let builtin = BUILTIN.iter().find(|pet| pet.id == selector);
    let sheet = match builtin {
        Some(pet) => Some(ensure_builtin(&config, pet).await?),
        None => None,
    };
    blocking(move || {
        let (manifest, sheet) = match sheet {
            Some(sheet) => (Manifest::default(), sheet),
            None => installed_asset(config.codex_home.as_path(), &selector)?,
        };
        compile(&sheet, &manifest, anchor, animations).map(Some)
    })
    .await
    .context("prepare ambient pet")?
}

fn compile(sheet: &Path, manifest: &Manifest, anchor: Anchor, animated: bool) -> Result<Ambient> {
    let grid = manifest.frame.unwrap_or_default();
    let sheet = image::open(sheet)?.into_rgba8();
    let size = ImageSize {
        width: u16::try_from(grid.width)?,
        height: u16::try_from(grid.height)?,
    };
    let mut clips = Vec::with_capacity(Motion::ALL.len());
    let mut remaining = MAX_ENCODED;
    for motion in Motion::ALL {
        let mut animation = timeline(manifest, grid, motion)?;
        if !animated {
            animation.frames.truncate(1);
            animation.plays = 0;
        }
        let clip = encode(&sheet, grid, &animation, remaining)?;
        remaining -= clip.data.len();
        clips.push(clip);
    }
    Ok(Ambient {
        size,
        anchor,
        clips: clips.try_into().ok().context("incomplete ambient clips")?,
    })
}

struct Timeline {
    /// Repeats of the same encoded frames. Zero is the APNG indefinite repeat.
    plays: u32,
    frames: Vec<(u32, u16)>,
}

fn timeline(manifest: &Manifest, grid: Grid, motion: Motion) -> Result<Timeline> {
    let count = grid.columns * grid.rows;
    if let Some(spec) = manifest.animations.get(motion.name()) {
        let fps = spec.fps.unwrap_or(8.0);
        if spec.frames.is_empty()
            || spec.frames.len() > MAX_FRAMES
            || spec.frames.iter().any(|&frame| frame >= count)
            || !(0.0 < fps && fps <= 60.0)
        {
            bail!("invalid animation {}", motion.name());
        }
        let delay = (1000.0 / fps).round().clamp(1.0, f64::from(u16::MAX)) as u16;
        return Ok(Timeline {
            plays: if spec.loop_animation.unwrap_or(true) {
                0
            } else {
                1
            },
            frames: spec.frames.iter().map(|&frame| (frame, delay)).collect(),
        });
    }
    let mut frames: Vec<_> = match motion {
        Motion::Idle => [(0, 1680), (1, 660), (2, 660), (3, 840), (4, 840), (5, 1920)].into(),
        motion => {
            let (row, frame_count, delay, last_delay) = match motion {
                Motion::Running => (7, 6, 120, 220),
                Motion::Waiting => (6, 6, 150, 260),
                Motion::Review => (8, 6, 150, 280),
                Motion::Failed => (5, 8, 140, 240),
                Motion::Idle => unreachable!(),
            };
            (0..frame_count)
                .map(|column| {
                    let time = if column + 1 == frame_count {
                        last_delay
                    } else {
                        delay
                    };
                    (row * 8 + column, time)
                })
                .collect()
        }
    };
    frames.retain(|&(frame, _)| frame < count);
    if frames.is_empty() {
        return timeline(manifest, grid, Motion::Idle);
    }
    Ok(Timeline {
        plays: if motion == Motion::Idle { 0 } else { 3 },
        frames,
    })
}

fn encode(sheet: &RgbaImage, grid: Grid, animation: &Timeline, budget: usize) -> Result<Clip> {
    let max_edge = grid.width.max(grid.height);
    let width = (grid.width * MAX_EDGE / max_edge.max(MAX_EDGE)).max(1);
    let height = (grid.height * MAX_EDGE / max_edge.max(MAX_EDGE)).max(1);
    let mut encoded = Vec::new();
    let mut encoder = png::Encoder::new(&mut encoded, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_compression(png::Compression::Fast);
    let animated = animation.frames.len() > 1 || animation.plays > 0;
    if animated {
        encoder.set_animated(u32::try_from(animation.frames.len())?, animation.plays)?;
    }
    let mut writer = encoder.write_header()?;
    let mut elapsed = 0;
    for &(frame, delay) in &animation.frames {
        let crop = imageops::crop_imm(
            sheet,
            frame % grid.columns * grid.width,
            frame / grid.columns * grid.height,
            grid.width,
            grid.height,
        );
        let pixels = if width == grid.width && height == grid.height {
            crop.to_image()
        } else {
            imageops::resize(&*crop, width, height, imageops::FilterType::Triangle)
        };
        if animated {
            writer.set_frame_delay(delay, 1000)?;
        }
        writer.write_image_data(pixels.as_raw())?;
        elapsed += u64::from(delay);
    }
    writer.finish()?;
    if encoded.len() > budget {
        bail!("ambient pet exceeds its PNG storage limit");
    }
    Ok(Clip {
        data: encoded.into(),
        until_idle: (animation.plays > 0)
            .then(|| Duration::from_millis(elapsed * u64::from(animation.plays))),
    })
}
