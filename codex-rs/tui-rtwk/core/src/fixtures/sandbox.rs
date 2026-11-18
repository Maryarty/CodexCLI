//! A model that lives on its own: the fixture a seeded screen runs over. It
//! takes events and intents like the app's model (through `CodexApp::ingest`)
//! and drops the effects they ask for, because it has no backend. Views name
//! this type, never `ingest`, `Ingress` or the script player.

use crate::app::AppEv;
use crate::app::CodexApp;
use crate::fixtures::script::ScriptError;

pub struct Sandbox {
    app: CodexApp,
}

impl Sandbox {
    /// The model a script makes; its `key` and `resize` inputs are dropped
    /// (a tester plays those, through `script::build`).
    pub fn play(text: &str) -> Result<Sandbox, ScriptError> {
        let _ = text;
        todo!("play")
    }

    pub fn app(&self) -> &CodexApp {
        &self.app
    }

    /// Applies `ev` under the model's epoch; the effects are dropped.
    pub fn send(&mut self, ev: AppEv) {
        let _ = ev;
        todo!("send")
    }
}
