//! Clipboard images (oracle `clipboard_paste.rs`): the first image on the
//! system clipboard, encoded as PNG into a temporary file the composer
//! attaches as `[Image #N]`. A copied file wins over its clipboard preview;
//! under WSL, where the Linux clipboard cannot see the Windows one, PowerShell
//! saves the image and the mapped path is attached directly.
//!
//! In Flutter this is `Clipboard.getData` for an image format: a future the
//! caller awaits ([`paste_image`]); the blocking read runs off the async
//! workers.

use std::path::Path;
use std::path::PathBuf;

use tempfile::Builder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteImageError {
    ClipboardUnavailable(String),
    NoImage(String),
    EncodeFailed(String),
    IoError(String),
}

impl std::fmt::Display for PasteImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasteImageError::ClipboardUnavailable(msg) => write!(f, "clipboard unavailable: {msg}"),
            PasteImageError::NoImage(msg) => write!(f, "no image on clipboard: {msg}"),
            PasteImageError::EncodeFailed(msg) => write!(f, "could not encode image: {msg}"),
            PasteImageError::IoError(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for PasteImageError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodedImageFormat {
    Png,
    Jpeg,
    Other,
}

impl EncodedImageFormat {
    pub fn label(self) -> &'static str {
        match self {
            EncodedImageFormat::Png => "PNG",
            EncodedImageFormat::Jpeg => "JPEG",
            EncodedImageFormat::Other => "IMG",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastedImageInfo {
    pub width: u32,
    pub height: u32,
    pub encoded_format: EncodedImageFormat, // Always PNG for now.
}

/// Infer an image format for the provided path based on its extension.
pub fn pasted_image_format(path: &Path) -> EncodedImageFormat {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => EncodedImageFormat::Png,
        Some("jpg") | Some("jpeg") => EncodedImageFormat::Jpeg,
        _ => EncodedImageFormat::Other,
    }
}

/// [`paste_image_to_temp_png`] off the async workers: the file the image was
/// written to (or, under WSL, the Windows file's path) and what it holds.
pub async fn paste_image() -> Result<(PathBuf, PastedImageInfo), PasteImageError> {
    tokio::task::spawn_blocking(paste_image_to_temp_png)
        .await
        .map_err(|error| PasteImageError::IoError(format!("the paste did not finish: {error}")))?
}

/// Read the first clipboard image and encode it as a PNG attachment.
#[cfg(not(target_os = "android"))]
pub fn paste_image_as_png() -> Result<(Vec<u8>, PastedImageInfo), PasteImageError> {
    let _span = tracing::debug_span!("paste_image_as_png").entered();
    tracing::debug!("attempting clipboard image read");
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| PasteImageError::ClipboardUnavailable(e.to_string()))?;
    let image = read_clipboard_image(&mut clipboard)?;

    let span = tracing::debug_span!("encode_image", byte_length = tracing::field::Empty).entered();
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| PasteImageError::EncodeFailed(e.to_string()))?;
    span.record("byte_length", png.get_ref().len());
    let info = PastedImageInfo {
        width: image.width(),
        height: image.height(),
        encoded_format: EncodedImageFormat::Png,
    };
    Ok((png.into_inner(), info))
}

#[cfg(not(target_os = "android"))]
fn read_clipboard_image(
    clipboard: &mut arboard::Clipboard,
) -> Result<image::DynamicImage, PasteImageError> {
    // A copied file takes precedence over its clipboard preview.
    let files = clipboard.get().file_list().unwrap_or_default();
    if let Some(image) = files.into_iter().find_map(|path| {
        let reader = image::ImageReader::open(path).ok()?;
        super::decode::decode_oriented(reader, image::Limits::default()).ok()
    }) {
        tracing::debug!(
            "clipboard image opened from file: {}x{}",
            image.width(),
            image.height()
        );
        return Ok(image);
    }

    #[cfg(target_os = "macos")]
    if let Some(image) = super::macos::clipboard_image() {
        return Ok(image);
    }

    let _span = tracing::debug_span!("get_image").entered();
    let bitmap = clipboard.get_image().map_err(|e| {
        if cfg!(windows) && !matches!(e, arboard::Error::ContentNotAvailable) {
            PasteImageError::ClipboardUnavailable(e.to_string())
        } else {
            PasteImageError::NoImage(e.to_string())
        }
    })?;
    let width = bitmap.width as u32;
    let height = bitmap.height as u32;
    tracing::debug!("clipboard image opened from image: {width}x{height}");
    let rgba = image::RgbaImage::from_raw(width, height, bitmap.bytes.into_owned())
        .ok_or_else(|| PasteImageError::EncodeFailed("invalid RGBA buffer".into()))?;
    Ok(image::DynamicImage::ImageRgba8(rgba))
}

/// Android/Termux does not support arboard; return a clear error.
#[cfg(target_os = "android")]
pub fn paste_image_as_png() -> Result<(Vec<u8>, PastedImageInfo), PasteImageError> {
    Err(PasteImageError::ClipboardUnavailable(
        "clipboard image paste is unsupported on Android".into(),
    ))
}

/// Convenience: write to a temp file and return its path + info.
#[cfg(not(target_os = "android"))]
pub fn paste_image_to_temp_png() -> Result<(PathBuf, PastedImageInfo), PasteImageError> {
    // First attempt: read image from system clipboard via arboard (native paths or image data).
    match paste_image_as_png() {
        Ok((png, info)) => {
            // Create a unique temporary file with a .png suffix to avoid collisions.
            let tmp = Builder::new()
                .prefix("codex-clipboard-")
                .suffix(".png")
                .tempfile()
                .map_err(|e| PasteImageError::IoError(e.to_string()))?;
            std::fs::write(tmp.path(), &png)
                .map_err(|e| PasteImageError::IoError(e.to_string()))?;
            // Persist the file (so it remains after the handle is dropped) and return its PathBuf.
            let (_file, path) = tmp
                .keep()
                .map_err(|e| PasteImageError::IoError(e.error.to_string()))?;
            Ok((path, info))
        }
        Err(e) => {
            #[cfg(target_os = "linux")]
            {
                try_wsl_clipboard_fallback(&e).or(Err(e))
            }
            #[cfg(not(target_os = "linux"))]
            {
                Err(e)
            }
        }
    }
}

#[cfg(target_os = "android")]
pub fn paste_image_to_temp_png() -> Result<(PathBuf, PastedImageInfo), PasteImageError> {
    // Keep error consistent with paste_image_as_png.
    Err(PasteImageError::ClipboardUnavailable(
        "clipboard image paste is unsupported on Android".into(),
    ))
}

/// Attempt WSL fallback for clipboard image paste.
///
/// If clipboard is unavailable (common under WSL because arboard cannot access
/// the Windows clipboard), attempt a WSL fallback that calls PowerShell on the
/// Windows side to write the clipboard image to a temporary file, then return
/// the corresponding WSL path.
#[cfg(target_os = "linux")]
fn try_wsl_clipboard_fallback(
    error: &PasteImageError,
) -> Result<(PathBuf, PastedImageInfo), PasteImageError> {
    use PasteImageError::ClipboardUnavailable;
    use PasteImageError::NoImage;

    if !super::super::is_wsl_session() || !matches!(error, ClipboardUnavailable(_) | NoImage(_)) {
        return Err(error.clone());
    }

    tracing::debug!("attempting Windows PowerShell clipboard fallback");
    let Some(win_path) = try_dump_windows_clipboard_image() else {
        return Err(error.clone());
    };

    tracing::debug!("powershell produced path: {}", win_path);
    let Some(mapped_path) = convert_windows_path_to_wsl(&win_path) else {
        return Err(error.clone());
    };

    let Ok((w, h)) = image::image_dimensions(&mapped_path) else {
        return Err(error.clone());
    };

    // Return the mapped path directly without copying.
    // The file will be read and base64-encoded during serialization.
    Ok((
        mapped_path,
        PastedImageInfo {
            width: w,
            height: h,
            encoded_format: EncodedImageFormat::Png,
        },
    ))
}

/// Try to call a Windows PowerShell command (several common names) to save the
/// clipboard image to a temporary PNG and return the Windows path to that file.
/// Returns None if no command succeeded or no image was present.
#[cfg(target_os = "linux")]
fn try_dump_windows_clipboard_image() -> Option<String> {
    // Powershell script: save image from clipboard to a temp png and print the path.
    // Force UTF-8 output to avoid encoding issues between powershell.exe (UTF-16LE default)
    // and pwsh (UTF-8 default).
    let script = r#"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $img = Get-Clipboard -Format Image; if ($img -ne $null) { $p=[System.IO.Path]::GetTempFileName(); $p = [System.IO.Path]::ChangeExtension($p,'png'); $img.Save($p,[System.Drawing.Imaging.ImageFormat]::Png); Write-Output $p } else { exit 1 }"#;

    for cmd in ["powershell.exe", "pwsh", "powershell"] {
        match std::process::Command::new(cmd)
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            // Executing PowerShell command
            Ok(output) => {
                if output.status.success() {
                    // Decode as UTF-8 (forced by the script above).
                    let win_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !win_path.is_empty() {
                        tracing::debug!("{} saved clipboard image to {}", cmd, win_path);
                        return Some(win_path);
                    }
                } else {
                    tracing::debug!("{} returned non-zero status", cmd);
                }
            }
            Err(err) => {
                tracing::debug!("{} not executable: {}", cmd, err);
            }
        }
    }
    None
}

/// The WSL path of a Windows `C:\...` path (`/mnt/c/...`); UNC paths have none.
#[cfg(target_os = "linux")]
fn convert_windows_path_to_wsl(input: &str) -> Option<PathBuf> {
    if input.starts_with("\\\\") {
        return None;
    }

    let drive_letter = input.chars().next()?.to_ascii_lowercase();
    if !drive_letter.is_ascii_lowercase() {
        return None;
    }

    if input.get(1..2) != Some(":") {
        return None;
    }

    let mut result = PathBuf::from(format!("/mnt/{drive_letter}"));
    for component in input
        .get(2..)?
        .trim_start_matches(['\\', '/'])
        .split(['\\', '/'])
        .filter(|component| !component.is_empty())
    {
        result.push(component);
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn pasted_image_format_png_jpeg_unknown() {
        assert_eq!(
            pasted_image_format(Path::new("/a/b/c.PNG")),
            EncodedImageFormat::Png
        );
        assert_eq!(
            pasted_image_format(Path::new("/a/b/c.jpg")),
            EncodedImageFormat::Jpeg
        );
        assert_eq!(
            pasted_image_format(Path::new("/a/b/c.JPEG")),
            EncodedImageFormat::Jpeg
        );
        assert_eq!(
            pasted_image_format(Path::new("/a/b/c")),
            EncodedImageFormat::Other
        );
        assert_eq!(
            pasted_image_format(Path::new("/a/b/c.webp")),
            EncodedImageFormat::Other
        );
    }

    #[test]
    fn pasted_image_format_with_windows_style_paths() {
        assert_eq!(
            pasted_image_format(Path::new(r"C:\\a\\b\\c.PNG")),
            EncodedImageFormat::Png
        );
        assert_eq!(
            pasted_image_format(Path::new(r"C:\\a\\b\\c.jpeg")),
            EncodedImageFormat::Jpeg
        );
        assert_eq!(
            pasted_image_format(Path::new(r"C:\\a\\b\\noext")),
            EncodedImageFormat::Other
        );
    }

    #[test]
    fn the_errors_read_as_the_oracles_do() {
        assert_eq!(
            PasteImageError::ClipboardUnavailable("x".into()).to_string(),
            "clipboard unavailable: x"
        );
        assert_eq!(
            PasteImageError::NoImage("x".into()).to_string(),
            "no image on clipboard: x"
        );
        assert_eq!(
            PasteImageError::EncodeFailed("x".into()).to_string(),
            "could not encode image: x"
        );
        assert_eq!(
            PasteImageError::IoError("x".into()).to_string(),
            "io error: x"
        );
        assert_eq!(EncodedImageFormat::Png.label(), "PNG");
        assert_eq!(EncodedImageFormat::Jpeg.label(), "JPEG");
        assert_eq!(EncodedImageFormat::Other.label(), "IMG");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_windows_path_maps_under_mnt() {
        assert_eq!(
            convert_windows_path_to_wsl(r"C:\Users\Alice\Pictures\example image.png"),
            Some(PathBuf::from(
                "/mnt/c/Users/Alice/Pictures/example image.png"
            ))
        );
        assert_eq!(convert_windows_path_to_wsl(r"\\server\share\a.png"), None);
        assert_eq!(convert_windows_path_to_wsl("relative.png"), None);
    }
}
