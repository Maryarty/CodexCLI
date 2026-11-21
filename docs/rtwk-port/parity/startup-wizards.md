# startup-wizards parity

Slice S02/S03 (part): the provider wizards of the sign-in step and the GovCloud guidance.
Code: core `core/src/screens/onboarding/provider/{bedrock,gateway,wire}.rs` (pure: stage
machines, `Intent`/`apply`/`back`/`answer`, the protocol mapping of every `Setup`),
`core/src/screens/onboarding/auth.rs` (the sign-in `State`s); views
`src/pages/startup/onboarding/signin/provider/{bedrock,gateway,gateway_login,gov_cloud}.rs`
(`BedrockWizard`, `GatewayWizard`: `StatefulWidget`s holding a `TextEditingController` +
`FocusNode` per field, the shared `Options` cursor, a `ScrollController` for a short pane),
words in `provider/strings.rs`; host `host/src/{providers,startup,launch}.rs`.

## Oracle rows

| oracle | ours | state |
|---|---|---|
| bedrock.rs discovery, methods, entries (profile, API key, region, access keys), `move_provider_field`, setup, render overflow | `provider::bedrock` + `BedrockWizard` (Options, `Focus.on_focus_change` marks the field, `SingleChildScrollView` keeps the footer) | ported (earlier commits) |
| provider_setup.rs `FieldDisplay::FocusedSecret` | `TextField::obscure_text` hides all | gap G55 |
| bedrock.rs `GovCloudWarning`, `check_gov_cloud`, `acknowledge_gov_cloud` | `Prompt::GovCloud` first in `due`, host `check_gov_cloud` at launch and the acknowledgment write, view `GovCloudGuidance` over a `ScrollController` (tap on the acknowledge row, wheel) | ported (e0817bbf1, bd3343df9) |
| gateway.rs picker (saved gateways, `Create`, selected row, notice, load error) | `gateway::Stage::Picker` + `GatewayWizard` picker (`Options`; a tap moves the cursor, a tap on the cursor row accepts); `Setup::GatewayList`, `GatewaySelect(id)` | ported |
| gateway.rs form (Name*, Base URL*, API key variable, Save, validation, suggestions, Ctrl+Space completion) | `gateway::Form` + form view (`TextField` per field, `Focus.on_focus_change` marks, `GestureDetector` Save, listener line for the suggestion, scroll to the marked row after a resize); `Setup::GatewayWrite(Patch)` | ported |
| gateway.rs unconfirmed save (list read again, note) and refused selection (picker kept, message) | `Answer::{GatewayUnconfirmed, GatewaySelected}` -> stage + notice | ported |
| gateway_login.rs states (starting, browser URL, failed) | `State::{GatewayOAuth, GatewayOAuthFailed, GatewayConfigured}`, seeds `GatewayOauth`/`GatewayOauthFailed`, view `gateway_login.rs`, fixtures `gateway_oauth_{starting,browser,failed}.jsonl` | ported (views and states) |
| gateway_login.rs runtime (`authenticate_gateway`, `GatewayOAuthRead/Login`, device login, URL changes, Enter retry, cancel on exit) | - | not built: handover item below |

## Tests

| oracle test | ours | state |
|---|---|---|
| bedrock_tests.rs discovery_prioritizes_profiles_and_groups_other_sign_in_methods | `tests/pages/signin_wizards.rs` + shard + core `bedrock::tests` | pass |
| bedrock_tests.rs credential_entries_share_region_and_keep_aws_secrets_hidden | `tests/pages/signin_wizards.rs` + core | pass; shard pending(G55) |
| bedrock_tests.rs gov_cloud_guidance_requires_acknowledgement, gov_cloud_guidance_scrolls_in_short_panes | shard (`bedrock_gov_cloud` fixture, snaps `gov_cloud_warning`, `gov_cloud_warning_short`; at the oracle's own 72x24 and 24x6, the generator's 80x20 / 80x8 differed in the wrap) + page tests (same names, tap, wheel) | pass |
| gateway_tests.rs x3 | shard `tests/oracle/signin.rs` (snaps, three) + `tests/pages/signin_wizards.rs` `mod gateway` | pass |
| auth.rs bedrock_option_requires_feature_and_api_login_permission | shard | pass |
| - (host) | `host/src/startup.rs` `the_built_provider_wizards_are_offered` (`Providers { bedrock, gcp, gateway }` all true) | pass |
| - (core) | `seeds_read_the_gateway_sign_in_states` (onboarding.rs), `the_gateway_sign_in_ends_the_step_only_when_configured` (auth.rs), `gateway::tests` | pass |

Page tests of `mod gateway` (`tests/pages/signin_wizards.rs`): select a saved gateway during
sign-in; create highlights the saved gateway until explicitly selected; Esc goes one stage
back; the load error leaves by back; an unconfirmed save reads the list again and says so; a
refused selection keeps the picker with the message; save names the field that is missing;
Ctrl+Space completes a saved variable; Tab moves the mark and Enter goes on. Mouse
(ADR-011): a tap on a picker row moves the cursor then accepts; a tap marks a field and Save
saves when marked; a tap on Back leaves the load error.

Bites (each shown red once, then restored): dropping the blank line after a method row fails
the Bedrock discovery test; `obscure_text(false)` fails the credentials test; without the
`awaiting` reset on a wizard's Esc (auth.rs `ProviderBack`) the discovery test never reopens
the wizard. Gateway (`/tmp/sw/bites.py`, each mutation red in its page/host test, then
restored): Esc leaves instead of going one stage back; the completion insert dropped; a tap on
Save does not mark first; a tap does not accept a row; `1` does not leave the load error; the
unconfirmed-save notice dropped; the refused-selection message dropped; Shift+Tab walking
forward; a missing name reported as a missing URL; the post-frame reveal after a resize
dropped; the host without the gateway arm of `wizard_built`.

## Binary run (`/tmp/sw/drive.py`, own pty driver: check.py only has chat scenarios; empty
CODEX_HOME with the three wizard features on; the real embedded server answers
`model_provider/list` and `config/write`; 80x24, oracle `~/.cache/codexcli/oracle-bin/codex`)

- Gateway, keys (menu, another way, empty picker, form, filled form, bad variable, server
  refusal, created picker, Esc, menu again, saved picker, selected): 11 of 12 checkpoints equal.
- 07_server_refusal differs (rows 9-11 of the capture), oracle first, then ours:
  `Enter an HTTP(S) base URL without credentials, a query, or a fragment.` /
  `modelProvider/config/write failed: Enter an HTTP(S) base URL without` +
  `credentials, a query, or a fragment. (code -32600), data:` +
  `{"config_write_error_code":"configValidationError"}`. Cause: `host/src/in_process.rs`
  `sender` maps a `TypedRequestError::Server` to `message: error.to_string()` ("the client's
  own text"), the oracle reads `source.message`; core `wire.rs` already reads `message`.
  Not mine: handover.
- Bedrock (discovery, Down+Enter to the next screen, Esc back, Esc to the menu): 4 of 4 equal.
- Mouse (ours only: the oracle has none): a tap on the Create row opens the form; a tap on the
  Base URL / Name value focuses it and the paste lands there; a tap on Save marks it, the second
  saves; the picker shows the saved gateway with the cursor on it; a tap on that row selects
  it (`Custom provider configured`). A tap on a label (not the value) does nothing, as in
  Flutter where only the field takes the pointer.

## Checks (final, after the last commit)

- fmt; `clippy -q --workspace --all-targets -D warnings`: clean.
- Whole `nextest run --workspace`: 1134 of 1135 pass; the one red is
  `shell_agents::typing_a_word_asks_the_server_once` (the agents debounce on the real clock,
  tm task `fake-loop-barrier`; fails alone too, passes sometimes; not this area).
- Oracle shards signin, onboarding, startup_prompts under `--profile oracle`: 71 run, 62 pass,
  9 pending by design: G48 (`api_key_entry_snapshots`, startup_prompts `prompt_snapshot`),
  G49 (`narrow_picker_keeps_selected_choice_and_escape_uses_session`), G50
  (`provider_picker_snapshots`), G51 (`onboarding::long_paths_keep_disclosure_and_choices_visible`),
  G55 (`credential_entries_share_region_and_keep_aws_secrets_hidden`), and three
  `startup-prompts` rows (inline presentation x2, the cwd persistence history line), owned by
  that task. `shards.py --check`: 0 files stale.
- A flake fixed on the way: the gateway page tests pressed several keys in a row; each key of a
  field walk goes through the model and the host, so a loaded run read a stale mark
  (`create_highlights...`: the URL got a `?` from a late Backspace). Every key of
  `mod gateway` now settles (`key(h, ..)`), five runs in a row green.

## Deltas

- The server-refusal text above (host `sender`).
- The oracle's unreachable "Enter a base URL…" error after a trimmed save is skipped (the
  form never submits without the required fields; `save_names_the_field_that_is_missing`).
- The vim-insert hint of the oracle's form is not ported.
- G55 (focused secret shows its last character), one rtwk-gaps row.
- `Rows` is private in `sheets/select/rows.rs`: the picker uses `Options` (no row type
  needed outside).

## Handover

- `host/src/in_process.rs` `sender`: for `TypedRequestError::Server { source, .. }` use
  `source.message.clone()` as the message (removes the refusal-text delta).
- Gateway OAuth runtime (the oracle's `authenticate_gateway`): primary sign-in first when
  needed, `GatewayOAuthRead` / `GatewayOAuthLogin` asks, `needs_device_login` /
  `prefer_device_login`, `GatewayOAuthChanged` URL updates, Enter retry on the failed state,
  cancel on exit. The states, seeds, views and fixtures exist; nothing sets them from a login.
