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
| gateway_login.rs runtime: `authenticate_gateway` (server restart with the selected gateway, account read, primary login first, gateway OAuth), `GatewayOAuthRead/Login`, `needs_device_login` short circuit, URL changes (`GatewayOAuthChanged`), Enter retry, cancel on exit | core `onboarding/gateway.rs` (`Phase`: Idle, Starting, Reading, Primary, OAuth, Done; requests numbered, a stale answer dropped), `Ask::{SelectGateway, GatewayLogins, GatewayOAuth, CancelGatewayOAuth}`, `Ev::{GatewayStarted, GatewayLogins, GatewayOAuth, GatewayOAuthChanged}`, `Intent::RetryGatewayOAuth`; host `host/src/gateway.rs` (reads, login, `prefer_device_login`), `startup.rs` arms, `in_process.rs` restart with an ack; page: Enter and a tap of the retry line (`Msg::Retry`) | ported; the device-code screen (`start_if_needed`, `run_gateway_device_login`) is not built: see Handover |

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
  `sender` mapped a `TypedRequestError::Server` to `message: error.to_string()`, the oracle
  reads `source.message`. FIXED by `startup-followup` (below): all 12 checkpoints are equal
  again, 07 included.
- Bedrock (discovery, Down+Enter to the next screen, Esc back, Esc to the menu): 4 of 4 equal.
- Mouse (ours only: the oracle has none): a tap on the Create row opens the form; a tap on the
  Base URL / Name value focuses it and the paste lands there; a tap on Save marks it, the second
  saves; the picker shows the saved gateway with the cursor on it; a tap on that row selects
  it (`Custom provider configured`). A tap on a label (not the value) does nothing, as in
  Flutter where only the field takes the pointer.

## Gateway OAuth runtime (`startup-followup`)

Flow, as the oracle (`lib.rs` `selected_gateway` -> `authenticate_gateway` ->
`start_gateway_oauth`), which runs after the onboarding screen (the trust step included), so
ours runs after all steps are done and before `ReloadConfig`/`Launch`: `SelectGateway`
(config read again, must select that gateway, embedded server restarted with it; the host
answers only after the pump swapped its handle, `Restart.done`), `GatewayLogins` (account read,
`gateway_oauth` of the provider), the primary login on the sign-in step's own states when
due, `GatewayOAuth` (`GatewayOAuthRead`; a login unless signed in or a device code is the
way), `GatewayOAuthChanged` shows the URL, a failure waits for Enter or a tap of the retry
line, Ctrl+C sends `CancelGatewayOAuth`. `Model::done(SignIn)` is false while the phase is
active, so the page stays and the launch waits.

Server-refusal text, SCOPED: the oracle reads `source.message` only for the GCP setup and the
gateway wizard (`gcp.rs`, `app/provider/rpc.rs`); Bedrock, archive, trust, account and the
gateway OAuth show the client's `<method> failed: <message> (code n)`. The brief's global
"Server errors use `source.message`" would have changed the latter, so core
`provider/wire.rs::reads_message_only(&ClientRequest)` names the requests (GcpSetup,
ModelProviderList, ModelProviderConfigWrite, ConfigValueWrite of `model_provider`) and
`in_process.rs::error_text` applies it; the test shows both texts.

Tests (new): core `core/tests/onboarding/gateway.rs` 11 (the sequence, the trust first,
a stale answer dropped, a lost connection, the retry, the exit cancel, ...); `wire.rs`
`reads_message_only` (1); host `gateway.rs` 8 (a fake `Server`: login due, signed in, device
preferred, another provider, decode text, refusal text, environment), `startup.rs` 4 (restart
acknowledged before the answer, provider mismatch ends the start, reload error, login +
cancel), `in_process.rs` 2 (`error_text`); page `tests/pages/signin_wizards.rs`
`mod gateway_oauth` 6 (starting, URL, Enter retry with a stale first answer, tap of the retry
line (the message is no button), success asks the launch, Ctrl+C cancels) and
`select_a_saved_gateway_during_sign_in` now also asks `SelectGateway` from the wizard's own
answer. Bites shown: the Enter arm switched off -> `enter_retries_a_failed_sign_in` red; the
`follow_provider` call after `Ev::Auth` removed -> `select_a_saved_gateway_during_sign_in`
red (the binary run found it: with the wizard's answer the provider step stayed open and the
launch never came; tests that set the state by `Ev::SignInState` did not show it).
Behaviour found on the way: oracle `BedrockConfigured`/`GcpConfigured`/`GatewaySelected`
are `StepState::Complete`, the screen ends at once; ours kept the provider step open for
Bedrock/GCP after an answer (stuck on the configured line in the binary). Fixed by the same
call; the two tests that waited for that line (`signin::gcp_setup_is_asked_of_the_host_...`,
`signin_wizards::credential_entries_share_region_...`) wait for the launch ask instead.
R1/layers: the blocking `webbrowser` check is the service `services::startup::browser_available`;
core holds no prose (the decode-error frame is the host's).

Binary run (`/tmp/sw/drive.py`, 80x24, own CODEX_HOME, the real embedded server): the
12 checkpoints of the gateway keys script: 12 of 12 equal with the oracle, 07_server_refusal
included. A second script (create `Team gateway`, select it): the picker (01) equal; after the
selection ours restarts the server, reads the account and reaches the chat (`>_ OpenAI Codex`,
the composer, `gpt-6.2-sol default`); the oracle does too. The chat rows differ (the
chat page's own task: the oracle's release-notes box, the path of the temp dir, the model).
A gateway with `gateway_oauth` was not driven in the binary (no server to sign in to): the
sequence is the page and host tests'.

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

- The server-refusal text: gone (scoped `source.message`, above).
- Windows `prefer_device_login` (`has_http_handler` instead of `Browser::is_available`) is not
  carried: this port targets macOS/Linux, the Windows branch is a `cfg(windows)` of the oracle.
- The oracle's unreachable "Enter a base URL…" error after a trimmed save is skipped (the
  form never submits without the required fields; `save_names_the_field_that_is_missing`).
- The vim-insert hint of the oracle's form is not ported.
- G55 (focused secret shows its last character), one rtwk-gaps row.
- `Rows` is private in `sheets/select/rows.rs`: the picker uses `Options` (no row type
  needed outside).

## Handover

- The oracle's device-code sign-in (`device_login.rs` `start_if_needed`, `lib.rs`
  `run_gateway_device_login`, `onboarding_screen.rs`): when the launch's provider is a gateway
  that needs sign-in and a device code is the way (`prefer_device_login`: SSH, Linux without a
  display, no browser), the oracle asks `GatewayOAuthLogin{device_auth: true}`, draws the code
  and the URL, waits for `GatewayOAuthChanged` of that `login_id` and cancels on exit. NOT
  built: ours skips the browser login in that case (`authenticate`'s `needs_device_login`
  short circuit, as the oracle's `start_gateway_oauth`), so on such a machine the gateway is
  left unsigned and no code is shown. It needs a screen of its own (a startup state, an ask,
  the event with the `login_id`, a cancel), and runs for any launch, not only after a wizard.
- The page and the host are one step apart on the Windows branch above.
