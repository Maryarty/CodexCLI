# onboarding: parity

Slice: the StartupPage's sign-in step (S02) and the provider step (S03).
The page and its step controller come from `startup-flow`. Built here:

- **Core**: `core/src/screens/onboarding/auth.rs`.
  - The offer, the sign-in states and the intents.
  - `AuthResult { request }` answers, matched by number; a stale one is dropped.
  - The login completion and account updates.
  - `Exit` cancels a login in progress.
- **Protocol**: `core/src/protocol/onboarding.rs`.
  - `sign_in_offer` reads the login policy.
  - `login_start`, `login_cancel` and `login_outcome` cover `account/login/start` and `account/login/cancel`.
- **Host**: `host/src/startup.rs`.
  - The launch facts weigh the forced login method, the provider wizards (by target, features, provider flag and model provider) and `OPENAI_API_KEY`.
  - It runs `Ask::Login` and `Ask::CancelLogin`.
  - It opens the browser for a ChatGPT login unless the workspace is remote.
- **View**: `src/pages/startup/onboarding/signin/`.
  - `SignInStep` is a `StatefulWidget`. Its state holds the highlight, a `TextEditingController` and `FocusNode` for the key entry, and the activation guard.
  - The guard runs in the nodes' `on_key_event`. Repeats and releases are caught by an `InputListener::with_raw_key_repeats`.
  - The key entry takes focus in a post-frame callback (`addPostFrameCallback`).

The provider wizards (Bedrock, Google Cloud, gateway) are **not built**. Choosing one moves the page to the provider step, whose place is empty; Esc there returns to the menu.

Oracle: codex-tui `onboarding/auth.rs`, `auth/headless_chatgpt_login.rs`, `auth/activation_key_guard.rs` (+ `_tests`), `onboarding_screen.rs` (quit, animation suppression); wizards in `provider_setup.rs`, `gateway*.rs`, `bedrock*.rs`, `gcp*.rs`.

## Oracle tests

Shard `signin`: `tests/oracle/signin.rs`. Run it with `--profile oracle -E 'test(signin::)'`. Page tests: `tests/pages/signin.rs`. Core state tests: `core/src/screens/onboarding/auth.rs` and `core/src/screens/onboarding.rs`. Protocol tests: `core/src/protocol/onboarding.rs`.

| oracle (file:line name) | ours | ported | n/a: reason (owner) | delta |
|---|---|---|---|---|
| auth.rs:1502 api_key_entry_snapshots | shard; snap api_key_entry (80x14) green | partly | snap api_key_entry_narrow pending G46 | G46 |
| auth.rs:1541 bedrock_option_requires_feature_and_api_login_permission | shard (menus at 76x19, the 12 rows of the Bedrock menu) | yes | | |
| auth.rs:1708 continue_in_browser_preserves_long_link_and_footer_at_narrow_width | shard, snap + OSC 8 link | yes | | |
| auth.rs:1747 chatgpt_success_message_renders_osc8_hyperlinks | shard: links, rows, dim/underline per text run | yes | the indent's 2 blanks: a blank shows no dim, and rtwk writes it as an erased cell (`Attrs::VISIBLE_ON_BLANK`), so the text cells are checked | |
| auth.rs:1859 mark_url_hyperlink_wraps_cyan_underlined_cells | shard na + link checks | na | ratatui buffer post-pass; rtwk's `Span::link` writes OSC 8 itself | |
| auth.rs:1885 mark_url_hyperlink_sanitizes_control_chars | shard na | na | same | |
| auth/activation_key_tests.rs:9 held_enter_requires_a_fresh_confirmation_to_save_a_prefilled_api_key | shard (entry screen); page test `a_held_enter_saves_the_prefilled_key_only_when_fresh` (kitty repeat/release, a fresh press saves, Left ends the guard) | yes | | |
| auth.rs:1408 login_can_complete_while_browser_is_opening | core, same name | yes | | |
| auth.rs:1525 api_key_flow_disabled_when_chatgpt_forced | core, same name | yes | | |
| auth.rs:1616 saving_api_key_is_blocked_when_chatgpt_forced | core, same name | yes | | |
| auth.rs:1633 existing_non_oauth_chatgpt_login_counts_as_signed_in | core, same name | yes | | |
| auth.rs:1649 cancel_active_attempt_resets_browser_login_state | core, same name | yes | | |
| auth.rs:1668 cancel_active_attempt_notifies_device_code_login | core, same name | yes | | |
| auth.rs:1809/1824 auth_widget_suppresses_animations_* (2) | core, same names; page test `a_login_link_hides_the_logo_and_a_quit_cancels_the_login` | yes | | |
| auth.rs:1835 device_code_login_completion_advances_to_success_message | core, same name | yes | | |
| auth.rs opens_login_browser_for_local_app_servers | host `Ask::Login` (`webbrowser::open` unless `Target::Remote`) | no test | host IO, no seam for a fake browser (onboarding) | |
| headless_chatgpt_login.rs:224 device_code_attempt_matches_only_for_matching_request_id | core `stale_auth_results_are_dropped`; page test `chatgpt_sign_in_runs_through_the_host_and_drops_stale_results` | yes | | |
| headless_chatgpt_login.rs:286 set_device_code_state_for_active_attempt_updates_only_when_active | core, same name | yes | | |
| headless_chatgpt_login.rs set_device_code_error_for_active_attempt_updates_only_when_active | core, same name | yes | | |
| onboarding_screen.rs:1016.. suppress_quit_* (4) | view: `q` quits unless the key entry holds text; ctrl+c/ctrl+d always; core `exit_cancels_the_login_in_progress` | partly | the predicate is not a pure fn here (the view's key handler); no own test for `q` while typing | |
| bedrock_tests.rs (4) gov_cloud_*, discovery_*, credential_* | shard, pending | pending | the Bedrock wizard (S03) is not built (onboarding) | |
| gateway_tests.rs (3) | shard, pending | pending | the gateway wizard is not built (onboarding) | |
| gcp_tests.rs:16 gcp_menu_gating_and_setup_flow | shard, pending; its menu snaps green in `provider_tests_sign_in_menus` | pending | the Google Cloud wizard is not built (onboarding) | |

Extra shard test: `provider_tests_sign_in_menus` (snaps sign_in_menu_short_without_cloud_wizards, gcp_sign_in_menu, gcp_sign_in_menu_short, gateway_sign_in_menu).

## Bites

Each test below was seen failing on broken code, which was then restored:

- The stale check accepting any request breaks:
  - `stale_auth_results_are_dropped`;
  - `chatgpt_sign_in_runs_through_the_host_and_drops_stale_results`.
- The guard never blocking breaks:
  - both held-Enter tests (the shard test and the page test).
- The offer not zeroing the wizards without API login breaks:
  - `the_sign_in_offer_weighs_the_login_policy`.
- `exit_cancels_the_login_in_progress` was red before `Exit` cancelled.
- In the logo and quit page test:
  - the logo shown while frozen fails the logo check;
  - the quit not sent fails the cancel check.

## Deltas and gaps

- **G46** (new, `rtwk-gaps.md`): `TextWrap::Word` breaks after a hyphen inside a word, but the oracle's `Paragraph` breaks only at blanks. Seen in the narrow API key entry; that snapshot is pending.
- **Ours, not a gap**: a line that is one word after its indent (a link or a code) wraps with `TextWrap::Grapheme`. This matches ratatui, which breaks a too-long word where it stands; `Word` would first move the word below the indent.
- **Dim**: the oracle's `.dim()` is the DIM attribute on the terminal's own colour (`Style::new().attrs(Attrs::DIM)`). The theme's `dim()` grey is not used.

## Left

- **Provider wizards (S03)**: Bedrock, Google Cloud and gateway, with their 8 oracle tests (pending).
- **Faded welcome logo**: the oracle fades and stops the logo while the API key entry is active (`Presentation::Faded`). The blossom stage (`pages::session::blossom`, outside this area) has no fade. The logo is shown animated instead.
- **Fixture `bedrock_gov_cloud`**: the pending gov-cloud tests name it, but it does not exist yet.
