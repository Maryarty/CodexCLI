use codex_tui_rtwk::services::remote::*;

#[test]
fn only_a_remote_server_and_the_cloud_have_a_workspace_of_their_own() {
    assert!(!Connection::Embedded.uses_remote_workspace());
    assert!(!Connection::LocalDaemon.uses_remote_workspace());
    assert!(Connection::Remote.uses_remote_workspace());
    assert!(Connection::Cloud.uses_remote_workspace());
}
