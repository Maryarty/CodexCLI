use codex_tui_rtwk::services::images::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn a_quoted_path_loses_its_quotes() {
    assert_eq!(
        normalize_pasted_path("'/tmp/My File.png'"),
        Some(PathBuf::from("/tmp/My File.png"))
    );
    assert_eq!(
        normalize_pasted_path("\"/tmp/a b.png\""),
        Some(PathBuf::from("/tmp/a b.png"))
    );
}

#[test]
fn a_file_url_is_its_decoded_path() {
    assert_eq!(
        normalize_pasted_path("file:///tmp/a%20b.png"),
        Some(PathBuf::from("/tmp/a b.png"))
    );
    assert_eq!(normalize_pasted_path("file://elsewhere/tmp/a.png"), None);
}

#[test]
fn a_shell_escaped_word_is_one_path_and_two_words_are_none() {
    assert_eq!(
        normalize_pasted_path(r"/tmp/a\ b.png"),
        Some(PathBuf::from("/tmp/a b.png"))
    );
    assert_eq!(normalize_pasted_path("/tmp/a.png /tmp/b.png"), None);
}

#[test]
fn a_windows_path_keeps_its_backslashes() {
    assert_eq!(
        normalize_pasted_path(r"'C:\Users\Alice\My File.jpeg'"),
        Some(PathBuf::from(r"C:\Users\Alice\My File.jpeg"))
    );
    assert_eq!(
        normalize_pasted_path(r"\\server\share\a.png"),
        Some(PathBuf::from(r"\\server\share\a.png"))
    );
}

#[test]
fn only_an_image_file_is_taken() {
    let dir = std::env::temp_dir().join(format!("codex-images-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let png = dir.join("a.png");
    std::fs::write(&png, b"\x89PNG\r\n\x1a\n0000").expect("png");
    let text = dir.join("a.txt");
    std::fs::write(&text, b"hello world, not an image").expect("txt");
    assert_eq!(pasted_image(&png.to_string_lossy()), Some(png.clone()));
    assert_eq!(pasted_image(&text.to_string_lossy()), None);
    assert_eq!(pasted_image(&dir.to_string_lossy()), None, "a directory");
    assert_eq!(pasted_image("hello"), None);
    std::fs::remove_dir_all(&dir).ok();
}
