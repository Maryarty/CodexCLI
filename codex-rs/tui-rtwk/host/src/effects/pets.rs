//! The local pet catalog and first idle frame.
//! Catalog data and the on-disk format follow oracle `pets/{catalog,asset_pack,model,picker}.rs`.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use codex_core::config::Config;
use codex_core::config::edit::{ConfigEditsBuilder, tui_pet_edit};
use codex_http_client::{ClientRouteClass, RouteAwareClientPool};
use codex_terminal_detection::{Multiplexer, TerminalInfo, TerminalName, terminal_info};
use codex_tui_core::app::effects::{ImagePreview, ImageSize};
use codex_tui_core::screens::pets::{AssetAsk, AssetDone, DISABLED, Pet, Picker, Unavailable};
use codex_tui_rtwk::services::agents::workspace::blocking;
use serde::Deserialize;

mod ambient;

const WIDTH: u32 = 1536;
const HEIGHT: u32 = 1872;
const MAX_DOWNLOAD: usize = 4 * 1024 * 1024;
const CDN: &str = "https://persistent.oaistatic.com/codex/pets/v1";

struct Builtin {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    file: &'static str,
}

const BUILTIN: &[Builtin] = &[
    Builtin {
        id: "codex",
        name: "Codex",
        description: "The original Codex companion",
        file: "codex-spritesheet-v4.webp",
    },
    Builtin {
        id: "dewey",
        name: "Dewey",
        description: "A tidy duck for calm workspace days",
        file: "dewey-spritesheet-v4.webp",
    },
    Builtin {
        id: "fireball",
        name: "Fireball",
        description: "Hot path energy for fast iteration",
        file: "fireball-spritesheet-v4.webp",
    },
    Builtin {
        id: "rocky",
        name: "Rocky",
        description: "A steady rock when the diff gets large",
        file: "rocky-spritesheet-v4.webp",
    },
    Builtin {
        id: "seedy",
        name: "Seedy",
        description: "Small green shoots for new ideas",
        file: "seedy-spritesheet-v4.webp",
    },
    Builtin {
        id: "stacky",
        name: "Stacky",
        description: "A balanced stack for deep work",
        file: "stacky-spritesheet-v4.webp",
    },
    Builtin {
        id: "bsod",
        name: "BSOD",
        description: "A tiny blue-screen gremlin",
        file: "bsod-spritesheet-v4.webp",
    },
    Builtin {
        id: "null-signal",
        name: "Null Signal",
        description: "Quiet signal from the void",
        file: "null-signal-spritesheet-v4.webp",
    },
];

pub(super) async fn run(ask: AssetAsk, config: Option<Arc<Config>>) -> AssetDone {
    match ask {
        AssetAsk::Catalog { graphics } => {
            let support = support(&terminal_info(), graphics);
            AssetDone::Catalog(match support {
                Ok(()) => Ok(blocking(move || catalog(config.as_deref()))
                    .await
                    .unwrap_or_else(|| catalog(None))),
                Err(reason) => Err(reason),
            })
        }
        AssetAsk::Preview { selector } => AssetDone::Preview(
            preview(config, selector)
                .await
                .map_err(|err| err.to_string()),
        ),
        AssetAsk::Ambient { selector } => AssetDone::Ambient(
            ambient::load(config, selector, &terminal_info())
                .await
                .ok()
                .flatten()
                .map(Arc::new),
        ),
        AssetAsk::Save { selector } => AssetDone::Saved(
            save(config.as_deref(), &selector)
                .await
                .map_err(|err| err.to_string()),
        ),
    }
}

fn support(info: &TerminalInfo, graphics: bool) -> Result<(), Unavailable> {
    // The toolkit does not establish an oracle-compatible pane-local graphics namespace (G10).
    match info.multiplexer {
        Some(Multiplexer::Tmux { .. }) => return Err(Unavailable::Tmux),
        Some(Multiplexer::Zellij { .. }) => return Err(Unavailable::Zellij),
        None => {}
    }
    let iterm = info.name == TerminalName::Iterm2
        || info
            .term_program
            .as_deref()
            .is_some_and(|name| name.to_ascii_lowercase().contains("iterm"));
    if iterm && !version_at_least_3_6(info.version.as_deref()) {
        return Err(Unavailable::OldIterm);
    }
    if graphics {
        Ok(())
    } else {
        Err(Unavailable::Terminal)
    }
}

fn version_at_least_3_6(value: Option<&str>) -> bool {
    let Some(value) = value else { return false };
    let mut parts = value.split('.');
    let major = parts.next().and_then(|part| part.parse::<u64>().ok());
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok();
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok();
    matches!((major, minor, patch), (Some(major), Some(minor), Some(_)) if (major, minor) >= (3, 6))
        && parts.next().is_none()
}

fn catalog(config: Option<&Config>) -> Picker {
    let mut pets: Vec<Pet> = BUILTIN
        .iter()
        .map(|pet| Pet {
            selector: pet.id.to_owned(),
            name: pet.name.to_owned(),
            description: Some(pet.description.to_owned()),
            legacy_selector: None,
        })
        .collect();
    if let Some(config) = config {
        pets.extend(custom_catalog(config.codex_home.as_path()));
    }
    Picker::build(config.and_then(|config| config.tui_pet.as_deref()), pets)
}

const CUSTOM_DIRS: [(&str, &str); 2] = [("pets", "pet.json"), ("avatars", "avatar.json")];

fn custom_catalog(home: &Path) -> impl Iterator<Item = Pet> {
    let mut pets = BTreeMap::new();
    // The modern pet directory takes priority over its legacy avatar counterpart.
    for (directory, filename) in CUSTOM_DIRS {
        let Ok(children) = fs::read_dir(home.join(directory)) else {
            continue;
        };
        for child in children.flatten() {
            let Ok(id) = child.file_name().into_string() else {
                continue;
            };
            if id == DISABLED || id.starts_with("custom:") || pets.contains_key(&id) {
                continue;
            }
            if !child.path().join(filename).is_file() {
                continue;
            }
            let Ok((manifest, _)) = installed_asset(home, &id) else {
                continue;
            };
            let trim = |value: Option<String>| {
                value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
            };
            pets.insert(
                id.clone(),
                Pet {
                    selector: format!("custom:{id}"),
                    name: trim(manifest.display_name)
                        .or_else(|| trim(manifest.id))
                        .unwrap_or_else(|| id.clone()),
                    description: trim(manifest.description),
                    legacy_selector: Some(id),
                },
            );
        }
    }
    pets.into_values()
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    id: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    spritesheet_path: Option<String>,
    frame: Option<Grid>,
    #[serde(default)]
    animations: BTreeMap<String, Animation>,
}

#[derive(Deserialize)]
struct Animation {
    #[serde(default)]
    frames: Vec<u32>,
    fps: Option<f64>,
    #[serde(rename = "loop")]
    loop_animation: Option<bool>,
    #[serde(default)]
    fallback: String,
}

#[derive(Clone, Copy, Deserialize)]
struct Grid {
    width: u32,
    height: u32,
    columns: u32,
    rows: u32,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            width: 192,
            height: 208,
            columns: 8,
            rows: 9,
        }
    }
}

fn custom_asset(directory: &Path, filename: &str) -> Result<(Manifest, PathBuf)> {
    let directory = directory.canonicalize()?;
    let manifest: Manifest = serde_json::from_slice(&fs::read(directory.join(filename))?)?;
    let relative = manifest
        .spritesheet_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .unwrap_or("spritesheet.webp");
    let relative = Path::new(relative);
    if !relative
        .components()
        .all(|part| matches!(part, Component::CurDir | Component::Normal(_)))
    {
        bail!("pet spritesheet must be inside its manifest directory");
    }
    let sheet = directory.join(relative).canonicalize()?;
    if !sheet.starts_with(&directory) {
        bail!("pet spritesheet must be inside its manifest directory");
    }
    validate_sheet(&sheet)?;
    validate_grid(manifest.frame.unwrap_or_default())?;
    Ok((manifest, sheet))
}

fn installed_asset(home: &Path, selector: &str) -> Result<(Manifest, PathBuf)> {
    let id = selector.strip_prefix("custom:").unwrap_or(selector);
    let mut components = Path::new(id).components();
    if matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none() {
        for (directory, filename) in CUSTOM_DIRS {
            let path = home.join(directory).join(id);
            if path.join(filename).is_file() {
                return custom_asset(&path, filename);
            }
        }
    }
    bail!("unknown pet {selector}");
}

fn local_asset(home: &Path, selector: &str) -> Result<(Manifest, PathBuf)> {
    let path = Path::new(selector);
    if !path.is_absolute() && !selector.contains(['/', '\\']) && !matches!(selector, "." | "..") {
        return installed_asset(home, selector);
    }
    let expanded;
    let path = if let Some(suffix) = selector.strip_prefix("~/") {
        let user_home = std::env::var_os("HOME").context("HOME is not set")?;
        expanded = PathBuf::from(user_home).join(suffix);
        expanded.as_path()
    } else {
        path
    };
    let metadata = fs::metadata(path).with_context(|| format!("pet path {}", path.display()))?;
    let directory = if metadata.is_dir() {
        path
    } else {
        path.parent()
            .context("pet path has no containing directory")?
    };
    // Explicit selectors name a pack. A file selector chooses its containing pack,
    // with the same manifest precedence as the oracle.
    for (_, filename) in CUSTOM_DIRS {
        if directory.join(filename).is_file() {
            return custom_asset(directory, filename);
        }
    }
    bail!("missing pet.json or avatar.json in {}", directory.display());
}

fn validate_grid(grid: Grid) -> Result<()> {
    if grid.width == 0
        || grid.height == 0
        || grid.columns == 0
        || grid.rows == 0
        || grid
            .width
            .checked_mul(grid.columns)
            .is_none_or(|span| span > WIDTH)
        || grid
            .height
            .checked_mul(grid.rows)
            .is_none_or(|span| span > HEIGHT)
    {
        bail!("pet sprite grid does not fit its spritesheet");
    }
    Ok(())
}

fn validate_sheet(path: &Path) -> Result<()> {
    if image::image_dimensions(path)? != (WIDTH, HEIGHT) {
        bail!("pet spritesheet must be {WIDTH}×{HEIGHT} pixels");
    }
    Ok(())
}

async fn preview(config: Option<Arc<Config>>, selector: String) -> Result<ImagePreview> {
    let config = config.context("pet configuration is unavailable")?;
    if let Some(pet) = BUILTIN.iter().find(|pet| pet.id == selector) {
        let sheet = ensure_builtin(&config, pet).await?;
        return blocking(move || first_idle(&sheet, &Manifest::default()))
            .await
            .context("read built-in pet frame")?;
    }
    blocking(move || {
        let (manifest, sheet) = local_asset(config.codex_home.as_path(), &selector)?;
        first_idle(&sheet, &manifest)
    })
    .await
    .context("read custom pet frame")?
}

fn first_idle(sheet: &Path, manifest: &Manifest) -> Result<ImagePreview> {
    validate_sheet(sheet)?;
    let grid = manifest.frame.unwrap_or_default();
    validate_grid(grid)?;
    let index = manifest
        .animations
        .get("idle")
        .and_then(|idle| {
            idle.frames
                .iter()
                .copied()
                .find(|index| *index < grid.columns * grid.rows)
        })
        .unwrap_or(0);
    let image = image::open(sheet)?.crop_imm(
        (index % grid.columns) * grid.width,
        (index / grid.columns) * grid.height,
        grid.width,
        grid.height,
    );
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png)?;
    Ok(ImagePreview {
        png: png.into_inner().into(),
        size: ImageSize {
            width: u16::try_from(grid.width)?,
            height: u16::try_from(grid.height)?,
        },
    })
}

async fn ensure_builtin(config: &Config, pet: &Builtin) -> Result<PathBuf> {
    let path = config
        .codex_home
        .as_path()
        .join("cache/tui-pets/v1/assets")
        .join(pet.file);
    let read_path = path.clone();
    if blocking(move || validate_sheet(&read_path).is_ok())
        .await
        .unwrap_or(false)
    {
        return Ok(path);
    }
    let pool = RouteAwareClientPool::new(config.http_client_factory(), ClientRouteClass::Other);
    let mut response = pool
        .get(format!("{CDN}/{}", pet.file))
        .timeout(Duration::from_secs(60))
        .send()
        .await?
        .error_for_status()?;
    if response.url().scheme() != "https"
        || response
            .content_length()
            .is_some_and(|count| count > MAX_DOWNLOAD as u64)
    {
        bail!("invalid pet sprite download");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > MAX_DOWNLOAD {
            bail!("pet sprite download exceeds {MAX_DOWNLOAD} bytes");
        }
        bytes.extend_from_slice(&chunk);
    }
    blocking(move || {
        let directory = path.parent().context("pet cache has no parent directory")?;
        fs::create_dir_all(directory)?;
        let mut temporary = tempfile::Builder::new()
            .suffix(".webp")
            .tempfile_in(directory)?;
        temporary.write_all(&bytes)?;
        validate_sheet(temporary.path())?;
        if let Err(error) = temporary.persist(&path) {
            validate_sheet(&path).with_context(|| format!("install pet sprite: {error}"))?;
        }
        Ok(path)
    })
    .await
    .context("install pet sprite")?
}

async fn save(config: Option<&Config>, selector: &str) -> Result<()> {
    let config = config.context("pet configuration is unavailable")?;
    ConfigEditsBuilder::new(config.codex_home.as_path())
        .with_edits([tui_pet_edit(selector)])
        .apply()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView as _;

    #[test]
    fn host_checks_the_terminal_before_enabling_pets() {
        let mut info = TerminalInfo {
            name: TerminalName::Iterm2,
            term_program: None,
            version: Some("3.5.8".into()),
            term: None,
            multiplexer: None,
            backend: None,
        };
        assert_eq!(support(&info, true), Err(Unavailable::OldIterm));
        info.version = Some("3.6".into());
        assert_eq!(support(&info, false), Err(Unavailable::Terminal));
        assert_eq!(support(&info, true), Ok(()));
        info.multiplexer = Some(Multiplexer::Tmux { version: None });
        assert_eq!(support(&info, true), Err(Unavailable::Tmux));
    }

    #[test]
    fn catalog_resolves_legacy_ids_and_reads_the_first_configured_idle_frame() -> Result<()> {
        let home = tempfile::tempdir()?;
        let directory = home.path().join("avatars/owl");
        fs::create_dir_all(&directory)?;
        fs::File::create(directory.join("avatar.json"))?.write_all(
            br#"{
            "displayName": "Owl",
            "spritesheetPath": "spritesheet.png",
            "animations": {"idle": {"frames": [1, 0]}}
        }"#,
        )?;
        let mut pixels = image::RgbaImage::new(WIDTH, HEIGHT);
        let marker = image::Rgba([80, 160, 120, 255]);
        pixels.put_pixel(192, 0, marker);
        pixels.save(directory.join("spritesheet.png"))?;
        let listed: Vec<_> = custom_catalog(home.path()).collect();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].selector, "custom:owl");
        assert_eq!(listed[0].legacy_selector.as_deref(), Some("owl"));
        for selector in ["owl", "custom:owl"] {
            let (manifest, path) = local_asset(home.path(), selector)?;
            let frame = first_idle(&path, &manifest)?;
            assert_eq!(
                frame.size,
                ImageSize {
                    width: 192,
                    height: 208
                }
            );
            assert_eq!(image::load_from_memory(&frame.png)?.get_pixel(0, 0), marker);
        }
        Ok(())
    }

    #[test]
    fn pet_path_selector_uses_the_containing_pack_for_absolute_and_relative_paths() -> Result<()> {
        let cwd = std::env::current_dir()?;
        let workspace = tempfile::tempdir_in(&cwd)?;
        let home = tempfile::tempdir()?;
        let pack = workspace.path().join("pet with spaces");
        fs::create_dir(&pack)?;
        let avatar = pack.join("avatar.json");
        fs::File::create(&avatar)?
            .write_all(br#"{"displayName":"Legacy","spritesheetPath":"sheet.png"}"#)?;
        image::RgbaImage::new(WIDTH, HEIGHT).save(pack.join("sheet.png"))?;
        let (manifest, _) = local_asset(home.path(), avatar.to_str().context("avatar path")?)?;
        assert_eq!(manifest.display_name.as_deref(), Some("Legacy"));

        let pet = pack.join("pet.json");
        fs::File::create(&pet)?
            .write_all(br#"{"displayName":"Modern","spritesheetPath":"sheet.png"}"#)?;
        let relative = pack.strip_prefix(&cwd)?;
        let relative_file = relative.join("avatar.json");
        let dotted = Path::new(".").join(&relative_file);
        let parent_relative = Path::new("..")
            .join(cwd.file_name().context("package directory")?)
            .join(&relative_file);
        let absolute = pack.canonicalize()?;
        let selectors: [&Path; 6] = [
            &absolute,
            &pet,
            relative,
            &relative_file,
            &dotted,
            &parent_relative,
        ];
        for selector in selectors {
            let selector = selector.to_str().context("UTF-8 pet selector")?;
            let (manifest, sheet) = local_asset(home.path(), selector)?;
            assert_eq!(manifest.display_name.as_deref(), Some("Modern"));
            assert_eq!(sheet, absolute.join("sheet.png"));
        }
        assert_eq!(custom_catalog(home.path()).count(), 0);
        assert!(local_asset(home.path(), "pet with spaces").is_err());
        assert!(local_asset(home.path(), "custom:pet with spaces").is_err());
        assert!(local_asset(home.path(), "./does-not-exist/pet.json").is_err());
        Ok(())
    }

    #[test]
    fn pet_path_selector_reuses_pack_validation_for_an_invalid_sheet() -> Result<()> {
        let home = tempfile::tempdir()?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("pet.json");
        let selector = path.to_str().context("UTF-8 pet selector")?;
        for (manifest, message) in [
            (
                r#"{"spritesheetPath":"../escape.png"}"#,
                "inside its manifest directory",
            ),
            (r#"{"spritesheetPath":"tiny.png"}"#, "1536×1872"),
        ] {
            image::RgbaImage::new(1, 1).save(dir.path().join("tiny.png"))?;
            fs::File::create(&path)?.write_all(manifest.as_bytes())?;
            let result = local_asset(home.path(), selector);
            let Err(error) = result else {
                panic!("expected invalid pet pack")
            };
            assert!(error.to_string().contains(message), "{error}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn configured_pet_path_opens_preview_and_native_ambient_from_the_same_pack() -> Result<()>
    {
        use codex_core::config::ConfigBuilder;
        use codex_tui_core::screens::pets::Motion;

        let home = tempfile::tempdir()?;
        let pack = tempfile::tempdir()?;
        let manifest = pack.path().join("pet.json");
        fs::File::create(&manifest)?.write_all(
            br#"{"displayName":"Outside","spritesheetPath":"sheet.png","animations":{"idle":{"frames":[1,0],"fps":8}}}"#,
        )?;
        let marker = image::Rgba([80, 160, 120, 255]);
        let mut sheet = image::RgbaImage::new(WIDTH, HEIGHT);
        sheet.put_pixel(192, 0, marker);
        sheet.save(pack.path().join("sheet.png"))?;
        let selector = manifest.to_str().context("UTF-8 pet selector")?;
        let config = || {
            ConfigBuilder::default()
                .codex_home(home.path().to_owned())
                .fallback_cwd(Some(home.path().to_owned()))
                .build()
        };
        save(Some(&config().await?), selector).await?;
        let config = Arc::new(config().await?);
        let picker = catalog(Some(&config));
        assert_eq!(picker.current.as_deref(), Some(selector));
        assert!(picker.pets.iter().all(|pet| pet.selector != selector));

        let preview = preview(Some(config.clone()), selector.to_owned()).await?;
        assert_eq!(
            image::load_from_memory(&preview.png)?.get_pixel(0, 0),
            marker
        );
        let kitty = TerminalInfo {
            name: TerminalName::Kitty,
            term_program: None,
            version: None,
            term: None,
            multiplexer: None,
            backend: None,
        };
        let ambient = ambient::load(Some(config), None, &kitty)
            .await?
            .context("configured path at startup")?;
        let data = &ambient.clip(Motion::Idle).data;
        let reader = png::Decoder::new(Cursor::new(data.as_ref())).read_info()?;
        let control = reader.info().animation_control.context("native APNG")?;
        assert_eq!((control.num_frames, control.num_plays), (2, 0));
        assert_eq!(image::load_from_memory(data)?.get_pixel(0, 0), marker);
        assert!(!home.path().join("cache/tui-pets/v1/assets").exists());
        Ok(())
    }
}
