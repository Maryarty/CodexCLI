//! Images pasted into the composer as paths: a terminal drops a dragged file
//! on the prompt as text (its path, quoted or shell-escaped, or a `file://`
//! URL). A path that names an image file is attached as `[Image #N]` instead
//! of being typed (oracle `clipboard_paste.rs`: `normalize_pasted_path`,
//! `chat_composer.rs`: `handle_paste_image_path`). The file is looked at
//! here, in a service, never in a view.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

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
