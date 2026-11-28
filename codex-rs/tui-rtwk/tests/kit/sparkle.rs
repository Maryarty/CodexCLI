//! The starfield of the Astra sparkle and the help-over-composer layout, as the footer shards
//! assert them: dots lit or gone, a fade that dims, the composer rows against the oracle's with
//! the dots masked, the dots against the oracle's within the clock's slack.

use super::Screen;

/// The braille starfield dots of the oracle's sparkle (`sparkle_field.rs`, `DOTS`).
const DOT_CHARS: [char; 8] = ['⠁', '⠂', '⠄', '⠈', '⠐', '⠠', '⡀', '⢀'];

/// The dots lit on a screen: `(x, y, glyph, contrast)`, the contrast being the distance of the
/// foreground from the cell's true-colour background (what the fade takes away).
pub struct Dots(Vec<(usize, usize, String, u32)>);

/// The composer's three rows (the blank row above the prompt, the prompt and the blank row below
/// it), trailing blanks cut: where the sparkle's dots are.
fn composer_rows(rows: &[String]) -> Vec<String> {
    let prompt = rows
        .iter()
        .position(|row| row.contains("Ask Codex") || row.starts_with('›'))
        .unwrap_or(usize::MAX);
    rows[prompt.saturating_sub(1)..=(prompt + 1).min(rows.len() - 1)]
        .iter()
        .map(|row| row.trim_end().to_string())
        .collect()
}

/// The composer's rows of the oracle's snapshot `name` after its row `marker` (`""`: from the
/// start): the snapshots of these tests join several screens, each under a label row.
fn oracle_composer(name: &str, marker: &str) -> Vec<String> {
    let rows: Vec<String> = Screen::snap_frame_names(name)
        .into_iter()
        .flat_map(|frame| {
            let rows = super::snap_rows(name, &frame);
            (!frame.is_empty())
                .then(|| frame.clone())
                .into_iter()
                .chain(rows)
        })
        .collect();
    // `marker`: the label rows to find one after the other (`\n` between them), for a snapshot
    // that repeats its labels
    let mut from = 0;
    for part in marker.split('\n').filter(|part| !part.is_empty()) {
        from += rows[from..]
            .iter()
            .position(|row| row.trim_end() == part)
            .map_or_else(|| panic!("{name}: no row `{part}`"), |at| at + 1);
    }
    composer_rows(&rows[from..])
}

fn bare(rows: &[String]) -> Vec<String> {
    rows.iter()
        .map(|row| {
            row.chars()
                .map(|c| if DOT_CHARS.contains(&c) { ' ' } else { c })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

impl Screen {
    /// The dots lit on the screen now.
    pub fn dots(&self) -> Dots {
        let channels = |name: &str| -> [u32; 3] {
            assert!(
                name.starts_with("Rgb(") && name.ends_with(')'),
                "a true colour, not `{name}`"
            );
            let mut out = [0u32; 3];
            for (slot, part) in out.iter_mut().zip(name[4..name.len() - 1].split(", ")) {
                *slot = part.parse().unwrap_or(u32::MAX);
            }
            out
        };
        let grid = self.grid();
        let mut out = Vec::new();
        for (y, row) in self.rows().iter().enumerate() {
            for (x, glyph) in row.chars().enumerate() {
                if DOT_CHARS.contains(&glyph) {
                    let style = grid.style(x, y);
                    let (fg, bg) = (channels(&style.fg), channels(&style.bg));
                    let contrast = fg.iter().zip(bg).map(|(a, b)| a.abs_diff(b)).sum();
                    out.push((x, y, glyph.to_string(), contrast));
                }
            }
        }
        Dots(out)
    }

    /// No starfield dot is on the screen.
    pub fn assert_no_dots(&self) {
        let dots = self.dots().0;
        assert!(dots.is_empty(), "dots {dots:?} on:\n{}", self.text());
    }

    /// Dots are lit exactly when `lit` (`why` names the case).
    pub fn assert_dots_lit(&self, lit: bool, why: &str) {
        let dots = self.dots().0;
        assert_eq!(
            !dots.is_empty(),
            lit,
            "{why}: dots {dots:?}\n{}",
            self.text()
        );
    }

    /// The dots lit now fade from `before`: some are lit, and every one that was lit there at
    /// the same cell is dimmer.
    pub fn assert_dots_fade_from(&self, before: &Dots) {
        let fading = self.dots().0;
        assert!(!fading.is_empty(), "the fading frame has dots");
        for (x, y, glyph, contrast) in &fading {
            if let Some((_, _, _, was)) = before
                .0
                .iter()
                .find(|d| d.0 == *x && d.1 == *y && d.2 == *glyph)
            {
                assert!(
                    contrast < was,
                    "the dot at ({x}, {y}) fades: {contrast} against {was}"
                );
            }
        }
    }

    /// The draft is empty, whether or not a star sits in the prompt row's gap (`›⠁Ask Codex ..`):
    /// the placeholder shows exactly while the draft is empty.
    pub fn assert_empty_draft(&self) {
        let placeholder = self
            .rows()
            .iter()
            .any(|row| row.starts_with('›') && row.contains("Ask Codex to do anything"));
        if !placeholder {
            self.assert_composer("");
        }
    }

    /// No dot sits in the first 22 cells of the draft in the prompt row.
    pub fn assert_no_dots_in_draft(&self) {
        let rows = self.rows();
        let prompt = rows
            .iter()
            .position(|row| row.starts_with('›'))
            .expect("the prompt row");
        let draft: String = rows[prompt].chars().skip(2).take(22).collect();
        assert!(
            !draft.chars().any(|c| DOT_CHARS.contains(&c)),
            "a dot in `{draft}`"
        );
    }

    /// The composer's rows are the oracle's off the dots (`marker`: the label row of the
    /// snapshot they follow): for a frame the oracle draws at an instant the test's keys cannot
    /// reach with the clock.
    pub fn assert_layout(&self, name: &str, marker: &str) -> (Vec<String>, Vec<String>) {
        let want = oracle_composer(name, marker);
        let got = composer_rows(&self.rows());
        let (bare_want, bare_got) = (bare(&want), bare(&got));
        super::verdict::screen(&bare_got, &bare_want, || {
            pretty_assertions::assert_eq!(
                bare_want.join("\n"),
                bare_got.join("\n"),
                "{name} / {marker}: ours (right) differs from the oracle (left) off the dots\n{}",
                self.text()
            );
        });
        (want, got)
    }

    /// The dots of the screen are the oracle's (the composer's rows after the label row
    /// `marker`): everything but the dots is the same cell for cell. The oracle draws its
    /// snapshots at an exact instant, the screen at the last tick of the app's 150 ms clock
    /// before the capture, so a dot at the edge of its twinkle may have come or gone: at most a
    /// fifth of the dots may differ, and any that is in both is the same glyph.
    pub fn assert_stars(&self, name: &str, marker: &str) {
        let (want, got) = self.assert_layout(name, marker);
        let cells = |rows: &[String]| -> Vec<(usize, usize, char)> {
            rows.iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.chars()
                        .enumerate()
                        .filter(|(_, c)| DOT_CHARS.contains(c))
                        .map(move |(x, c)| (x, y, c))
                })
                .collect()
        };
        let (want_dots, got_dots) = (cells(&want), cells(&got));
        let differing = want_dots
            .iter()
            .filter(|d| !got_dots.contains(d))
            .chain(got_dots.iter().filter(|d| !want_dots.contains(d)))
            .count();
        assert!(
            differing * 5 <= want_dots.len() + got_dots.len(),
            "{name} / {marker}: {differing} dots differ of {} and {}\nwant {:?}\ngot  {:?}",
            want_dots.len(),
            got_dots.len(),
            want_dots,
            got_dots
        );
    }

    /// The help card of the snapshot `name` (one frame) is ours from its first row down.
    pub fn assert_help_snap(&self, name: &str) {
        self.assert_help_rows(&super::snap_rows(name, ""), name);
    }

    /// Our help card (from `Keyboard shortcuts` down, trailing blanks cut, the gap row above the
    /// box removed) is the oracle's `oracle` rows, the prompt row aside: it carries the page's own
    /// placeholder, so it must start with the oracle's.
    pub fn assert_help_rows(&self, oracle: &[String], name: &str) {
        let at = |rows: &[String], text: &str| {
            rows.iter()
                .position(|row| row.contains(text))
                .unwrap_or_else(|| panic!("no row with `{text}`"))
        };
        let ours = self.rows();
        let (o_help, o_prompt) = (at(oracle, "Keyboard shortcuts"), at(oracle, "Ask Codex"));
        let (help, prompt) = (at(&ours, "Keyboard shortcuts"), at(&ours, "Ask Codex"));
        let mut got: Vec<String> = ours[help..]
            .iter()
            .map(|row| row.trim_end().to_string())
            .collect();
        // customize, the gap, the box's blank row, the prompt: the gap is two rows above the prompt
        got.remove(prompt - help - 2);
        let want = &oracle[o_help..];
        let at_prompt = o_prompt - o_help;
        assert!(
            got[at_prompt].starts_with(&want[at_prompt]),
            "{name}: the prompt row `{}` does not start with the oracle's `{}`",
            got[at_prompt],
            want[at_prompt]
        );
        got[at_prompt] = want[at_prompt].clone();
        super::verdict::screen(&got, want, || {
            pretty_assertions::assert_eq!(
                want.join("\n"),
                got.join("\n"),
                "{name}: ours (right) differs from the oracle (left)"
            );
        });
    }

    /// The cursor is on the final row of the screen.
    pub fn assert_cursor_on_last_row(&self) {
        let (_, row) = self.grid().cursor.expect("the cursor is shown");
        assert_eq!(
            row,
            self.rows().len() - 1,
            "the cursor is on the final row:\n{}",
            self.text()
        );
    }
}

/// The frames of a snapshot's rows, which the oracle joined with `---` rows: there are `n`.
pub fn split_frames(rows: &[String], n: usize) -> Vec<Vec<String>> {
    let frames: Vec<Vec<String>> = rows.split(|row| row == "---").map(<[_]>::to_vec).collect();
    assert_eq!(frames.len(), n, "the snapshot holds {n} renders");
    frames
}
