//! What the system clipboard holds, read (oracle `clipboard_paste.rs`): its
//! text ([`TextReader`]) and its first image ([`paste_image`]). Flutter's
//! `Clipboard.getData`. The app asks; the blocking reads run off the async
//! workers. No `Command` asks for either yet: the composer's paste (Ctrl+V /
//! Ctrl+Shift+V, `[Image #N]`) will, once core-wiring defines it (the pack
//! names the Command/Ev it expects).

mod decode;
#[cfg(target_os = "macos")]
mod macos;
mod picture;
mod text;

pub use picture::EncodedImageFormat;
pub use picture::PasteImageError;
pub use picture::PastedImageInfo;
pub use picture::paste_image;
pub use picture::paste_image_as_png;
pub use picture::paste_image_to_temp_png;
pub use picture::pasted_image_format;
pub use text::TextReader;
