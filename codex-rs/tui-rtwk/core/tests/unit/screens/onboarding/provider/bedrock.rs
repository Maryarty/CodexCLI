use codex_tui_core::screens::onboarding::auth::provider::Answer;
use codex_tui_core::screens::onboarding::auth::provider::Effect;
use codex_tui_core::screens::onboarding::auth::provider::Setup;
use codex_tui_core::screens::onboarding::auth::provider::bedrock::*;

fn profile(name: &str, region: Option<&str>) -> Profile {
    Profile {
        name: name.into(),
        region: region.map(Into::into),
    }
}

fn env(kind: CredentialKind, region: Option<&str>) -> EnvCredential {
    EnvCredential {
        kind,
        region: region.map(Into::into),
    }
}

fn found(profiles: Vec<Profile>, environment: Vec<EnvCredential>) -> Bedrock {
    Bedrock::discovered(Discovery {
        profiles,
        environment,
    })
}

fn run(bedrock: &mut Bedrock, intent: Intent) -> Effect {
    bedrock.apply(intent, &mut || 7)
}

const ALL: [Method; 4] = [
    Method::ManualProfile,
    Method::AccessKeys,
    Method::EnvironmentInstructions,
    Method::ApiKey,
];

// bedrock_tests.rs:87 discovery_prioritizes_profiles_and_groups_other_sign_in_methods
#[test]
fn discovery_prioritizes_profiles_and_groups_other_sign_in_methods() {
    let mut bedrock = Bedrock::default();
    assert_eq!(
        run(&mut bedrock, Intent::Discover),
        Effect::Ask(7, Setup::BedrockDiscover)
    );
    assert_eq!(bedrock.stage, Stage::Discovering(7));
    // Esc while setting up returns to the first list.
    bedrock.stage = Stage::Configuring {
        request: 2,
        credential: Credential::Environment,
        region: "us-east-1".into(),
    };
    assert_eq!(bedrock.back(), Effect::None);
    assert_eq!(bedrock.stage, Stage::Methods(List::Detected));

    let engineering = profile("engineering", Some("us-east-2"));
    let keys = env(CredentialKind::AccessKeys, Some("us-west-2"));
    for (profiles, environment, expected) in [
        (
            vec![engineering.clone()],
            vec![keys.clone()],
            vec![
                Method::Profile(0),
                Method::Environment,
                Method::OtherMethods,
            ],
        ),
        (
            vec![
                engineering.clone(),
                profile("production", Some("us-west-2")),
            ],
            vec![keys.clone()],
            vec![
                Method::Profile(0),
                Method::Profile(1),
                Method::Environment,
                Method::OtherMethods,
            ],
        ),
        (
            Vec::new(),
            vec![keys.clone()],
            vec![Method::Environment, Method::OtherMethods],
        ),
        (Vec::new(), Vec::new(), ALL.to_vec()),
    ] {
        assert_eq!(found(profiles, environment).methods(), expected);
    }
    let mut both = found(
        vec![engineering],
        vec![env(CredentialKind::BedrockApiKey, Some("us-east-2"))],
    );
    assert_eq!(
        run(&mut both, Intent::Authenticate(Method::OtherMethods)),
        Effect::None
    );
    assert_eq!(both.methods(), ALL.to_vec());
    // The environment's list alone has no descriptions.
    assert!(!found(Vec::new(), vec![keys]).describes());
    assert!(found(Vec::new(), Vec::new()).describes());
}

// bedrock_tests.rs:385 credential_entries_share_region_and_keep_aws_secrets_hidden
#[test]
fn credential_entries_share_region_and_keep_aws_secrets_hidden() {
    for (profiles, environment, method) in [
        (
            vec![profile("engineering", None)],
            Vec::new(),
            Method::Profile(0),
        ),
        (
            Vec::new(),
            vec![env(CredentialKind::AccessKeys, None)],
            Method::Environment,
        ),
    ] {
        let mut bedrock = found(profiles, environment);
        run(&mut bedrock, Intent::Authenticate(method));
        assert!(matches!(bedrock.stage, Stage::RegionEntry { .. }));
        assert!(bedrock.is_text_entry());
    }
    for stage in [
        Stage::ProfileEntry,
        Stage::ApiKeyEntry,
        Stage::AccessKeyEntry { field: 0 },
        Stage::RegionEntry {
            credential: Credential::Environment,
            prefill: String::new(),
        },
    ] {
        let bedrock = Bedrock {
            stage,
            ..Bedrock::default()
        };
        assert!(bedrock.is_text_entry());
    }
}

#[test]
fn a_region_found_sets_up_at_once_and_a_refusal_returns_to_it() {
    let mut bedrock = found(vec![profile("engineering", Some("us-east-2"))], Vec::new());
    let effect = run(&mut bedrock, Intent::Authenticate(Method::Profile(0)));
    let credential = Credential::Profile("engineering".into());
    assert_eq!(
        effect,
        Effect::Ask(
            7,
            Setup::Bedrock(Params {
                credential: credential.clone(),
                region: "us-east-2".into(),
            })
        )
    );
    // No second submission while it runs; a stale answer is dropped.
    assert_eq!(run(&mut bedrock, Intent::Submit("x".into())), Effect::None);
    assert!(!bedrock.answer(6, Ok(Answer::Written)));
    assert!(!bedrock.answer(7, Err("denied".into())));
    assert_eq!(
        bedrock.stage,
        Stage::RegionEntry {
            credential,
            prefill: "us-east-2".into(),
        }
    );
    assert_eq!(bedrock.error, Some(Error::Setup("denied".into())));
    run(&mut bedrock, Intent::Submit(" us-west-1 ".into()));
    assert!(bedrock.answer(7, Ok(Answer::Written)));
}

#[test]
fn access_keys_move_on_and_go_back_to_a_missing_field() {
    let mut bedrock = Bedrock {
        stage: Stage::AccessKeyEntry { field: 0 },
        ..Bedrock::default()
    };
    let keys = |a: &str, b: &str, c: &str| [a.to_owned(), b.to_owned(), c.to_owned()];
    run(&mut bedrock, Intent::SubmitKeys(keys("", "", "")));
    assert_eq!(bedrock.stage, Stage::AccessKeyEntry { field: 0 });
    run(&mut bedrock, Intent::SubmitKeys(keys("AKIA", "", "")));
    assert_eq!(bedrock.stage, Stage::AccessKeyEntry { field: 1 });
    run(&mut bedrock, Intent::Field(2));
    run(&mut bedrock, Intent::SubmitKeys(keys("AKIA", " ", "")));
    assert_eq!(bedrock.stage, Stage::AccessKeyEntry { field: 1 });
    run(&mut bedrock, Intent::Field(2));
    run(&mut bedrock, Intent::SubmitKeys(keys("AKIA", "secret", "")));
    assert_eq!(
        bedrock.stage,
        Stage::RegionEntry {
            credential: Credential::AccessKeys {
                access_key_id: "AKIA".into(),
                secret_access_key: "secret".into(),
                session_token: None,
            },
            prefill: String::new(),
        }
    );
}

#[test]
fn a_failed_discovery_offers_every_method_with_its_message() {
    let mut bedrock = Bedrock {
        stage: Stage::Discovering(3),
        ..Bedrock::default()
    };
    assert!(!bedrock.answer(3, Err("no daemon".into())));
    assert_eq!(bedrock.methods(), ALL.to_vec());
    assert_eq!(bedrock.error, Some(Error::Discover("no daemon".into())));
    // Esc on the first list leaves; on another, returns to it.
    run(&mut bedrock, Intent::Authenticate(Method::ApiKey));
    assert_eq!(bedrock.back(), Effect::None);
    assert_eq!(bedrock.back(), Effect::Leave);
}
