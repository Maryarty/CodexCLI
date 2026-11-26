//! The server's answer to the background terminals a resumed thread asks for
//! (`thread/backgroundTerminals/list`, [`Open::replay`](super::Open::replay)): the request
//! stays open until a test answers it, so a test can end one of the commands while the list is
//! in flight (the oracle calls `begin_background_terminal_restore` and `restore_background_terminals`
//! by hand around it).

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::screens::{ScreenCommand, chat};

use super::Screen;

impl Screen {
    /// How many terminal list requests the app has made (a resumed thread asks once, the next
    /// page of a long list once more).
    pub fn terminal_lists_asked(&self) -> usize {
        self.h
            .fake
            .commands()
            .iter()
            .filter(|command| is_list(command))
            .count()
    }

    /// The server answers the newest terminal list request with the terminals `(item id,
    /// process id, command)`, the last page of the list.
    pub fn terminals_listed(&mut self, listed: &[(&str, &str, &str)]) -> &mut Screen {
        let data: Vec<_> = listed
            .iter()
            .map(|(item, process, command)| {
                serde_json::json!({
                    "itemId": item, "processId": process, "command": command, "cwd": "/work",
                    "osPid": null, "cpuPercent": null, "rssKb": null
                })
            })
            .collect();
        self.answer_terminals(&serde_json::json!({ "data": data, "nextCursor": null }))
    }

    fn answer_terminals(&mut self, page: &serde_json::Value) -> &mut Screen {
        // a list the app has not asked for yet (the thread was just opened) is waited for a little
        for _ in 0..20 {
            if self.terminal_lists_asked() > 0 {
                break;
            }
            self.h.advance(100);
        }
        let command = self
            .h
            .fake
            .commands()
            .into_iter()
            .rfind(is_list)
            .expect("no terminal list request");
        let (ticket, reply) = codex_tui_core::protocol::reply(&command, Ok(page.clone()))
            .expect("a command with a reply");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(epoch, AppEv::Reply(ticket, reply));
        self.h.advance(100);
        self
    }
}

fn is_list(command: &Command) -> bool {
    matches!(
        command,
        Command::Screen {
            command: ScreenCommand::Chat(chat::Command::ListTerminals { .. }),
            ..
        }
    )
}
