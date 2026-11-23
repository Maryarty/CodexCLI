//! Amazon Bedrock setup (`onboarding/bedrock.rs`, `BedrockState`): the
//! credentials the app server finds are asked for first, then a method is
//! chosen (a profile found, the server environment's credentials, or one
//! entered: a profile name, access keys, a Bedrock API key), its region
//! where none came with it, and the app server sets it up; the step stays
//! "setting up" until it answers. The fields' text is the view's (it passes
//! it with the intent that needs it); the cursor of the method list is the
//! view's too (the list starts on its first row whenever it changes).

use super::{Answer, Effect, Setup};
use crate::screens::onboarding::Request;

/// An AWS profile the server found (`BedrockAwsProfile`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub region: Option<String>,
}

/// What kind of credential the server environment holds (`AwsCredentialType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialKind {
    AccessKeys,
    BedrockApiKey,
}

/// A credential of the server environment (`BedrockEnvironmentCredential`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvCredential {
    pub kind: CredentialKind,
    pub region: Option<String>,
}

/// `account/bedrock/discover`'s answer (`BedrockDiscoverResponse`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    pub profiles: Vec<Profile>,
    pub environment: Vec<EnvCredential>,
}

/// Which list of methods shows (`BedrockMethodList`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    /// What was found, then "Other AWS sign-in methods".
    Detected,
    /// Every way to enter credentials.
    All,
}

/// The credential to set up (`BedrockCredential`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Credential {
    Profile(String),
    Environment,
    AccessKeys {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
    ApiKey(String),
}

/// A row of the method list (`BedrockMethod`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Profile(usize),
    Environment,
    OtherMethods,
    ManualProfile,
    AccessKeys,
    EnvironmentInstructions,
    ApiKey,
}

/// Where the setup stands (`BedrockView`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// Shown, not asked yet: the view asks for the discovery once it is up
    /// (`start_bedrock_discovery`).
    #[default]
    Start,
    Discovering(Request),
    Methods(List),
    ProfileEntry,
    /// The access key ID, the secret and the session token; `field` is the
    /// one marked and typed in.
    AccessKeyEntry {
        field: usize,
    },
    ApiKeyEntry,
    /// The region for `credential`; its field starts with `prefill`.
    RegionEntry {
        credential: Credential,
        prefill: String,
    },
    EnvironmentInstructions,
    /// The app server sets it up; a refusal returns to the region entry
    /// with `region` in it.
    Configuring {
        request: Request,
        credential: Credential,
        region: String,
    },
}

/// What went wrong; the view says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// "Unable to check AWS credentials: {0}".
    Discover(String),
    /// "Unable to set up Amazon Bedrock: {0}".
    Setup(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bedrock {
    pub stage: Stage,
    pub discovery: Discovery,
    pub error: Option<Error>,
}

/// `account/bedrock/setup` or `account/login/start` (`start_bedrock_setup`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Params {
    pub credential: Credential,
    pub region: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The wizard is up: ask the server for the credentials it finds.
    Discover,
    /// A method of the list (Enter on the cursor's row, or its digit).
    Authenticate(Method),
    /// Enter in the profile, API key or region field, with its text.
    Submit(String),
    /// Enter in the access keys, with the three fields' text.
    SubmitKeys([String; 3]),
    /// Another access key field takes the keys (Tab, Up, Down, a click).
    Field(usize),
}

/// `ProviderInput::value`: the trimmed text, none when blank.
fn value(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

impl Bedrock {
    /// The wizard over what discovery found (`BedrockState::discovered`).
    pub fn discovered(discovery: Discovery) -> Bedrock {
        Bedrock {
            stage: Stage::Methods(List::Detected),
            discovery,
            error: None,
        }
    }

    /// The rows of the list shown (`methods`).
    pub fn methods(&self) -> Vec<Method> {
        if self.stage == Stage::EnvironmentInstructions {
            return vec![Method::OtherMethods];
        }
        let detected = self.stage == Stage::Methods(List::Detected);
        let Discovery {
            profiles,
            environment,
        } = &self.discovery;
        if detected && !profiles.is_empty() {
            let mut methods: Vec<Method> = (0..profiles.len()).map(Method::Profile).collect();
            if !environment.is_empty() {
                methods.push(Method::Environment);
            }
            methods.push(Method::OtherMethods);
            return methods;
        }
        if detected && !environment.is_empty() {
            return vec![Method::Environment, Method::OtherMethods];
        }
        vec![
            Method::ManualProfile,
            Method::AccessKeys,
            Method::EnvironmentInstructions,
            Method::ApiKey,
        ]
    }

    /// A row's description shows (`render_methods`): not on the list of the
    /// server environment's credentials alone.
    pub fn describes(&self) -> bool {
        self.stage != Stage::Methods(List::Detected)
            || !self.discovery.profiles.is_empty()
            || self.discovery.environment.is_empty()
    }

    /// A request is out (what `Esc` leaves, its answer dropped).
    pub fn waits(&self) -> bool {
        matches!(self.stage, Stage::Discovering(_) | Stage::Configuring { .. })
    }

    pub fn is_text_entry(&self) -> bool {
        matches!(
            self.stage,
            Stage::ProfileEntry
                | Stage::AccessKeyEntry { .. }
                | Stage::ApiKeyEntry
                | Stage::RegionEntry { .. }
        )
    }

    /// `enter_region`.
    fn enter_region(&mut self, credential: Credential, prefill: &str) {
        self.stage = Stage::RegionEntry {
            credential,
            prefill: prefill.to_owned(),
        };
    }

    /// `start_bedrock_setup`: set up `credential` in `region`.
    fn configure(
        &mut self,
        credential: Credential,
        region: String,
        request: &mut dyn FnMut() -> Request,
    ) -> Effect {
        let request = request();
        self.stage = Stage::Configuring {
            request,
            credential: credential.clone(),
            region: region.clone(),
        };
        Effect::Ask(request, Setup::Bedrock(Params { credential, region }))
    }

    /// Esc (`handle_key_event`'s cancel): out of the wizard from the first
    /// list (or while it is found), else back to it; a setup running is
    /// left (its answer is dropped).
    pub fn back(&mut self) -> Effect {
        self.error = None;
        if matches!(
            self.stage,
            Stage::Start | Stage::Discovering(_) | Stage::Methods(List::Detected)
        ) {
            return Effect::Leave;
        }
        self.stage = Stage::Methods(List::Detected);
        Effect::None
    }

    /// `select_method`.
    fn choose(&mut self, method: Method, request: &mut dyn FnMut() -> Request) -> Effect {
        match method {
            Method::Profile(index) => {
                let Some(profile) = self.discovery.profiles.get(index).cloned() else {
                    return Effect::None;
                };
                let credential = Credential::Profile(profile.name);
                match profile.region {
                    Some(region) => return self.configure(credential, region, request),
                    None => self.enter_region(credential, ""),
                }
            }
            Method::Environment => {
                let region = self
                    .discovery
                    .environment
                    .iter()
                    .find_map(|credential| credential.region.clone());
                match region {
                    Some(region) => return self.configure(Credential::Environment, region, request),
                    None => self.enter_region(Credential::Environment, ""),
                }
            }
            Method::OtherMethods => self.stage = Stage::Methods(List::All),
            Method::ManualProfile => self.stage = Stage::ProfileEntry,
            Method::AccessKeys => self.stage = Stage::AccessKeyEntry { field: 0 },
            Method::EnvironmentInstructions => self.stage = Stage::EnvironmentInstructions,
            Method::ApiKey => self.stage = Stage::ApiKeyEntry,
        }
        Effect::None
    }

    /// `confirm_input` for the fields of one value.
    fn submit(&mut self, text: &str, request: &mut dyn FnMut() -> Request) -> Effect {
        let Some(text) = value(text) else {
            return Effect::None;
        };
        let credential = match &self.stage {
            Stage::ProfileEntry => Credential::Profile(text),
            Stage::ApiKeyEntry => Credential::ApiKey(text),
            Stage::RegionEntry { credential, .. } => {
                return self.configure(credential.clone(), text, request);
            }
            _ => return Effect::None,
        };
        self.after_credential(credential, request)
    }

    /// `confirm_input` for the access keys: Enter moves on from a field
    /// with a value; on the last, a missing required field takes the keys.
    fn submit_keys(&mut self, texts: &[String; 3], request: &mut dyn FnMut() -> Request) -> Effect {
        let Stage::AccessKeyEntry { field } = &mut self.stage else {
            return Effect::None;
        };
        if *field < 2 {
            if value(&texts[*field]).is_some() {
                *field += 1;
            }
            return Effect::None;
        }
        let Some(access_key_id) = value(&texts[0]) else {
            *field = 0;
            return Effect::None;
        };
        let Some(secret_access_key) = value(&texts[1]) else {
            *field = 1;
            return Effect::None;
        };
        let credential = Credential::AccessKeys {
            access_key_id,
            secret_access_key,
            session_token: value(&texts[2]),
        };
        self.after_credential(credential, request)
    }

    /// A credential entered: a profile found with its region is set up at
    /// once, any other asks for the region.
    fn after_credential(
        &mut self,
        credential: Credential,
        request: &mut dyn FnMut() -> Request,
    ) -> Effect {
        if let Credential::Profile(name) = &credential
            && let Some(region) = self
                .discovery
                .profiles
                .iter()
                .find_map(|found| (found.name == *name).then(|| found.region.clone()).flatten())
        {
            return self.configure(credential, region, request);
        }
        self.enter_region(credential, "");
        Effect::None
    }

    pub fn apply(&mut self, intent: Intent, request: &mut dyn FnMut() -> Request) -> Effect {
        self.error = None;
        if matches!(self.stage, Stage::Configuring { .. }) {
            return Effect::None;
        }
        match intent {
            Intent::Discover => {
                if self.stage != Stage::Start {
                    return Effect::None;
                }
                let request = request();
                self.stage = Stage::Discovering(request);
                Effect::Ask(request, Setup::BedrockDiscover)
            }
            Intent::Authenticate(method) => {
                if !matches!(
                    self.stage,
                    Stage::Methods(_) | Stage::EnvironmentInstructions
                ) || !self.methods().contains(&method)
                {
                    return Effect::None;
                }
                self.choose(method, request)
            }
            Intent::Submit(text) => self.submit(&text, request),
            Intent::SubmitKeys(texts) => self.submit_keys(&texts, request),
            Intent::Field(index) => {
                if let Stage::AccessKeyEntry { field } = &mut self.stage
                    && index < 3
                {
                    *field = index;
                }
                Effect::None
            }
        }
    }

    /// The answer of the discovery or the setup the wizard waits for:
    /// whether Bedrock is configured. A failed discovery shows every method
    /// with its message; a refused setup returns to the region entry.
    pub fn answer(&mut self, request: Request, outcome: super::Outcome) -> bool {
        match (&self.stage, outcome) {
            (Stage::Discovering(r), outcome) if *r == request => {
                match outcome {
                    Ok(Answer::Discovered(discovery)) => *self = Bedrock::discovered(discovery),
                    // Not the answer of a discovery: nothing found.
                    Ok(_) => *self = Bedrock::discovered(Discovery::default()),
                    Err(message) => {
                        *self = Bedrock::discovered(Discovery::default());
                        self.error = Some(Error::Discover(message));
                    }
                }
                false
            }
            (
                Stage::Configuring {
                    request: r,
                    credential,
                    region,
                },
                outcome,
            ) if *r == request => match outcome {
                Ok(_) => true,
                Err(message) => {
                    let (credential, region) = (credential.clone(), region.clone());
                    self.enter_region(credential, &region);
                    self.error = Some(Error::Setup(message));
                    false
                }
            },
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                vec![Method::Profile(0), Method::Environment, Method::OtherMethods],
            ),
            (
                vec![engineering.clone(), profile("production", Some("us-west-2"))],
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
}
