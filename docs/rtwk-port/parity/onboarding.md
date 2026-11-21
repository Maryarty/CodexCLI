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
| auth.rs:1502 api_key_entry_snapshots | shard; snap api_key_entry (80x14) green | partly | snap api_key_entry_narrow pending G48 | G48 (was G46 on the branch; renumbered at landing, import holds G46) |
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
| gcp_tests.rs:16 gcp_menu_gating_and_setup_flow | shard (`providers`): menu snaps, then the wizard's `gcp_project_entry`, `gcp_location_entry`, `gcp_credentials_entry`, `gcp_configuring` green; page test `gcp_setup_is_asked_of_the_host_refused_and_retried` (fake host) holds the host half: setup params, `gcp_setup_error_narrow` rows, retry under a new number, stale answer dropped, `gcp_configured` line; core tests `the_gcp_wizard_runs_its_setup_through_an_ask`, `esc_on_the_first_gcp_stage_returns_to_the_menu`, `provider::gcp::tests` (4) | yes | the oracle test's `gcp_setup_error_narrow` and `gcp_configured` are asserted in the page test, not the shard (a debug entry has no host to answer the setup; the fixture seed cannot be extended: core/src/screens/onboarding.rs belongs to startup-deltas); the Enter that opens the wizard is held back by the guard for 1 s, so the shard test waits 1.1 s where the oracle test sends a key release (the kit has no release) | |

Extra shard test: `provider_tests_sign_in_menus` (snaps sign_in_menu_short_without_cloud_wizards, gcp_sign_in_menu, gcp_sign_in_menu_short, gateway_sign_in_menu).

## Review round 1 (providers task)

- **O1** (double ChatGPT login): `choose` ignores an activation while a login is awaited; a stale `Outcome::Browser` is cancelled by `Ask::CancelLogin`; Esc on the key entry forgets the pending save. Core tests `second_choose_while_a_login_is_awaited_is_ignored`, `stale_browser_outcome_cancels_the_login`, `cancel_forgets_the_pending_api_key`; page test extended.
- **O2** (menu on shared widgets): the held-Enter guard is `widgets::key_guard::KeyGuard` (7 tests). The menu keeps `Options`' cursor and `step` (wraps); it cannot use `Options`' two-column grid, whose rows differ from the oracle's (number and label on one row with the description under it, the caret `>`). What `options.rs` lacks for this menu: a one-column layout with a description row under each option and a caret marker.
- **N-O5**: one place (`rows` in `signin/mod.rs`) decides the ChatGPT row.
- **O3**: the blossom has a `faded` prop and a 400 ms fade; `welcome::header(.., faded)` and the page pass `SignIn::is_text_entry` (announced edits of `welcome.rs` and `onboarding/mod.rs`). Page test `the_welcome_logo_stops_and_fades_while_the_api_key_is_typed`.

## Provider wizards (providers task)

- Core: `auth::provider::{Wizard, Setup, Intent, Effect, gcp}` (the module is declared from `auth.rs`); `SignIn.wizard`, `State::ProviderConfigured(Provider)`, `Login::Provider(Setup)`, `Outcome::Provider(Result<(), String>)`, `Intent::Provider`/`ProviderBack`. The setup is asked as `Ask::Login`, answered as `Ev::Auth` (request-number guarded like every login); the host needs no new arm (`wire::login_start` maps `Login::Provider(Setup::Gcp)` to `ClientRequest::GcpSetup`, `login_outcome` to `Outcome::Provider`), so `host/src/startup.rs` is untouched and `host/src/providers.rs` does not exist yet.
- View: `signin/provider/{mod,gcp,strings}.rs`: one `TextField` per stage (keyed, post-frame focus), the footer at the bottom of the room (`frame`), `guarded_keys` shared with the step (guard and quit chords; a `q` is text).
- Announced edit outside the area: `core/src/protocol/onboarding.rs` (two match arms; the area was widened for it).

## Bites

Each test below was seen failing on broken code, which was then restored:


Each test below was seen failing on broken code, which was then restored:

- The stale check accepting any request breaks:
  - `stale_auth_results_are_dropped`;
  - `chatgpt_sign_in_runs_through_the_host_and_drops_stale_results`.
- The guard never blocking breaks:
  - both held-Enter tests (the shard test and the page test).
- The offer not zeroing the wizards without API login breaks:
  - `the_sign_in_offer_weighs_the_login_policy`.
- `exit_cancels_the_login_in_progress` was red before `Exit` cancelled.
- GCP (providers task):
  - dropping the stale-answer check of `Gcp::answer` breaks `a_refusal_returns_to_the_credentials_and_a_stale_answer_is_dropped`;
  - dropping the post-frame focus of the stage's field breaks the shard test `gcp_menu_gating_and_setup_flow` (Enter reaches no field).
- In the logo and quit page test:
  - the logo shown while frozen fails the logo check;
  - the quit not sent fails the cancel check.

## Mouse (startup-mouse task, ADR-011)

The oracle has no mouse; the port's contract is held by `tests/pages/startup_mouse.rs` (28 tests, own test binary, kit mouse events on the oracle's debug entries; startup-finish added the trust step, the import source and review, the hooks, unarchive and daemon prompts, so every startup view has a mouse test). With the mouse unused every screen equals the oracle (the signin, onboarding and startup_prompts shards stay green).

- **`Options` takes the mouse itself** (`src/widgets/options.rs`): a tap selects the row, a tap on the selected row (so also a double tap) calls the caller's `on_accept(row)` (Enter's action), `on_move(row)` tells the caller where a tap moved the cursor (the OSS description follows it), the wheel scrolls through the `ScrollController`. Cursor and scroll stay in the controllers.
- **Sign-in menu (S02) is Options rows** (`Options::view_stacked`, `StackedRow`, `Stacked`): description under the label, a gap after each row, compact (labels only) when the pane is short; the hand lines, `rows_of` re-measuring and the compact splice are gone. Disabled rows take no tap.
- **Wired**: sign-in menus (`signin/mod.rs`, `Msg::Accept`), update prompt (`update_prompt.rs`, `Msg::Accept`), startup prompts panel (`prompts/panel.rs`, `Cmd::Take` / `Cmd::Cursor`).
- **GCP wizard fields** take focus on a click natively (TextField); the test is a regression guard.
- **Tests**: signin_a_click_selects_the_row_under_it, signin_a_click_on_the_description_selects_the_row, signin_a_click_on_the_selected_row_accepts_it, signin_a_double_click_accepts_the_row, signin_a_click_selects_in_the_second_menu_too, signin_a_click_in_the_compact_menu_selects, signin_a_click_on_a_disabled_row_does_nothing, gcp_a_click_on_the_field_focuses_it, update_prompt_a_click_selects_the_row, update_prompt_a_click_on_the_selected_row_accepts_it, update_prompt_a_double_click_accepts_the_row, update_prompt_the_wheel_scrolls_the_rows, cwd_prompt_a_click_selects_the_row, cwd_prompt_a_click_on_the_selected_row_accepts_it, cwd_prompt_a_double_click_accepts_the_row, migration_prompt_a_click_selects_and_a_second_accepts, oss_prompt_a_click_moves_the_description_with_the_cursor. Red then green for the tap and accept tests (commits 88ef89fc9, fc633bdac); the wheel and GCP tests passed from the start (rtwk owns both) and the GCP one bites through Tab.
- **Caveat**: on a pane shorter than even the compact sign-in menu, the oracle cut the bottom (the hints); now the hints win and the rows scroll. No oracle test covers it.

## Deltas and gaps

- **G48** (new, `rtwk-gaps.md`; G46 on the branch, renumbered at landing): `TextWrap::Word` breaks after a hyphen inside a word, but the oracle's `Paragraph` breaks only at blanks. Seen in the narrow API key entry; that snapshot is pending.
- **Ours, not a gap**: a line that is one word after its indent (a link or a code) wraps with `TextWrap::Grapheme`. This matches ratatui, which breaks a too-long word where it stands; `Word` would first move the word below the indent.
- **Dim**: the oracle's `.dim()` is the DIM attribute on the terminal's own colour (`Style::new().attrs(Attrs::DIM)`). The theme's `dim()` grey is not used.

## Left

- **Provider wizards (S03)**: Google Cloud is built (above). Bedrock (guidance, discovery, credential entries; `bedrock.rs`, `bedrock_tests.rs`) and the gateway wizard with its login (`gateway.rs`, `gateway_login.rs`, needs the provider form of `app/provider`, not ported) are not: 7 oracle tests stay pending (`gov_cloud_*` x2, `discovery_*`, `credential_*`, `gateway_tests` x3).
- **Fixture `bedrock_gov_cloud`**: the pending gov-cloud tests name it; it needs a seed event for a wizard state (`SeedEv` in `core/src/screens/onboarding.rs`, held by startup-deltas), or key steps in the fixture script from `sign_in_bedrock`.
