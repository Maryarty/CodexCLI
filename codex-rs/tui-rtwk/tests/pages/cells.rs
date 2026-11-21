//! Screen tests of the `cells` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/cells/`.

use crate::common::slot;

#[test]
fn cells_slot_is_registered_and_empty() {
    slot("cells");
}

// ---- the patch cell's disclosure (oracle transcript_view/text.rs
// `with_control`: an activity control sits a quarter of the width in, at most
// four columns; the oracle's pty capture of the patch-approval scenario reads
// `    + Show details` at 120 columns) ----

mod patch_disclosure {
    use std::sync::Arc;

    use codex_tui_core::threads::{PatchActivity, PatchChange, PatchKind, ToolStatus};
    use codex_tui_rtwk::pages::session::cells::patch::PatchCell;

    fn cell() -> PatchCell {
        PatchCell {
            activity: Arc::new(PatchActivity {
                changes: vec![PatchChange {
                    path: "notes.txt".into(),
                    kind: PatchKind::Add,
                    diff: "one\ntwo\nthree\nfour\n".into(),
                }],
                status: ToolStatus::Completed,
            }),
            cwd: None,
            detailed: false,
        }
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    #[test]
    fn patch_disclosure_control_is_indented_like_the_oracle() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(120, 8));
        t.pump_widget(cell());
        let shown = rows(&t);
        assert!(
            shown.iter().any(|row| row == "    + Show details"),
            "{shown:#?}"
        );

        let mut narrow = rtwk_test::Tester::with(rtwk_test::Options::new().size(12, 8));
        narrow.pump_widget(cell());
        let shown = rows(&narrow);
        assert!(
            shown.iter().any(|row| row.starts_with("   + Show")),
            "a quarter of 12 columns: {shown:#?}"
        );
    }

    #[test]
    fn tapping_the_patch_disclosure_reveals_every_line() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(120, 8));
        t.pump_widget(cell());
        assert!(!rows(&t).iter().any(|row| row.contains("4 +four")));
        let y = rows(&t)
            .iter()
            .position(|row| row.contains("+ Show details"))
            .expect("the control");
        t.click(6, y as u16);
        t.pump();
        let shown = rows(&t);
        assert!(
            shown.iter().any(|row| row == "    − Hide details"),
            "{shown:#?}"
        );
        assert!(
            shown.iter().any(|row| row.contains("4 +four")),
            "{shown:#?}"
        );
    }
}
