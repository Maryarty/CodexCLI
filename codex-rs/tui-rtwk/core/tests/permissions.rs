#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the `/permissions` family of S14: the sheets the screen asks for, the
//! server's catalog as a fact and the rules of `chatwidget/permission_popups.rs`,
//! `permissions_menu.rs` and `permission_discovery.rs` (a row opens the next
//! menu or is final, a disabled row is not chosen, one request serves the
//! menu that waits and an answer of an older one is dropped). The sheet's own page is the
//! test's `Sandbox::sheet`, moved by the pure `PermissionsMenu::choose` the way the UI's sheet
//! moves it.

use codex_tui_core::app::AppFx;
use codex_tui_core::screens::select::permissions::{
    Approval, CANCEL, CONTINUE, Chosen, Current, Disabled, Discovery, Failure, Heading, Kind,
    Label, Notice, Page, PermissionsMenu, Phase, Preset, RETRY, Recommend, Reviewer, Selection,
};
use codex_tui_core::screens::select::{Ev, Intent as SelectIntent, Opening, Picked};
use pretty_assertions::assert_eq;
use serde_json::json;

const ASK: &str = ":workspace";
const APPROVE: &str = ":workspace+auto-review";
const FULL: &str = ":danger-full-access";
const READ_ONLY: &str = ":read-only";

fn current(value: serde_json::Value) -> Current {
    serde_json::from_value(value).expect("current")
}

fn discovery(value: serde_json::Value) -> Discovery {
    serde_json::from_value(value).expect("discovery")
}

/// The catalog `Discovery::local` makes in the oracle's tests: the three
/// built-in profiles.
fn local() -> serde_json::Value {
    json!({"profiles": [{"id": ":read-only"}, {"id": ":workspace"}, {"id": ":danger-full-access"}]})
}

/// The picker's own model: these are state tests of the screen, not of the
/// app that answers its asks (`core/tests/select_live.rs` is that). The `sheet` stands in for the
/// UI's sheet: the page it is on, taken from the show the model made.
#[derive(Default)]
struct Sandbox {
    model: codex_tui_core::screens::select::Model,
    fx: AppFx,
    sheet: Option<PermissionsMenu>,
}

fn sandbox() -> Sandbox {
    Sandbox::default()
}

/// The presenter: the sheet of each show waiting opens, and the show is said to be shown.
fn present(sandbox: &mut Sandbox) {
    let shows: Vec<_> = sandbox
        .model
        .shows
        .pending()
        .map(|show| (show.seq, show.opening.clone()))
        .collect();
    for (seq, opening) in shows {
        if let Opening::Permissions(menu) = opening {
            sandbox.sheet = Some(menu);
        }
        codex_tui_core::screens::select::apply_intent(
            &mut sandbox.model,
            SelectIntent::Shown(seq),
            &mut sandbox.fx,
        );
    }
}

fn event(sandbox: &mut Sandbox, ev: Ev) {
    codex_tui_core::screens::select::apply(&mut sandbox.model, ev, &mut sandbox.fx);
    present(sandbox);
}

fn intent(sandbox: &mut Sandbox, intent: SelectIntent) {
    codex_tui_core::screens::select::apply_intent(&mut sandbox.model, intent, &mut sandbox.fx);
    present(sandbox);
}

/// A row of the sheet's page was chosen: what the sheet does with `PermissionsMenu::choose`.
fn choose(sandbox: &mut Sandbox, id: &str) {
    let menu = sandbox.sheet.clone().expect("a sheet is open");
    match menu.choose(&sandbox.model.profiles, id) {
        Some(Chosen::Done(selection)) => {
            intent(sandbox, SelectIntent::SetPermissions(selection));
            sandbox.sheet = None;
        }
        Some(Chosen::Menu(next)) => sandbox.sheet = Some(next),
        Some(Chosen::Retry) => intent(sandbox, SelectIntent::LoadProfiles),
        None => {}
    }
}

/// Esc: the sheet closes and says so.
fn leave(sandbox: &mut Sandbox) {
    intent(sandbox, SelectIntent::AbandonPermissions);
    sandbox.sheet = None;
}

fn model(sandbox: &Sandbox) -> &codex_tui_core::screens::select::Model {
    &sandbox.model
}

fn page(sandbox: &Sandbox) -> Page {
    let menu = sandbox.sheet.as_ref().expect("a sheet is open");
    menu.page(&sandbox.model.profiles, &sandbox.model.catalog)
}

fn phase(sandbox: &Sandbox) -> Phase {
    let menu = sandbox.sheet.as_ref().expect("a sheet is open");
    menu.phase(&sandbox.model.profiles)
}

fn ids(sandbox: &Sandbox) -> Vec<String> {
    page(sandbox).rows.into_iter().map(|row| row.id).collect()
}

fn kinds(sandbox: &Sandbox) -> Vec<Kind> {
    page(sandbox).rows.into_iter().map(|row| row.kind).collect()
}

fn marks(sandbox: &Sandbox) -> Vec<bool> {
    page(sandbox)
        .rows
        .into_iter()
        .map(|row| row.current)
        .collect()
}

fn reasons(sandbox: &Sandbox) -> Vec<Option<Disabled>> {
    page(sandbox)
        .rows
        .into_iter()
        .map(|row| row.disabled)
        .collect()
}

/// How many sheets are open: the one, or none once it closed.
fn depth(sandbox: &Sandbox) -> usize {
    usize::from(sandbox.sheet.is_some())
}

fn picks(sandbox: &Sandbox) -> Vec<Selection> {
    model(sandbox)
        .picked
        .iter()
        .filter_map(|picked| match picked {
            Picked::Permissions(selection) => Some(selection.clone()),
            _ => None,
        })
        .collect()
}

fn open(sandbox: &mut Sandbox, current: Current) {
    intent(sandbox, SelectIntent::OpenPermissions(current));
}

fn legacy(value: serde_json::Value) -> Sandbox {
    let mut sandbox = sandbox();
    open(&mut sandbox, current(value));
    sandbox
}

/// A thread's menu on the three built-in profiles, the catalog answered.
fn profiles(value: serde_json::Value, catalog: serde_json::Value) -> Sandbox {
    let mut sandbox = sandbox();
    event(&mut sandbox, Ev::Listed(discovery(catalog)));
    let mut value = value;
    value["discover"] = json!(true);
    open(&mut sandbox, current(value));
    sandbox
}

#[test]
fn the_legacy_list_has_ask_for_approval_and_full_access() {
    let sandbox = legacy(json!({}));
    assert_eq!(ids(&sandbox), vec![ASK, FULL]);
    assert_eq!(
        page(&sandbox).heading,
        Heading::Permissions { notice: None }
    );
}

#[test]
fn approve_for_me_is_listed_only_with_the_guardian_feature() {
    let sandbox = legacy(json!({"guardian": true}));
    assert_eq!(ids(&sandbox), vec![ASK, APPROVE, FULL]);
}

#[test]
fn the_current_mark_follows_the_preset_and_the_reviewer() {
    assert_eq!(
        marks(&legacy(json!({"guardian": true, "matches": "auto"}))),
        vec![true, false, false]
    );
    assert_eq!(
        marks(&legacy(
            json!({"guardian": true, "matches": "auto", "reviewer": "auto_review"})
        )),
        vec![false, true, false]
    );
    // a mandatory auto-review marks `Approve for me` whatever the sandbox is
    assert_eq!(
        marks(&legacy(json!({"guardian": true, "reviewer": "auto_review",
                             "approval": "on-request", "auto_required": true}))),
        vec![false, true, false]
    );
    // full access is current by its preset alone
    assert_eq!(
        marks(&legacy(json!({"matches": "full-access"}))),
        vec![false, true]
    );
}

#[test]
fn a_preset_the_config_forbids_is_disabled_with_the_configs_reason() {
    let mut sandbox = legacy(json!({"limits": {"approvals": [
        {"approval": "never", "reason": "this message should be printed in the description"}]}}));
    assert_eq!(
        reasons(&sandbox),
        vec![
            None,
            Some(Disabled::Reason(
                "this message should be printed in the description".into()
            ))
        ]
    );
    // a disabled row is not chosen
    choose(&mut sandbox, FULL);
    assert_eq!(depth(&sandbox), 1);
    assert_eq!(ids(&sandbox), vec![ASK, FULL]);
    assert!(picks(&sandbox).is_empty());
}

#[test]
fn a_reviewer_the_config_forbids_disables_its_rows() {
    let sandbox = legacy(json!({"guardian": true, "limits": {
        "reviewer_user": "user review is off", "reviewer_auto": "auto-review is off"}}));
    assert_eq!(
        reasons(&sandbox),
        vec![
            Some(Disabled::Reason("user review is off".into())),
            Some(Disabled::Reason("auto-review is off".into())),
            Some(Disabled::Reason("user review is off".into())),
        ]
    );
}

#[test]
fn a_legacy_pick_is_final_and_closes_the_menu() {
    let mut sandbox = legacy(json!({"guardian": true}));
    choose(&mut sandbox, APPROVE);
    assert_eq!(
        picks(&sandbox),
        vec![Selection::Preset {
            preset: Preset::Auto,
            reviewer: Reviewer::AutoReview
        }]
    );
    assert_eq!(depth(&sandbox), 0);
}

#[test]
fn full_access_asks_before_it_applies() {
    let mut sandbox = legacy(json!({}));
    choose(&mut sandbox, FULL);
    // the menu is replaced by the confirmation; nothing is picked yet
    assert_eq!(depth(&sandbox), 1);
    assert!(picks(&sandbox).is_empty());
    assert_eq!(page(&sandbox).heading, Heading::FullAccess { cyber: None });
    assert_eq!(ids(&sandbox), vec![CONTINUE, CANCEL]);
    assert_eq!(kinds(&sandbox), vec![Kind::Continue, Kind::Cancel]);
    choose(&mut sandbox, CONTINUE);
    assert_eq!(
        picks(&sandbox),
        vec![Selection::Preset {
            preset: Preset::FullAccess,
            reviewer: Reviewer::User
        }]
    );
    assert_eq!(depth(&sandbox), 0);
}

#[test]
fn declining_the_confirmation_opens_the_menu_again() {
    let mut sandbox = legacy(json!({"guardian": true}));
    choose(&mut sandbox, FULL);
    choose(&mut sandbox, CANCEL);
    assert!(picks(&sandbox).is_empty());
    assert_eq!(depth(&sandbox), 1);
    assert_eq!(ids(&sandbox), vec![ASK, APPROVE, FULL]);
}

#[test]
fn esc_on_the_confirmation_closes_without_a_pick() {
    let mut sandbox = legacy(json!({}));
    choose(&mut sandbox, FULL);
    leave(&mut sandbox);
    assert_eq!(depth(&sandbox), 0);
    assert!(picks(&sandbox).is_empty());
}

#[test]
fn a_cyber_models_confirmation_recommends_what_is_available() {
    for (available, expected) in [(true, Recommend::AutoReview), (false, Recommend::Ask)] {
        let mut sandbox = legacy(json!({"cyber": true, "auto_available": available}));
        choose(&mut sandbox, FULL);
        assert_eq!(
            page(&sandbox).heading,
            Heading::FullAccess {
                cyber: Some(expected)
            }
        );
    }
}

#[test]
fn a_threads_menu_waits_for_the_catalog_with_one_request() {
    let mut sandbox = legacy(json!({"discover": true}));
    assert!(matches!(sandbox.sheet, Some(PermissionsMenu::Catalog(_))));
    assert_eq!(phase(&sandbox), Phase::Loading);
    assert_eq!(kinds(&sandbox), vec![Kind::Loading]);
    assert_eq!(reasons(&sandbox), vec![Some(Disabled::Idle)]);
    let request = model(&sandbox).profiles.pending;
    assert!(request.is_some());
    // opening again while the request is out reuses it
    open(&mut sandbox, current(json!({"discover": true})));
    assert_eq!(model(&sandbox).profiles.pending, request);
}

#[test]
fn the_answer_turns_the_wait_of_the_menu_into_the_catalog() {
    let mut sandbox = legacy(
        json!({"discover": true, "guardian": true, "matches": "auto",
                                    "approval": "on-request"}),
    );
    let request = model(&sandbox).profiles.pending.expect("asked");
    event(
        &mut sandbox,
        Ev::Discovered {
            request,
            result: Ok(discovery(local())),
        },
    );
    assert_eq!(model(&sandbox).profiles.pending, None);
    // the same menu shows the catalog now: it is a fact, not a reopened menu
    assert_eq!(phase(&sandbox), Phase::Listed);
    assert_eq!(ids(&sandbox), vec![ASK, APPROVE, FULL, READ_ONLY]);
    assert_eq!(marks(&sandbox), vec![true, false, false, false]);
}

#[test]
fn an_answer_of_another_request_is_dropped() {
    let mut sandbox = legacy(json!({"discover": true}));
    let request = model(&sandbox).profiles.pending.expect("asked");
    event(
        &mut sandbox,
        Ev::Discovered {
            request: request + 7,
            result: Ok(discovery(local())),
        },
    );
    assert_eq!(model(&sandbox).profiles.pending, Some(request));
    assert_eq!(kinds(&sandbox), vec![Kind::Loading]);
    assert!(model(&sandbox).profiles.found.is_none());
}

#[test]
fn a_menu_opens_on_the_catalog_held_without_a_request() {
    let sandbox = profiles(json!({}), local());
    assert_eq!(model(&sandbox).profiles.pending, None);
    assert_eq!(ids(&sandbox), vec![ASK, FULL, READ_ONLY]);
}

#[test]
fn a_failed_read_offers_retry_and_retry_asks_again() {
    let mut sandbox = legacy(json!({"discover": true}));
    let request = model(&sandbox).profiles.pending.expect("asked");
    let failure = Failure::Other("disconnected".into());
    event(
        &mut sandbox,
        Ev::Discovered {
            request,
            result: Err(failure.clone()),
        },
    );
    assert_eq!(
        page(&sandbox).heading,
        Heading::Permissions {
            notice: Some(Notice::Failed(failure))
        }
    );
    assert_eq!(phase(&sandbox), Phase::Failed);
    assert_eq!(ids(&sandbox), vec![RETRY]);
    assert_eq!(kinds(&sandbox), vec![Kind::Retry]);
    assert!(model(&sandbox).profiles.found.is_none());
    choose(&mut sandbox, RETRY);
    let again = model(&sandbox).profiles.pending;
    assert!(again.is_some() && again != Some(request));
    assert_eq!(model(&sandbox).profiles.failure, None);
    assert_eq!(depth(&sandbox), 1);
    assert_eq!(kinds(&sandbox), vec![Kind::Loading]);
}

#[test]
fn a_scope_change_drops_the_answer_and_dismisses_by_the_scope_fact() {
    let mut sandbox = legacy(json!({"discover": true}));
    let request = model(&sandbox).profiles.pending.expect("asked");
    let scope = model(&sandbox).profiles.scope;
    event(&mut sandbox, Ev::ScopeChanged);
    // the pages that wait on the catalog compare the scope they opened under to this one
    assert_eq!(model(&sandbox).profiles.scope, scope + 1);
    assert_eq!(model(&sandbox).profiles.pending, None);
    event(
        &mut sandbox,
        Ev::Discovered {
            request,
            result: Ok(discovery(local())),
        },
    );
    assert!(model(&sandbox).profiles.found.is_none());
    assert_eq!(model(&sandbox).profiles.failure, None);
}

#[test]
fn a_scope_change_keeps_the_legacy_list_and_the_confirmation() {
    let mut sandbox = legacy(json!({}));
    event(&mut sandbox, Ev::ScopeChanged);
    assert_eq!(ids(&sandbox), vec![ASK, FULL]);
    choose(&mut sandbox, FULL);
    event(&mut sandbox, Ev::ScopeChanged);
    assert_eq!(ids(&sandbox), vec![CONTINUE, CANCEL]);
}

#[test]
fn the_profile_rows_are_the_built_in_modes_then_the_servers_own() {
    let sandbox = profiles(
        json!({"guardian": true}),
        json!({"profiles": [
            {"id": ":read-only"}, {"id": ":workspace"}, {"id": ":danger-full-access"},
            {"id": "locked-down", "description": "Inspect and patch only approved workspace files."},
            {"id": "web-enabled", "allowed": false}]}),
    );
    assert_eq!(
        ids(&sandbox),
        vec![ASK, APPROVE, FULL, READ_ONLY, "locked-down", "web-enabled"]
    );
    assert_eq!(
        kinds(&sandbox)[4..],
        [
            Kind::Profile {
                description: Some("Inspect and patch only approved workspace files.".into())
            },
            Kind::Profile { description: None },
        ]
    );
    // a profile the requirements do not allow is disabled by them
    assert_eq!(
        reasons(&sandbox),
        vec![None, None, None, None, None, Some(Disabled::Requirements)]
    );
}

#[test]
fn the_active_profile_marks_its_row_and_the_mode_must_agree_with_the_policy() {
    // a custom profile is current by its id
    let sandbox = profiles(
        json!({"profile": "locked-down", "approval": "on-request"}),
        json!({"profiles": [{"id": ":workspace"}, {"id": "locked-down"}]}),
    );
    assert_eq!(ids(&sandbox), vec![ASK, FULL, READ_ONLY, "locked-down"]);
    assert_eq!(marks(&sandbox), vec![false, false, false, true]);
    // a built-in mode is current when the profile, the policy and the reviewer agree
    let sandbox = profiles(
        json!({"profile": ":read-only", "approval": "on-request"}),
        json!({"profiles": [{"id": ":workspace"}, {"id": ":read-only"}]}),
    );
    assert_eq!(marks(&sandbox), vec![false, false, true]);
    let sandbox = profiles(
        json!({"profile": ":read-only", "approval": "never"}),
        json!({"profiles": [{"id": ":workspace"}, {"id": ":read-only"}]}),
    );
    assert_eq!(marks(&sandbox), vec![false, false, false]);
}

#[test]
fn requirements_disable_by_profile_policy_and_reviewer() {
    let sandbox = profiles(
        json!({"guardian": true}),
        json!({"profiles": [{"id": ":read-only"}, {"id": ":workspace"}, {"id": ":danger-full-access"}],
               "requirements": {
                   "profiles": {":workspace": true, ":read-only": false, ":danger-full-access": true},
                   "approvals": ["on-request"],
                   "reviewers": ["user"]}}),
    );
    assert_eq!(
        reasons(&sandbox),
        vec![
            None,                         // Ask for approval
            Some(Disabled::Requirements), // Approve for me: reviewer not allowed
            Some(Disabled::Requirements), // Full Access: policy `never` not allowed
            Some(Disabled::Requirements), // Read Only: profile not allowed
        ]
    );
}

#[test]
fn an_empty_catalog_keeps_only_the_active_profile() {
    let sandbox = profiles(
        json!({"profile": "current-profile"}),
        json!({"profiles": []}),
    );
    assert_eq!(ids(&sandbox), vec!["current-profile"]);
    assert_eq!(kinds(&sandbox), vec![Kind::Unlisted]);
    assert_eq!(marks(&sandbox), vec![true]);
    assert_eq!(reasons(&sandbox), vec![Some(Disabled::NotOnServer)]);
    assert_eq!(
        page(&sandbox).heading,
        Heading::Permissions {
            notice: Some(Notice::NoProfiles)
        }
    );
}

#[test]
fn an_empty_catalog_without_an_active_profile_has_no_rows() {
    let sandbox = profiles(json!({}), json!({"profiles": []}));
    assert!(ids(&sandbox).is_empty());
}

#[test]
fn an_active_profile_the_server_does_not_list_is_added_when_nothing_is_current() {
    let sandbox = profiles(
        json!({"profile": "retired", "approval": "never"}),
        json!({"profiles": [{"id": ":workspace"}, {"id": ":read-only"}, {"id": ":danger-full-access"}]}),
    );
    assert_eq!(ids(&sandbox).last().map(String::as_str), Some("retired"));
    assert_eq!(reasons(&sandbox).last(), Some(&Some(Disabled::NotOnServer)));
}

#[test]
fn a_builtin_mode_of_the_catalog_is_final_and_carries_its_policy() {
    let mut sandbox = profiles(json!({"guardian": true}), local());
    choose(&mut sandbox, APPROVE);
    assert_eq!(
        picks(&sandbox),
        vec![Selection::Profile {
            id: ":workspace".into(),
            approval: Some(Approval::OnRequest),
            reviewer: Some(Reviewer::AutoReview),
            label: Label::Mode {
                preset: Preset::Auto,
                reviewer: Reviewer::AutoReview
            },
            unchanged: false,
        }]
    );
    assert_eq!(depth(&sandbox), 0);
}

#[test]
fn a_servers_profile_is_final_with_neither_policy_nor_reviewer() {
    let mut sandbox = profiles(
        json!({}),
        json!({"profiles": [{"id": ":workspace"}, {"id": "locked-down"}]}),
    );
    choose(&mut sandbox, "locked-down");
    assert_eq!(
        picks(&sandbox),
        vec![Selection::Profile {
            id: "locked-down".into(),
            approval: None,
            reviewer: None,
            label: Label::Profile("locked-down".into()),
            unchanged: false,
        }]
    );
}

#[test]
fn full_access_of_the_catalog_asks_first_and_applies_as_a_profile() {
    let mut sandbox = profiles(json!({}), local());
    choose(&mut sandbox, FULL);
    assert!(picks(&sandbox).is_empty());
    assert_eq!(ids(&sandbox), vec![CONTINUE, CANCEL]);
    choose(&mut sandbox, CONTINUE);
    assert_eq!(
        picks(&sandbox),
        vec![Selection::Profile {
            id: ":danger-full-access".into(),
            approval: Some(Approval::Never),
            reviewer: Some(Reviewer::User),
            label: Label::Mode {
                preset: Preset::FullAccess,
                reviewer: Reviewer::User
            },
            unchanged: false,
        }]
    );
    // declining reopens the catalog's menu, not the legacy list
    let mut sandbox = profiles(json!({}), local());
    choose(&mut sandbox, FULL);
    choose(&mut sandbox, CANCEL);
    assert_eq!(ids(&sandbox), vec![ASK, FULL, READ_ONLY]);
}

#[test]
fn a_disabled_profile_row_is_not_chosen() {
    let mut sandbox = profiles(
        json!({}),
        json!({"profiles": [{"id": ":workspace"}, {"id": "web-enabled", "allowed": false}]}),
    );
    choose(&mut sandbox, "web-enabled");
    assert!(picks(&sandbox).is_empty());
    assert_eq!(depth(&sandbox), 1);
}

#[test]
fn an_id_that_is_no_row_changes_nothing() {
    let mut sandbox = legacy(json!({}));
    choose(&mut sandbox, "nope");
    assert_eq!(depth(&sandbox), 1);
    assert!(picks(&sandbox).is_empty());
}

#[test]
fn a_catalog_that_lists_an_id_twice_is_refused() {
    use codex_app_server_protocol::PermissionProfileSummary;
    let summary = |id: &str| PermissionProfileSummary {
        id: id.into(),
        description: None,
        allowed: true,
    };
    assert_eq!(
        Discovery::from_wire(vec![summary("a"), summary("a")], None),
        Err(Failure::Duplicate)
    );
    let found = Discovery::from_wire(vec![summary("a"), summary("b")], None).expect("distinct");
    assert_eq!(found.profiles.len(), 2);
}

#[test]
fn the_wire_requests_and_answers_decode() {
    use codex_app_server_protocol::{ClientRequest, RequestId};
    use codex_tui_core::screens::select::permissions as perms;
    let request = perms::profiles_request(
        RequestId::Integer(3),
        Some("next".into()),
        Some("/work".into()),
    );
    let ClientRequest::PermissionProfileList { params, .. } = request else {
        panic!("a profile list request");
    };
    assert_eq!(
        (
            params.limit,
            params.cursor.as_deref(),
            params.cwd.as_deref()
        ),
        (Some(100), Some("next"), Some("/work"))
    );
    assert!(matches!(
        perms::requirements_request(RequestId::Integer(4)),
        ClientRequest::ConfigRequirementsRead { .. }
    ));
    let (profiles, next) = perms::profiles_page(json!({
        "data": [{"id": ":workspace", "description": null, "allowed": true}],
        "nextCursor": "c2"
    }))
    .expect("a page");
    assert_eq!((profiles.len(), next.as_deref()), (1, Some("c2")));
    let required = perms::requirements(json!({"requirements": {
        "allowedApprovalPolicies": ["on-request"],
        "allowedPermissionProfiles": {":workspace": true}}}))
    .expect("requirements")
    .expect("some");
    let found = Discovery::from_wire(profiles, Some(&required)).expect("found");
    assert_eq!(
        found.disabled(":workspace", Some(Approval::Never), None),
        Some(Disabled::Requirements)
    );
    assert_eq!(
        found.disabled(":workspace", Some(Approval::OnRequest), None),
        None
    );
    assert_eq!(
        found.disabled(":nope", None, None),
        Some(Disabled::NotOnServer)
    );
}

/// The opener takes the config's part from the status facts the host read
/// (guardian, requirements, limits) and the model of the thread.
#[test]
fn the_opener_carries_the_configs_facts() {
    use codex_tui_core::screens::chat::StatusFacts;
    use codex_tui_core::screens::select::permissions::{Facts, Limit, Limits};
    let facts = StatusFacts {
        model: Some("gpt-sol".into()),
        permissions: Facts {
            guardian: true,
            auto_required: true,
            auto_available: true,
            limits: Limits {
                approvals: vec![Limit {
                    approval: codex_app_server_protocol::AskForApproval::Never,
                    reason: "forbidden by requirements".into(),
                }],
                reviewer_user: Some("no user reviewer".into()),
                reviewer_auto: None,
            },
        },
        ..StatusFacts::default()
    };
    let current = Current::from_facts(&facts);
    assert!(current.guardian && current.auto_required && current.auto_available);
    assert_eq!(current.limits, facts.permissions.limits);
    assert_eq!(current.model.as_deref(), Some("gpt-sol"));
    assert!(current.discover);
}
