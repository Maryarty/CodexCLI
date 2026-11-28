//! A model that lives on its own: the fixture a seeded screen runs over. It
//! takes events and intents like the app's model (through `CodexApp::ingest`)
//! and drops the effects they ask for, because it has no backend. Views name
//! this type, never `ingest`, `Ingress` or the script player.

use crate::app::AppEv;
use crate::app::CodexApp;
use crate::app::Ingress;
use crate::data::history::Lookup;
use crate::app::effects::Ask;
use crate::screens::ScreenEv;
use crate::screens::popovers;
use crate::fixtures::script::ScriptError;
use crate::fixtures::script::build;

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
    pub fn send(&mut self, ev: AppEv) {
        let mut queue = std::collections::VecDeque::from([ev]);
        while let Some(ev) = queue.pop_front() {
            let epoch = self.app.epoch();
            let Some(fx) = self.app.ingest(Ingress::new(epoch, ev)) else {
                continue;
            };
            for command in fx.commands() {
                if let Some(Ask::Popovers {
                    ask: popovers::Ask::Lookup(lookup),
                    ..
                }) = command.host_ask()
                {
                    queue.push_back(AppEv::Screen(ScreenEv::Popovers(failed(lookup))));
                }
            }
        }
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
