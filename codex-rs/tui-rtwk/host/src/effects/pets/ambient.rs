//! Compile a spritesheet once into the immutable PNG/APNG sources consumed by rtwk.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use codex_config::types::TuiPetAnchor;
use codex_core::config::Config;
use codex_terminal_detection::terminal_info;
use codex_tui_core::app::effects::ImageSize;
use codex_tui_core::screens::pets::{Ambient, Anchor, Clip, ClipTransition, DISABLED, Motion};
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
    let mut compiler = Compiler {
        sheet: &sheet,
        manifest,
        grid,
        animated,
        remaining: MAX_ENCODED,
        sources: BTreeMap::new(),
    };
    let clips = Motion::ALL
        .into_iter()
        .map(|motion| compiler.clip(motion.name(), true))
        .collect::<Result<Vec<_>>>()?;
    Ok(Ambient {
        size,
        anchor,
        clips: clips.try_into().ok().context("incomplete ambient clips")?,
    })
}

#[derive(Clone)]
struct Encoded {
    data: Arc<[u8]>,
    after: Option<(Duration, String)>,
}

struct Compiler<'a> {
    sheet: &'a RgbaImage,
    manifest: &'a Manifest,
    grid: Grid,
    animated: bool,
    remaining: usize,
    sources: BTreeMap<String, Encoded>,
}

impl Compiler<'_> {
    fn source(&mut self, name: &str) -> Result<Encoded> {
        if let Some(source) = self.sources.get(name) {
            return Ok(source.clone());
        }
        let mut timeline = timeline(self.manifest, self.grid, name)?;
        if !self.animated {
            timeline.frames.truncate(1);
            timeline.plays = 0;
        }
        let source = encode(self.sheet, self.grid, &timeline, self.remaining)?;
        self.remaining -= source.data.len();
        self.sources.insert(name.to_owned(), source.clone());
        Ok(source)
    }

    fn clip(&mut self, name: &str, allow_manifest_fallback: bool) -> Result<Clip> {
        let Encoded { data, after } = self.source(name)?;
        let after = match after {
            Some((delay, next))
                if next != name
                    && (allow_manifest_fallback
                        || !self.manifest.animations.contains_key(name)) =>
            {
                // The manifest defines one fallback. A stock fallback has its own idle tail.
                Some(ClipTransition {
                    delay,
                    next: Arc::new(self.clip(&next, false)?),
                })
            }
            _ => None,
        };
        Ok(Clip { data, after })
    }
}

struct Timeline {
    /// Repeats of the same encoded frames. Zero is the APNG indefinite repeat.
    plays: u32,
    frames: Vec<(u32, u16)>,
    fallback: String,
}

fn timeline(manifest: &Manifest, grid: Grid, name: &str) -> Result<Timeline> {
    let count = grid.columns * grid.rows;
    if let Some(spec) = manifest.animations.get(name) {
        let fps = spec.fps.unwrap_or(8.0);
        if spec.frames.is_empty()
            || spec.frames.len() > MAX_FRAMES
            || spec.frames.iter().any(|&frame| frame >= count)
            || !(0.0 < fps && fps <= 60.0)
        {
            bail!("invalid animation {name}");
        }
        let delay = (1000.0 / fps).round().clamp(1.0, f64::from(u16::MAX)) as u16;
        return Ok(Timeline {
            plays: if spec.loop_animation.unwrap_or(true) {
                0
            } else {
                1
            },
            frames: spec.frames.iter().map(|&frame| (frame, delay)).collect(),
            fallback: if spec.fallback.is_empty() {
                "idle".into()
            } else {
                spec.fallback.clone()
            },
        });
    }
    let mut frames: Vec<_> = if name == "idle" {
        [(0, 1680), (1, 660), (2, 660), (3, 840), (4, 840), (5, 1920)].into()
    } else {
        let (row, frame_count, delay, last_delay) = match name {
            "running-right" | "move_right" => (1, 8, 120, 220),
            "running-left" | "move_left" => (2, 8, 120, 220),
            "waving" | "wave" => (3, 4, 140, 280),
            "jumping" | "bounce" => (4, 5, 140, 280),
            "failed" | "sad" => (5, 8, 140, 240),
            "waiting" => (6, 6, 150, 260),
            "running" => (7, 6, 120, 220),
            "review" => (8, 6, 150, 280),
            _ => bail!("unknown animation {name}"),
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
    };
    frames.retain(|&(frame, _)| frame < count);
    if frames.is_empty() {
        return timeline(manifest, grid, "idle");
    }
    Ok(Timeline {
        plays: if name == "idle" { 0 } else { 3 },
        frames,
        fallback: "idle".into(),
    })
}

fn encode(sheet: &RgbaImage, grid: Grid, animation: &Timeline, budget: usize) -> Result<Encoded> {
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
    Ok(Encoded {
        data: encoded.into(),
        after: (animation.plays > 0).then(|| {
            let delay = Duration::from_millis(elapsed * u64::from(animation.plays));
            (delay, animation.fallback.clone())
        }),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn ambient_apng_contains_each_stock_frame_once_with_three_plays() -> Result<()> {
        let grid = Grid {
            width: 8,
            height: 8,
            ..Grid::default()
        };
        let mut sheet = RgbaImage::new(grid.width * grid.columns, grid.height * grid.rows);
        for frame in 0..grid.columns * grid.rows {
            let red = u8::try_from(frame)?;
            let tile =
                RgbaImage::from_pixel(grid.width, grid.height, image::Rgba([red, 0, 255, 255]));
            imageops::replace(
                &mut sheet,
                &tile,
                i64::from(frame % grid.columns * grid.width),
                i64::from(frame / grid.columns * grid.height),
            );
        }
        let timeline = timeline(&Manifest::default(), grid, "running")?;
        let clip = encode(&sheet, grid, &timeline, MAX_ENCODED)?;
        assert_eq!(
            clip.after,
            Some((Duration::from_millis(2460), "idle".into()))
        );
        let mut reader = png::Decoder::new(Cursor::new(&*clip.data)).read_info()?;
        let control = reader
            .info()
            .animation_control
            .context("expected APNG control")?;
        assert_eq!((control.num_frames, control.num_plays), (6, 3));
        let mut rgba = vec![0; reader.output_buffer_size().context("expected buffer")?];
        for (frame, expected_delay) in timeline.frames {
            reader.next_frame(&mut rgba)?;
            let control = reader
                .info()
                .frame_control
                .context("expected APNG frame control")?;
            assert_eq!(
                (control.delay_num, control.delay_den),
                (expected_delay, 1000)
            );
            assert_eq!(rgba[0], u8::try_from(frame)?);
        }
        Ok(())
    }

    #[test]
    fn ambient_manifest_uses_only_the_referenced_custom_fallback_and_reuses_its_bytes() -> Result<()>
    {
        let manifest: Manifest = serde_json::from_str(
            r#"{"animations":{
                "running":{"frames":[56,57],"fps":8,"loop":false,"fallback":"waiting"},
                "waiting":{"frames":[48,49],"fps":8,"loop":true},
                "unused":{"frames":[55],"fps":8}
            }}"#,
        )?;
        let grid = Grid {
            width: 1,
            height: 1,
            ..Grid::default()
        };
        let mut sheet = RgbaImage::new(grid.columns, grid.rows);
        for (index, pixel) in sheet.pixels_mut().enumerate() {
            *pixel = image::Rgba([u8::try_from(index)?, 4, 9, 255]);
        }
        let mut compiler = Compiler {
            sheet: &sheet,
            manifest: &manifest,
            grid,
            animated: true,
            remaining: MAX_ENCODED,
            sources: BTreeMap::new(),
        };
        let running = compiler.clip("running", true)?;
        let transition = running
            .after
            .as_ref()
            .context("expected a finite transition")?;
        assert_eq!(transition.delay, Duration::from_millis(250));
        assert!(transition.next.after.is_none());

        for (clip, expected_plays, reds) in [
            (&running, 1, [56, 57]),
            (transition.next.as_ref(), 0, [48, 49]),
        ] {
            let mut reader = png::Decoder::new(Cursor::new(&*clip.data)).read_info()?;
            let control = reader
                .info()
                .animation_control
                .context("expected APNG control")?;
            assert_eq!((control.num_frames, control.num_plays), (2, expected_plays));
            let mut rgba = [0; 4];
            for red in reds {
                reader.next_frame(&mut rgba)?;
                assert_eq!(rgba, [red, 4, 9, 255]);
            }
        }

        let waiting = compiler.clip("waiting", true)?;
        assert!(Arc::ptr_eq(&waiting.data, &transition.next.data));
        assert_eq!(compiler.sources.len(), 2);
        assert!(!compiler.sources.contains_key("unused"));
        assert_eq!(
            MAX_ENCODED - compiler.remaining,
            running.data.len() + waiting.data.len()
        );
        Ok(())
    }

    #[test]
    fn ambient_manifest_custom_loop_and_reduced_motion_have_distinct_deadlines() -> Result<()> {
        let manifest: Manifest = serde_json::from_str(
            r#"{"animations":{"idle":{"frames":[3,1],"fps":8,"loop":true}}}"#,
        )?;
        let grid = Grid {
            width: 1,
            height: 1,
            ..Grid::default()
        };
        let sheet = RgbaImage::new(grid.width * grid.columns, grid.height * grid.rows);
        let mut timeline = timeline(&manifest, grid, "idle")?;
        assert_eq!(timeline.frames, [(3, 125), (1, 125)]);
        let animated = encode(&sheet, grid, &timeline, MAX_ENCODED)?;
        let reader = png::Decoder::new(Cursor::new(&*animated.data)).read_info()?;
        let control = reader
            .info()
            .animation_control
            .context("expected APNG control")?;
        assert_eq!((control.num_frames, control.num_plays), (2, 0));
        assert_eq!(animated.after, None);

        timeline.frames.truncate(1);
        let still = encode(&sheet, grid, &timeline, MAX_ENCODED)?;
        let reader = png::Decoder::new(Cursor::new(&*still.data)).read_info()?;
        assert!(reader.info().animation_control.is_none());
        assert_eq!(still.after, None);
        assert!(encode(&sheet, grid, &timeline, 1).is_err());
        Ok(())
    }
}
