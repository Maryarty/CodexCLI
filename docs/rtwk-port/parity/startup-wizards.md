# startup-wizards parity

Slice S03 (part): the Amazon Bedrock provider wizard of the sign-in step. Code: core
`core/src/screens/onboarding/provider/bedrock.rs` (pure: `Bedrock`, `View`, `Method`,
`Credential`, `Intent`, `apply`, `back`, `answer`, `methods`, `describes`),
`core/src/screens/onboarding/provider/wire.rs` (the protocol mapping of every `Setup`: the
request and the reading of its answer, GCP's too; `gov_cloud_check` / `is_gov_cloud`),
the view `src/pages/startup/onboarding/signin/provider/bedrock.rs` (`BedrockWizard`: a
`StatefulWidget` holding a `TextEditingController` + `FocusNode` per field, the shared
`Options` cursor, a `ScrollController` for a short pane), words in `provider/strings.rs`.
The host runs the asks through `wire::login_start` / `login_outcome`, which now hand a
`Login::Provider` to `provider::wire`.

## Oracle rows

| oracle | ours | state |
|---|---|---|
| bedrock.rs `BedrockState::discovering`, `start_bedrock_discovery` | the view emits `Intent::Discover` in `init_state`; `Setup::BedrockDiscover` -> `account/bedrock/discover` | ported |
| bedrock.rs `methods`, `render_methods`, `select_method` | `Bedrock::methods`, `describes`, `apply(Choose)` | ported |
| bedrock.rs entries (profile, API key, region, access keys), `move_provider_field` | `View::*Entry`, `Intent::{Submit, SubmitKeys, Field}`; Tab/BackTab/Up/Down move the focus, the focus change (`Focus.on_focus_change`, also a click) marks the field | ported |
| bedrock.rs `start_bedrock_setup`, provider_setup.rs `spawn_provider_setup` | `Setup::Bedrock(Params)` -> `account/login/start` (API key, access keys) or `account/bedrock/setup` (profile, environment) | ported |
| bedrock.rs render overflow (footer kept, marked row scrolled in) | `SingleChildScrollView` + `ScrollController::jump_to(marked + 2 - viewport)`, footer below | ported (asserted at 72x10) |
| provider_setup.rs `FieldDisplay::FocusedSecret` (last character shown) | `TextField::obscure_text` hides all | gap G55 |
| bedrock.rs `GovCloudWarning`, `check_gov_cloud`, `acknowledge_gov_cloud` | core wire only (`gov_cloud_check`, `is_gov_cloud`) | not built: launch step, view, config write (handover) |
| gateway.rs, gateway_login.rs | - | not built (handover) |

## Tests

| oracle test | ours | state |
|---|---|---|
| bedrock_tests.rs discovery_prioritizes_profiles_and_groups_other_sign_in_methods | `tests/pages/signin_wizards.rs` (same name: every inline snapshot at 72x24, the 72x10 and 24x12 checks); shard `tests/oracle/signin.rs` (the discovering screen); core `bedrock::tests` (same name, the method lists) | pass |
| bedrock_tests.rs credential_entries_share_region_and_keep_aws_secrets_hidden | `tests/pages/signin_wizards.rs` (same name: access keys with the secrets unfocused, profile, region, environment instructions, refusal, configured); core `bedrock::tests` (same name) | pass; shard pending(G55) for the focused secret |
| bedrock_tests.rs gov_cloud_guidance_requires_acknowledgement | shard | pending (launch step not built) |
| bedrock_tests.rs gov_cloud_guidance_scrolls_in_short_panes | shard | pending (launch step not built) |
| gateway_tests.rs x3 | shard | pending (gateway not built) |
| auth.rs bedrock_option_requires_feature_and_api_login_permission | shard (unchanged) | pass |
| - (host) | `host/src/startup.rs` only_the_built_provider_wizards_are_offered (Bedrock, GCP offered; gateway not) | pass |

Bites: dropping the blank line after a method row fails the discovery test; `obscure_text(false)`
fails the credentials test; without the `awaiting` reset on a wizard's Esc (auth.rs
`ProviderBack`) the discovery test never reopens the wizard (red before the fix).

## Deltas

None besides G55 (rtwk-gaps.md, one row).
