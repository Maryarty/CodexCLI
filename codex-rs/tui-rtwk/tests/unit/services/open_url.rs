use std::io;
use std::sync::Arc;
use std::sync::Mutex;

use pretty_assertions::assert_eq;

use codex_tui_rtwk::services::open_url::*;
use codex_tui_rtwk::services::startup::Browser;

#[tokio::test]
async fn a_page_is_opened_with_the_browser_off_the_async_workers() {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&opened);
    let browser: Browser = Arc::new(move |url| {
        seen.lock().unwrap().push(url.to_owned());
        Ok(())
    });

    let result = open_url(browser, "https://chatgpt.com/settings/usage".to_owned()).await;

    assert_eq!(result, Ok(()));
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["https://chatgpt.com/settings/usage"]
    );
}

#[tokio::test]
async fn a_browser_that_fails_gives_back_the_page_and_the_reason() {
    let browser: Browser = Arc::new(|_| Err(io::Error::other("no browser")));

    let result = open_url(browser, "https://example.com".to_owned()).await;

    assert_eq!(
        result,
        Err(OpenFailure {
            url: "https://example.com".to_owned(),
            error: "no browser".to_owned()
        })
    );
}

#[test]
fn the_desktop_link_is_the_codex_scheme_with_the_thread() {
    assert_eq!(
        desktop_thread_url("019c5a1e-0000-7000-8000-000000000001"),
        "codex://threads/019c5a1e-0000-7000-8000-000000000001"
    );
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[tokio::test]
async fn the_desktop_app_is_only_launched_on_macos_and_windows() {
    assert_eq!(
        open_desktop_thread("t".to_owned()).await,
        Err("The Desktop app is only available on macOS and Windows".to_owned())
    );
}
