//! Which of a palette's colours a thread gets (oracle: `thread_color.rs`
//! `color_for_id`). Identity only: the slot depends on the full thread id and
//! the palette's length, never on names, list order or other threads. The
//! colours themselves are the app's theme's (`ThreadPalette`), not core's.

/// The slot of `id` among `colors` colours; `None` with no colours (the
/// oracle then draws the terminal's default colour).
///
/// Explicit FNV-1a over the id's text, rather than a process-random or
/// Rust-version-specific hasher, so a thread keeps its colour everywhere.
pub fn slot(id: &str, colors: usize) -> Option<usize> {
    let hash = id.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    let colors = u64::try_from(colors).ok().filter(|n| *n > 0)?;
    usize::try_from(hash % colors).ok()
}
