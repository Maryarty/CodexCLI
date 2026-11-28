//! A model that lives on its own: the fixture a seeded screen runs over. It
//! takes events and intents like the app's model (through `CodexApp::ingest`)
//! and drops the effects they ask for, because it has no backend. Views name
//! this type, never `ingest`, `Ingress` or the script player.

use crate::app::AppEv;
use crate::app::CodexApp;
use crate::app::Ingress;
use crate::app::effects::Ask;
use crate::data::history::Lookup;
use crate::fixtures::script::ScriptError;
use crate::fixtures::script::build;
use crate::screens::ScreenEv;
use crate::screens::popovers;

pub struct Sandbox {
    app: CodexApp,
}

impl Sandbox {
    /// The model a script makes; its `key` and `resize` inputs are dropped
    /// (a tester plays those, through `script::build`).
    pub fn play(text: &str) -> Result<Sandbox, ScriptError> {
        Ok(Sandbox {
            app: build(text)?.app,
        })
    }

    pub fn app(&self) -> &CodexApp {
        &self.app
    }

    /// Applies `ev` under the model's epoch; the effects are dropped, except that a read of the
    /// message history file fails (the file is not there to read: the search ends "unavailable").
    /// The failures are answered once: an answer asks for nothing (a read starts from an intent
    /// of the page, never from an answer), so its own effects are dropped like any other.
    pub fn send(&mut self, ev: AppEv) {
        for answer in self.apply(ev) {
            drop(self.apply(answer));
        }
    }

    /// One ingest; the answers to the history reads it asked for.
    fn apply(&mut self, ev: AppEv) -> Vec<AppEv> {
        let epoch = self.app.epoch();
        let Some(fx) = self.app.ingest(Ingress::new(epoch, ev)) else {
            return Vec::new();
        };
        let mut answers = Vec::new();
        for command in fx.commands() {
            if let Some(Ask::Popovers {
                ask: popovers::Ask::Lookup(lookup),
                ..
            }) = command.host_ask()
            {
                answers.push(AppEv::Screen(ScreenEv::Popovers(failed(lookup))));
            }
        }
        answers
    }
}

fn failed(lookup: &Lookup) -> popovers::Ev {
    match *lookup {
        Lookup::Entry { log_id, offset } => popovers::Ev::HistoryEntry {
            log_id,
            offset,
            text: None,
        },
        Lookup::Batch { log_id, end_offset } => {
            popovers::Ev::HistoryBatchFailed { log_id, end_offset }
        }
    }
}
