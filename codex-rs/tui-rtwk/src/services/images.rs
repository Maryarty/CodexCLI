//! Images pasted into the composer as paths: a terminal drops a dragged file
//! on the prompt as text (its path, quoted or shell-escaped, or a `file://`
//! URL). A path that names an image file is attached as `[Image #N]` instead
//! of being typed (oracle `clipboard_paste.rs`: `normalize_pasted_path`,
//! `chat_composer.rs`: `handle_paste_image_path`). The file is looked at
//! here, in a service, never in a view.

use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codex_tui_core::app::effects::{ImagePreview, ImageSize, ImageSnapshot};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use url::Url;

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DECODE_BYTES: u64 = 32 * 1024 * 1024;

/// The host's worker for local reads. Keep simultaneous optional raster decodes bounded.
pub async fn read_local_snapshot(path: String, preview: Option<ImageSize>) -> ImageSnapshot {
    static READERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let Ok(permit) = READERS.acquire().await else {
        return ImageSnapshot::default();
    };
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        local_snapshot(Path::new(&path), preview)
    })
    .await
    .unwrap_or_default()
}

/// Read one snapshot of a path whose workspace the host has already verified as local.
/// No path-only cache is shared between requests, so rereading the same path sees a rewrite.
pub fn local_snapshot(path: &Path, preview: Option<ImageSize>) -> ImageSnapshot {
    let Some((target, url)) = local_image_target(path) else {
        return ImageSnapshot::default();
    };
    let link = (ImageFormat::from_path(path).is_ok() && ImageFormat::from_path(&target).is_ok())
        .then(|| url.to_string());
    let preview = preview.and_then(|size| read_preview(&target, size));
    ImageSnapshot { link, preview }
}

fn local_image_target(path: &Path) -> Option<(PathBuf, Url)> {
    let supplied = Url::from_file_path(path).ok()?;
    if supplied.host().is_some() {
        return None;
    }
    // The ordinary Windows canonicalize may follow a junction to a network share. Take the
    // same conservative path ancestry gate as the oracle before following any of them.
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        for ancestor in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
            if ancestor.symlink_metadata().ok()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT
                != 0
            {
                return None;
            }
        }
    }
    let target = dunce::canonicalize(path).ok()?;
    let url = Url::from_file_path(&target).ok()?;
    (url.host().is_none() && target.is_file()).then_some((target, url))
}

fn read_preview(path: &Path, bound: ImageSize) -> Option<ImagePreview> {
    let width = u32::from(bound.width.min(384));
    let height = u32::from(bound.height.min(160));
    if width == 0 || height == 0 {
        return None;
    }
    let mut open = OpenOptions::new();
    open.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A file can be replaced after canonicalization, including by a symlink or FIFO.
        open.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        open.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = open.open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    if file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes).ok()? as u64 > MAX_FILE_BYTES {
        return None;
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().ok()?;
    limits.reserve(decoder.total_bytes()).ok()?;
    decoder.set_limits(limits).ok()?;
    let orientation = decoder.orientation().ok();
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    let thumbnail = image.thumbnail(width.min(image.width()), height.min(image.height()));
    let size = ImageSize {
        width: u16::try_from(thumbnail.width()).ok()?,
        height: u16::try_from(thumbnail.height()).ok()?,
    };
    let mut png = Cursor::new(Vec::new());
    thumbnail.write_to(&mut png, ImageFormat::Png).ok()?;
    Some(ImagePreview {
        png: Arc::from(png.into_inner()),
        size,
    })
}

/// The image file `pasted` names, `None` when it is no path, no regular file
/// (opening a device or FIFO could block the input loop) or no image.
pub fn pasted_image(pasted: &str) -> Option<PathBuf> {
    let path = normalize_pasted_path(pasted)?;
    (path.is_file() && is_image(&path)).then_some(path)
}

/// The path a pasted text stands for: a quoted one, a `file://` URL, a
/// Windows or UNC path, or one shell-escaped word.
pub fn normalize_pasted_path(pasted: &str) -> Option<PathBuf> {
    let pasted = pasted.trim();
    let unquoted = pasted
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| pasted.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(pasted);
    if let Some(rest) = unquoted.strip_prefix("file://") {
        // `file://host/path`: only the local host's
        let path = rest.strip_prefix("localhost").unwrap_or(rest);
        return path
            .starts_with('/')
            .then(|| PathBuf::from(percent_decode(path)));
    }
    if is_windows_path(unquoted) {
        return Some(PathBuf::from(unquoted));
    }
    let mut words = shlex::Shlex::new(pasted);
    let word = words.next()?;
    if words.next().is_some() {
        return None;
    }
    Some(PathBuf::from(word))
}

fn is_windows_path(text: &str) -> bool {
    let mut chars = text.chars();
    let drive = matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(d), Some(':'), Some('\\' | '/')) if d.is_ascii_alphabetic()
    );
    drive || text.starts_with("\\\\")
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[at] == b'%'
            && at + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[at + 1]), hex(bytes[at + 2]))
        {
            out.push(u8::try_from(high * 16 + low).unwrap_or(b'%'));
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether the file starts as an image of a kind the model takes (PNG, JPEG,
/// GIF, WebP): what `image_dimensions` reading its header decides in the
/// oracle.
fn is_image(path: &Path) -> bool {
    let mut head = [0u8; 12];
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let Ok(read) = file.read(&mut head) else {
        return false;
    };
    let head = &head[..read];
    head.starts_with(b"\x89PNG\r\n\x1a\n")
        || head.starts_with(&[0xff, 0xd8, 0xff])
        || head.starts_with(b"GIF87a")
        || head.starts_with(b"GIF89a")
        || (head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const PREVIEW: ImageSize = ImageSize {
        width: 384,
        height: 96,
    };

    #[test]
    fn same_path_rewrite_gives_new_bounded_pixels_and_the_same_safe_link() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a screenshot.png");
        let write = |color| {
            image::RgbaImage::from_pixel(480, 160, image::Rgba(color))
                .save(&path)
                .unwrap();
        };
        write([255, 0, 0, 255]);
        let first = local_snapshot(&path, Some(PREVIEW));
        write([0, 0, 255, 255]);
        let second = local_snapshot(&path, Some(PREVIEW));

        assert_eq!(first.link, second.link);
        assert!(
            first
                .link
                .as_deref()
                .unwrap()
                .ends_with("/a%20screenshot.png")
        );
        let first = first.preview.unwrap();
        let second = second.preview.unwrap();
        assert_eq!(
            first.size,
            ImageSize {
                width: 288,
                height: 96
            }
        );
        assert_eq!(first.size, second.size);
        assert_ne!(first, second);
        assert_ne!(first.png, second.png);
    }

    #[test]
    fn an_image_larger_than_the_decode_budget_only_keeps_its_link() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.png");
        image::RgbImage::new(4000, 3000).save(&path).unwrap();
        let snapshot = local_snapshot(&path, Some(PREVIEW));
        assert!(snapshot.link.is_some());
        assert!(snapshot.preview.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_to_a_nonimage_never_becomes_a_file_link() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("not_image.txt");
        std::fs::write(&target, "text").unwrap();
        let path = dir.path().join("picture.png");
        symlink(target, &path).unwrap();
        assert_eq!(
            local_snapshot(&path, Some(PREVIEW)),
            ImageSnapshot::default()
        );
    }
}
