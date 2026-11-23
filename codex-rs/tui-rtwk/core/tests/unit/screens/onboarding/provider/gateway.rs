use codex_tui_core::screens::onboarding::auth::provider::Answer;
use codex_tui_core::screens::onboarding::auth::provider::Effect;
use codex_tui_core::screens::onboarding::auth::provider::Settled;
use codex_tui_core::screens::onboarding::auth::provider::Setup;
use codex_tui_core::screens::onboarding::auth::provider::gateway::*;

fn saved(id: &str, variable: Option<&str>) -> Saved {
    Saved {
        id: id.into(),
        name: id.to_uppercase(),
        variable: variable.map(str::to_owned),
    }
}

fn page(gateways: Vec<Saved>) -> Page {
    Page {
        gateways,
        default_id: "openai".into(),
    }
}

fn picking(gateways: Vec<Saved>) -> Gateway {
    Gateway {
        stage: Stage::Picking(Picker {
            list: page(gateways),
            lands_on: None,
            notice: None,
        }),
    }
}

fn gateway_rows(gateway: &Gateway) -> usize {
    match &gateway.stage {
        Stage::Picking(picker) => picker.list.gateways.len(),
        _ => 0,
    }
}

fn fields(name: &str, url: &str, variable: &str) -> Fields {
    Fields {
        name: name.into(),
        url: url.into(),
        variable: variable.into(),
    }
}

fn editing(gateway: &mut Gateway) {
    let create = match &gateway.stage {
        Stage::Picking(picker) => picker.list.create_row(),
        _ => panic!("not picking"),
    };
    assert_eq!(create, gateway_rows(gateway));
    gateway.apply(Intent::Create, &mut || 0);
}

// gateway_tests.rs:33 select_a_saved_gateway_during_sign_in (the wizard's part)
#[test]
fn the_picker_opens_by_asking_for_the_saved_gateways() {
    let (gateway, effect) = Gateway::start(&mut || 4);
    assert_eq!(effect, Effect::Ask(4, Setup::GatewayList));
    assert!(gateway.waits());
    let mut gateway = gateway;
    assert_eq!(gateway.back(), Effect::Leave);
}

#[test]
fn the_list_answer_opens_the_picker_or_says_why_not() {
    let (mut gateway, _) = Gateway::start(&mut || 4);
    let found = page(vec![saved("office", None)]);
    // A stale answer is dropped.
    let settled = gateway.answer(3, Ok(Answer::Gateways(found.clone())), &mut || 9);
    assert_eq!(settled, Settled::Waiting);
    assert!(gateway.waits());
    gateway.answer(4, Ok(Answer::Gateways(found.clone())), &mut || 9);
    assert_eq!(
        gateway.stage,
        Stage::Picking(Picker {
            list: found,
            lands_on: None,
            notice: None
        })
    );
    let (mut gateway, _) = Gateway::start(&mut || 5);
    gateway.answer(5, Err(POLICY.into()), &mut || 9);
    assert_eq!(gateway.stage, Stage::LoadError(POLICY.into()));
    assert_eq!(gateway.back(), Effect::Leave);
}

#[test]
fn a_saved_gateway_is_selected_by_asking_the_server_to_make_it_the_default() {
    let mut gateway = picking(vec![saved("office", None), saved("team", None)]);
    let effect = gateway.apply(Intent::Use(1), &mut || 7);
    assert_eq!(effect, Effect::Ask(7, Setup::GatewaySelect("team".into())));
    // No cancel and no second activation while the server works.
    assert_eq!(gateway.back(), Effect::None);
    assert_eq!(gateway.apply(Intent::Use(0), &mut || 8), Effect::None);
    assert_eq!(
        gateway.answer(7, Ok(Answer::GatewaySelected), &mut || 9),
        Settled::Selected("team".into())
    );
}

#[test]
fn a_refused_selection_returns_to_the_picker_on_its_row_with_the_message() {
    let mut gateway = picking(vec![saved("office", None), saved("team", None)]);
    gateway.apply(Intent::Use(1), &mut || 7);
    gateway.answer(7, Err("overridden".into()), &mut || 9);
    let Stage::Picking(picker) = &gateway.stage else {
        panic!("not picking: {:?}", gateway.stage);
    };
    assert_eq!(picker.lands_on, Some(1));
    assert_eq!(picker.notice.as_deref(), Some("overridden"));
}

// editor.rs `ProviderForm::input`: from Save the form validates and saves (the move from
// field to field is the view's).
#[test]
fn save_validates_before_it_asks() {
    let mut gateway = picking(Vec::new());
    editing(&mut gateway);
    assert!(gateway.is_text_entry());
    let enter = |gateway: &mut Gateway, f: Fields| gateway.apply(Intent::Save(f), &mut || 7);
    let refuse = |gateway: &mut Gateway, f: Fields, error: FormError| {
        assert_eq!(enter(gateway, f), Effect::None);
        let Stage::Editing { form, .. } = &gateway.stage else {
            panic!("not editing");
        };
        assert_eq!(form.error, Some(error));
    };
    refuse(&mut gateway, fields("  ", "u", ""), FormError::EnterName);
    refuse(&mut gateway, fields("Team", " ", ""), FormError::EnterUrl);
    refuse(
        &mut gateway,
        fields("Team", "https://t/v1", "TEAM_KEY-"),
        FormError::BadVariable,
    );
    refuse(
        &mut gateway,
        fields("Team", "https://t/v1", "1KEY"),
        FormError::BadVariable,
    );
    let effect = enter(
        &mut gateway,
        fields(" Team ", " https://t/v1 ", " TEAM_KEY "),
    );
    assert_eq!(
        effect,
        Effect::Ask(
            7,
            Setup::GatewayWrite(Patch {
                name: "Team".into(),
                url: "https://t/v1".into(),
                variable: Some("TEAM_KEY".into()),
            })
        )
    );
    assert!(gateway.waits());
    assert_eq!(gateway.back(), Effect::None);
}

#[test]
fn esc_on_the_form_returns_to_the_picker_on_the_create_row() {
    let mut gateway = picking(vec![saved("office", None)]);
    editing(&mut gateway);
    assert_eq!(gateway.back(), Effect::None);
    let Stage::Picking(picker) = &gateway.stage else {
        panic!("not picking");
    };
    assert_eq!(picker.lands_on, Some(1));
    assert_eq!(gateway.back(), Effect::Leave);
}

// create_highlights_the_saved_gateway_until_explicitly_selected
#[test]
fn a_saved_gateway_is_marked_in_the_picker_not_selected() {
    let mut gateway = picking(vec![saved("office", None)]);
    editing(&mut gateway);
    gateway.apply(Intent::Save(fields("Team", "https://t/v1", "")), &mut || 7);
    let settled = gateway.answer(7, Ok(Answer::GatewaySaved(saved("team", None))), &mut || 9);
    assert_eq!(settled, Settled::Waiting);
    let Stage::Picking(picker) = &gateway.stage else {
        panic!("not picking");
    };
    assert_eq!(picker.lands_on, Some(1));
    assert_eq!(picker.list.gateways.len(), 2);
}

#[test]
fn a_refused_save_returns_to_the_form_with_the_message() {
    let mut gateway = picking(Vec::new());
    editing(&mut gateway);
    gateway.apply(Intent::Save(fields("Team", "https://t/v1?", "")), &mut || 7);
    gateway.answer(7, Err("Enter an HTTP(S) base URL.".into()), &mut || 9);
    let Stage::Editing { form, .. } = &gateway.stage else {
        panic!("not editing");
    };
    assert_eq!(
        form.error,
        Some(FormError::Server("Enter an HTTP(S) base URL.".into()))
    );
}

#[test]
fn a_save_without_a_confirmed_reply_reads_the_list_again_with_a_note() {
    let mut gateway = picking(Vec::new());
    editing(&mut gateway);
    gateway.apply(Intent::Save(fields("Team", "https://t/v1", "")), &mut || 7);
    let settled = gateway.answer(7, Ok(Answer::GatewayUnconfirmed), &mut || 8);
    assert_eq!(settled, Settled::Ask(8, Setup::GatewayList));
    gateway.answer(
        8,
        Ok(Answer::Gateways(page(vec![saved("team", None)]))),
        &mut || 9,
    );
    let Stage::Picking(picker) = &gateway.stage else {
        panic!("not picking");
    };
    assert_eq!(picker.notice.as_deref(), Some(UNCONFIRMED_CREATE));
}

// editor.rs `suggestion`: a saved variable the text starts.
#[test]
fn the_variable_field_completes_from_the_saved_variables() {
    let mut gateway = picking(vec![
        saved("a", Some("TEAM_API_KEY")),
        saved("b", Some("TEAM_OTHER")),
        saved("c", None),
    ]);
    editing(&mut gateway);
    let Stage::Editing { form, .. } = &gateway.stage else {
        panic!("not editing");
    };
    assert_eq!(form.suggestion("TEAM"), Some("TEAM_API_KEY"));
    assert_eq!(form.suggestion("TEAM_O"), Some("TEAM_OTHER"));
    assert_eq!(form.suggestion("TEAM_API_KEY"), None);
    assert_eq!(form.suggestion(""), None);
    assert_eq!(form.suggestion("OTHER"), None);
}
