//! The startup prompts' key bindings (the oracle's `handle_key` of each
//! prompt).
//!
//! In Flutter this is the `Shortcuts` map of the page: key -> intent, with
//! the `Actions` answering them. Here a key is bound to a [`Cmd`]; the panel
//! turns the command into what the prompt does with it ([`Cmd::react`], over
//! core's `Prompt::{step, number, take}`) and core's `apply` validates the
//! answer (ADR-003: the view picks, core decides).

use rtwk::focus::KeyPress;
use rtwk::prelude::Mods;

use codex_tui_core::screens::startup::{
    Answer, CwdSelection, HooksChoice, MigrationOutcome, Prompt, Reaction, UnarchiveChoice,
};

/// What a key asks of the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// The cursor one row on (1) or back (-1), round the ends.
    Step(i64),
    /// A number key (1-based, over the rows that are not disabled).
    Number(usize),
    /// Take this row (a hotkey).
    Take(usize),
    /// Take the row the cursor stands on (Enter).
    TakeCursor,
    /// The cursor goes to this row.
    Cursor(usize),
    /// The prompt returns this answer (Ctrl-C, Esc).
    Answer(Answer),
}

impl Cmd {
    /// What the command does to `prompt` with the cursor on `cursor`.
    pub fn react(self, prompt: &Prompt, cursor: usize) -> Reaction {
        match self {
            Cmd::Step(delta) => prompt.step(cursor, delta),
            Cmd::Number(n) => prompt.number(n),
            Cmd::Take(row) => prompt.take(row),
            Cmd::TakeCursor => prompt.take(cursor),
            Cmd::Cursor(row) => Reaction::Cursor(row),
            Cmd::Answer(answer) => Reaction::Answer(answer),
        }
    }
}

fn ctrl(c: char) -> KeyPress {
    KeyPress::char(c).with_mods(Mods::CTRL)
}

fn ch(c: char) -> KeyPress {
    KeyPress::char(c)
}

/// Ctrl-H, which a terminal sends as Ctrl-Backspace.
fn ctrl_h() -> KeyPress {
    KeyPress::BACKSPACE.with_mods(Mods::CTRL)
}

fn digits(range: std::ops::RangeInclusive<usize>) -> Vec<(KeyPress, Cmd)> {
    range
        .filter_map(|n| {
            let c = char::from_digit(u32::try_from(n).ok()?, 10)?;
            Some((ch(c), Cmd::Number(n)))
        })
        .collect()
}

/// The keys `prompt` reads, as it stands now (the panel rebuilds them with
/// the prompt, so a prompt that stops reading keys, the hooks review while
/// the trust write runs, simply has none).
pub fn bindings(prompt: &Prompt) -> Vec<(KeyPress, Cmd)> {
    let quit = |answer| {
        vec![
            (ctrl('c'), Cmd::Answer(answer)),
            (ctrl('d'), Cmd::Answer(answer)),
        ]
    };
    let up_down = [
        (KeyPress::UP, Cmd::Step(-1)),
        (ch('k'), Cmd::Step(-1)),
        (KeyPress::DOWN, Cmd::Step(1)),
        (ch('j'), Cmd::Step(1)),
    ];
    let mut keys = Vec::new();
    match prompt {
        // The guidance's own widget reads its keys.
        Prompt::GovCloud => {}
        Prompt::Cwd(_) => {
            keys.extend(quit(Answer::CwdExit));
            keys.extend(up_down);
            keys.extend(digits(1..=4));
            keys.push((KeyPress::ENTER, Cmd::TakeCursor));
            keys.push((
                KeyPress::ESC,
                Cmd::Answer(Answer::Cwd(CwdSelection::Session)),
            ));
        }
        Prompt::Migration(p) => {
            keys.extend(quit(Answer::Migration(MigrationOutcome::Exit)));
            if p.can_opt_out {
                keys.extend([
                    (KeyPress::UP, Cmd::Cursor(0)),
                    (ch('k'), Cmd::Cursor(0)),
                    (KeyPress::DOWN, Cmd::Cursor(1)),
                    (ch('j'), Cmd::Cursor(1)),
                ]);
                keys.extend(digits(1..=2));
                keys.push((KeyPress::ENTER, Cmd::TakeCursor));
                keys.push((KeyPress::ESC, Cmd::TakeCursor));
            } else {
                let accept = Cmd::Answer(Answer::Migration(MigrationOutcome::Accepted));
                keys.push((KeyPress::ENTER, accept));
                keys.push((KeyPress::ESC, accept));
            }
        }
        Prompt::Hooks(_) => {
            if prompt.rows().iter().all(|row| row.disabled) {
                return keys;
            }
            keys.extend(up_down);
            keys.extend(digits(1..=9));
            keys.push((KeyPress::ENTER, Cmd::TakeCursor));
            keys.push((KeyPress::ESC, Cmd::Answer(Answer::Hooks(HooksChoice::Skip))));
        }
        Prompt::Unarchive(_) => {
            keys.extend(quit(Answer::Unarchive(UnarchiveChoice::Quit)));
            keys.extend([
                (KeyPress::UP, Cmd::Step(1)),
                (KeyPress::DOWN, Cmd::Step(1)),
                (ch('k'), Cmd::Step(1)),
                (ch('j'), Cmd::Step(1)),
            ]);
            keys.extend(
                [('1', 0), ('y', 0), ('Y', 0), ('2', 1), ('n', 1), ('N', 1)]
                    .map(|(c, row)| (ch(c), Cmd::Take(row))),
            );
            keys.push((KeyPress::ESC, Cmd::Take(1)));
            keys.push((KeyPress::ENTER, Cmd::TakeCursor));
        }
        Prompt::Oss(_) => {
            keys.push((ctrl('c'), Cmd::Answer(Answer::OssCancelled)));
            keys.extend([
                (KeyPress::LEFT, Cmd::Step(-1)),
                (KeyPress::UP, Cmd::Step(-1)),
                (ctrl_h(), Cmd::Step(-1)),
                (ch('k'), Cmd::Step(-1)),
                (KeyPress::RIGHT, Cmd::Step(1)),
                (KeyPress::DOWN, Cmd::Step(1)),
                (ctrl('l'), Cmd::Step(1)),
                (ch('j'), Cmd::Step(1)),
            ]);
            keys.push((KeyPress::ENTER, Cmd::TakeCursor));
            keys.push((KeyPress::ESC, Cmd::Take(0)));
            keys.extend(
                [('1', 0), ('l', 0), ('L', 0), ('2', 1), ('o', 1), ('O', 1)]
                    .map(|(c, row)| (ch(c), Cmd::Take(row))),
            );
        }
        Prompt::Daemon(_) => {
            keys.extend(quit(Answer::DaemonQuit));
            keys.extend(up_down);
            keys.extend(digits(1..=9));
            keys.push((KeyPress::ENTER, Cmd::TakeCursor));
            keys.push((KeyPress::ESC, Cmd::Take(2)));
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::screens::startup::{
        Action, CwdPrompt, DaemonChoice, DaemonPrompt, HooksPrompt, MigrationPrompt, OssPrompt,
        Provider, Status, Trust, UnarchivePrompt,
    };

    fn up() -> KeyPress {
        KeyPress::UP
    }
    fn down() -> KeyPress {
        KeyPress::DOWN
    }
    fn enter() -> KeyPress {
        KeyPress::ENTER
    }
    fn esc() -> KeyPress {
        KeyPress::ESC
    }

    fn react(prompt: &Prompt, cursor: usize, key: &KeyPress) -> Reaction {
        bindings(prompt)
            .into_iter()
            .find(|(press, _)| press == key)
            .map_or(Reaction::Nothing, |(_, cmd)| cmd.react(prompt, cursor))
    }

    /// Plays keys the way the panel does: the cursor follows `Cursor`, the
    /// first answer ends it.
    fn play(prompt: &Prompt, keys: &[KeyPress]) -> (usize, Option<Reaction>) {
        let mut cursor = prompt.initial_cursor();
        for key in keys {
            match react(prompt, cursor, key) {
                Reaction::Nothing => {}
                Reaction::Cursor(row) => cursor = row,
                other => return (cursor, Some(other)),
            }
        }
        (cursor, None)
    }

    fn answer(answer: Answer) -> Option<Reaction> {
        Some(Reaction::Answer(answer))
    }

    fn cwd(allow: bool) -> Prompt {
        Prompt::Cwd(CwdPrompt {
            action: Action::Resume,
            current: "/Users/example/current".into(),
            session: "/Users/example/session".into(),
            remembered_current: "/Users/example/launched".into(),
            allow_remember_current: allow,
        })
    }

    #[test]
    fn cwd_prompt_selects_session_by_default() {
        assert_eq!(
            play(&cwd(true), &[enter()]).1,
            answer(Answer::Cwd(CwdSelection::Session))
        );
    }

    #[test]
    fn cwd_prompt_can_select_current() {
        assert_eq!(
            play(&cwd(true), &[down(), enter()]).1,
            answer(Answer::Cwd(CwdSelection::Current))
        );
    }

    #[test]
    fn cwd_prompt_omits_unusable_remembered_current_choice() {
        let prompt = cwd(false);
        assert_eq!(play(&prompt, &[ch('4')]), (0, None));
        assert_eq!(play(&prompt, &[up()]), (2, None));
    }

    #[test]
    fn cwd_prompt_ctrl_c_exits_instead_of_selecting() {
        assert_eq!(play(&cwd(true), &[ctrl('c')]).1, answer(Answer::CwdExit));
    }

    #[test]
    fn cwd_prompt_remembered_choices_select_matching_directory() {
        assert_eq!(
            play(&cwd(true), &[ch('3')]).1,
            answer(Answer::Cwd(CwdSelection::SessionAndRemember))
        );
        assert_eq!(
            play(&cwd(true), &[ch('4')]).1,
            answer(Answer::Cwd(CwdSelection::CurrentAndRemember))
        );
    }

    #[test]
    fn narrow_picker_escape_uses_session() {
        assert_eq!(
            play(&cwd(true), &[up(), esc()]).1,
            answer(Answer::Cwd(CwdSelection::Session))
        );
        assert_eq!(play(&cwd(true), &[up()]), (3, None));
    }

    fn migration(can_opt_out: bool) -> Prompt {
        Prompt::Migration(MigrationPrompt {
            current_model: "gpt-old".into(),
            target_model: "gpt-new".into(),
            target_display_name: "gpt-new".into(),
            target_description: None,
            copy: None,
            link: None,
            markdown: None,
            can_opt_out,
            effort: None,
        })
    }

    #[test]
    fn escape_key_accepts_prompt() {
        assert_eq!(
            play(&migration(true), &[esc()]).1,
            answer(Answer::Migration(MigrationOutcome::Accepted))
        );
        assert_eq!(
            play(&migration(false), &[esc()]).1,
            answer(Answer::Migration(MigrationOutcome::Accepted))
        );
    }

    #[test]
    fn selecting_use_existing_model_rejects_upgrade() {
        assert_eq!(
            play(&migration(true), &[down(), enter()]).1,
            answer(Answer::Migration(MigrationOutcome::Rejected))
        );
        assert_eq!(
            play(&migration(true), &[ctrl('c')]).1,
            answer(Answer::Migration(MigrationOutcome::Exit))
        );
    }

    #[test]
    fn a_migration_without_opt_out_reads_only_accept_and_quit() {
        let prompt = migration(false);
        assert_eq!(play(&prompt, &[down(), ch('2'), ch('j')]), (0, None));
        assert_eq!(
            play(&prompt, &[enter()]).1,
            answer(Answer::Migration(MigrationOutcome::Accepted))
        );
    }

    fn hooks_idle() -> Prompt {
        Prompt::Hooks(HooksPrompt {
            count: 2,
            trust: Trust::Idle,
            updates: Vec::new(),
        })
    }

    #[test]
    fn fragmented_terminal_response_cannot_grant_startup_hook_trust() {
        let prompt = hooks_idle();
        let mut cursor = 0;
        for c in "20;rgb:2222/ffff/ffff".chars() {
            let reaction = react(&prompt, cursor, &ch(c));
            assert_ne!(reaction, Reaction::TrustAll);
            if let Reaction::Cursor(row) = reaction {
                cursor = row;
            }
        }
        assert_eq!(cursor, 1);
        assert_eq!(react(&prompt, cursor, &enter()), Reaction::TrustAll);
    }

    #[test]
    fn non_trust_startup_hook_shortcuts_remain_immediate() {
        assert_eq!(
            play(&hooks_idle(), &[ch('1')]).1,
            answer(Answer::Hooks(HooksChoice::Review))
        );
        assert_eq!(
            play(&hooks_idle(), &[ch('3')]).1,
            answer(Answer::Hooks(HooksChoice::Skip))
        );
    }

    #[test]
    fn picker_progress_blocks_selection() {
        let prompt = Prompt::Hooks(HooksPrompt {
            count: 2,
            trust: Trust::Trusting(1),
            updates: Vec::new(),
        });
        assert!(prompt.rows().iter().all(|row| row.disabled));
        assert!(bindings(&prompt).is_empty());
        assert_eq!(play(&prompt, &[enter(), ch('1'), esc()]), (0, None));
    }

    #[test]
    fn confirmation_requires_an_accept_key() {
        let prompt = Prompt::Unarchive(UnarchivePrompt {
            thread: "t".into(),
            action: Action::Fork,
        });
        let cancel = answer(Answer::Unarchive(UnarchiveChoice::Cancel));
        for key in [esc(), ch('n'), ch('2')] {
            assert_eq!(play(&prompt, &[key]).1, cancel);
        }
        for c in ['c', 'd'] {
            assert_eq!(
                play(&prompt, &[ctrl(c)]).1,
                answer(Answer::Unarchive(UnarchiveChoice::Quit))
            );
        }
        assert_eq!(
            play(&prompt, &[enter()]).1,
            answer(Answer::Unarchive(UnarchiveChoice::Unarchive))
        );
        assert_eq!(play(&prompt, &[down(), enter()]).1, cancel);
    }

    fn oss() -> Prompt {
        Prompt::Oss(OssPrompt {
            lmstudio: Status::Unknown,
            ollama: Status::Unknown,
        })
    }

    #[test]
    fn ctrl_h_l_move_provider_selection() {
        assert_eq!(play(&oss(), &[ctrl('l')]), (1, None));
        assert_eq!(play(&oss(), &[ctrl('l'), ctrl_h()]), (0, None));
    }

    #[test]
    fn navigation_wraps_without_confirming() {
        assert_eq!(play(&oss(), &[up()]), (1, None));
        assert_eq!(play(&oss(), &[down(), down()]), (0, None));
    }

    #[test]
    fn shortcuts_select_the_provider_even_when_unavailable() {
        for (key, provider) in [
            (ch('l'), Provider::LmStudio),
            (ch('O'), Provider::Ollama),
            (ch('2'), Provider::Ollama),
            (esc(), Provider::LmStudio),
        ] {
            assert_eq!(play(&oss(), &[key]).1, answer(Answer::Oss(provider)));
        }
        assert_eq!(play(&oss(), &[ctrl('c')]).1, answer(Answer::OssCancelled));
    }

    fn daemon(managed: bool, restartable: bool) -> Prompt {
        Prompt::Daemon(DaemonPrompt {
            reason: "r".into(),
            restart_features: restartable.then(|| vec![("a".into(), true)]),
            managed,
        })
    }

    #[test]
    fn daemon_recovery_requires_explicit_restart_and_defaults_to_cancel() {
        for (managed, restartable) in [(true, true), (false, true), (true, false)] {
            let prompt = daemon(managed, restartable);
            assert_eq!(
                play(&prompt, &[enter()]).1,
                answer(Answer::Daemon(DaemonChoice::Cancel))
            );
            if managed && restartable {
                assert_eq!(play(&prompt, &[ch('2')]), (1, None));
                assert_eq!(
                    play(&prompt, &[ch('2'), enter()]).1,
                    answer(Answer::Daemon(DaemonChoice::Restart))
                );
            } else {
                // The disabled row has no number: `2` is Cancel.
                assert_eq!(
                    play(&prompt, &[ch('2')]).1,
                    answer(Answer::Daemon(DaemonChoice::Cancel))
                );
                assert_eq!(play(&prompt, &[up()]), (0, None));
            }
        }
    }
}
