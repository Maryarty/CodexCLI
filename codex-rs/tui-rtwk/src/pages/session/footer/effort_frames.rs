//! The frames of the status line's transition (oracle `effort_status_line.rs`
//! `transition_line_at`, `tier_label_line`, `truncate_line_to_width`): pure
//! functions of the elapsed time that return the spans of one row. The oracle
//! computes the same row at each frame; rtwk's `Text` shows it. The width
//! arithmetic is the oracle's own (offsets and gaps in cells), measured by the
//! tree's segmenter.

use std::time::Duration;

use rtwk::style::{Attrs, Span, Style};

use crate::app::theme::effort::{Colors, Grade, fade};

pub(crate) const SCROLL_OUT: Duration = Duration::from_millis(620);
const ASSEMBLE: Duration = Duration::from_millis(700);
const HOLD: Duration = Duration::from_millis(360);
const FADE_OUT: Duration = Duration::from_millis(340);
const FADE_IN: Duration = Duration::from_millis(480);
/// The whole run: 2.5 s.
pub(crate) const TRANSITION: Duration = Duration::from_millis(
    (SCROLL_OUT.as_millis()
        + ASSEMBLE.as_millis()
        + HOLD.as_millis()
        + FADE_OUT.as_millis()
        + FADE_IN.as_millis()) as u64,
);

/// What a frame is made with besides the lines: the tier, its letters, the
/// terminal's colours and the tree's measure of text.
#[derive(Clone, Copy)]
pub(crate) struct Ink<'a> {
    pub grade: Grade,
    pub label: &'a str,
    pub colors: Colors,
    pub measure: &'a dyn Fn(&str) -> usize,
}

fn ease_in_cubic(progress: f32) -> f32 {
    progress.clamp(0.0, 1.0).powi(3)
}

fn ease_out_cubic(progress: f32) -> f32 {
    1.0 - (1.0 - progress.clamp(0.0, 1.0)).powi(3)
}

fn blank(cells: usize) -> Span {
    let text = " ".repeat(cells);
    Span::new(text)
}

fn fade_spans(spans: &mut [Span], ink: &Ink<'_>, opacity: f32, tint: f32) {
    for span in spans {
        fade(&mut span.style, ink.grade, opacity, tint, ink.colors);
    }
}

/// `truncate_line_to_width`: what fits `width` cells of `spans`.
pub(crate) fn clip(spans: Vec<Span>, width: usize, measure: &dyn Fn(&str) -> usize) -> Vec<Span> {
    let mut left = width;
    let mut out = Vec::new();
    for span in spans {
        if left == 0 {
            break;
        }
        let mut kept = String::new();
        for ch in span.text.chars() {
            let cells = measure(&ch.to_string());
            if cells > left {
                left = 0;
                break;
            }
            left -= cells;
            kept.push(ch);
        }
        if !kept.is_empty() {
            out.push(Span::styled(kept, span.style));
        }
    }
    out
}

/// `tier_label_line`: the tier's letters across `width`, the gaps wider toward
/// the edges while `assemble` is below 1.
pub(crate) fn label_line(ink: &Ink<'_>, width: usize, assemble: f32, opacity: f32) -> Vec<Span> {
    let letters: Vec<char> = ink.label.chars().collect();
    let gap_count = letters.len().saturating_sub(1);
    let spread = width.saturating_sub(letters.len() + gap_count) as f32;
    let spread = (spread * (1.0 - assemble.clamp(0.0, 1.0))).round() as usize;
    let weights: Vec<usize> = (0..gap_count)
        .map(|index| (index * 2 + 1).abs_diff(gap_count) + 1)
        .collect();
    let total = weights.iter().sum::<usize>().max(1);
    let gaps: Vec<usize> = weights.iter().map(|w| 1 + spread * w / total).collect();
    let label_width = letters.len() + gaps.iter().sum::<usize>();
    let mut spans = vec![blank(width.saturating_sub(label_width) / 2)];
    let center = letters.len().saturating_sub(1);
    for (index, letter) in letters.iter().enumerate() {
        let edge = if center == 0 {
            0.0
        } else {
            (index * 2).abs_diff(center) as f32 / center as f32
        };
        let stagger = 0.22 * edge;
        let own = ((opacity - stagger) / (1.0 - stagger)).clamp(0.0, 1.0);
        let mut style = Style::new().attrs(Attrs::BOLD);
        fade(&mut style, ink.grade, own, 1.0, ink.colors);
        spans.push(Span::styled(letter.to_string(), style));
        if let Some(gap) = gaps.get(index) {
            spans.push(blank(*gap));
        }
    }
    clip(spans, width, ink.measure)
}

/// `transition_line_at`: the status line `elapsed` into the run, `width` cells
/// wide. `previous` is the line that was showing, `current` the new one.
pub(crate) fn transition_spans(
    ink: &Ink<'_>,
    previous: &[Span],
    current: &[Span],
    elapsed: Duration,
    width: usize,
) -> Vec<Span> {
    if width == 0 {
        return Vec::new();
    }
    if elapsed < SCROLL_OUT {
        let progress = elapsed.as_secs_f32() / SCROLL_OUT.as_secs_f32();
        let offset = (width as f32 * ease_in_cubic(progress)).round() as usize;
        let mut frame = previous.to_vec();
        fade_spans(
            &mut frame,
            ink,
            1.0 - progress * progress,
            0.12 + 0.88 * progress,
        );
        frame.insert(0, blank(offset));
        return clip(frame, width, ink.measure);
    }
    let after = elapsed.saturating_sub(SCROLL_OUT);
    let label_run = ASSEMBLE + HOLD + FADE_OUT;
    if after < label_run {
        let (assemble, opacity) = if after < ASSEMBLE {
            let progress = after.as_secs_f32() / ASSEMBLE.as_secs_f32();
            (ease_out_cubic(progress), (progress / 0.55).clamp(0.0, 1.0))
        } else if after < ASSEMBLE + HOLD {
            (1.0, 1.0)
        } else {
            let faded = after.saturating_sub(ASSEMBLE + HOLD).as_secs_f32();
            (1.0, 1.0 - faded / FADE_OUT.as_secs_f32())
        };
        return label_line(ink, width, assemble, opacity);
    }
    let opacity = ((after - label_run).as_secs_f32() / FADE_IN.as_secs_f32()).clamp(0.0, 1.0);
    let mut frame = current.to_vec();
    fade_spans(&mut frame, ink, opacity, 0.0);
    clip(frame, width, ink.measure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::theme::effort::{palette_styles, sample_model_style};
    use crate::pages::session::footer::strings::sample;

    fn text(spans: &[Span]) -> String {
        spans.iter().map(|span| span.text.to_string()).collect()
    }

    fn line(model: &str) -> Vec<Span> {
        vec![
            Span::styled(model.to_string(), sample_model_style()),
            Span::styled(sample::BRANCH, Style::new().attrs(Attrs::DIM)),
        ]
    }

    fn width_of(text: &str) -> usize {
        text.chars().count()
    }

    // oracle effort_status_line_tests.rs `transition_frames_snapshot`
    // (snapshot effort_status_line_transition_frames): the status line's text at each time
    #[test]
    fn the_transition_frames_equal_the_oracle_snapshot() {
        let want = [
            (Grade::Max, 0, "gpt-5.4 high \u{b7} main"),
            (Grade::Max, 300, "    gpt-5.4 high \u{b7} main"),
            (Grade::Max, 520, "                   gpt-5.4 high"),
            (Grade::Max, 680, "   M           A           X"),
            (Grade::Max, 820, "        M      A      X"),
            (Grade::Max, 1050, "            M  A  X"),
            (Grade::Max, 1320, "             M A X"),
            (Grade::Max, 1550, "             M A X"),
            (Grade::Max, 1800, "             M A X"),
            (Grade::Max, 2020, "gpt-5.4 max \u{b7} main"),
            (Grade::Max, 2250, "gpt-5.4 max \u{b7} main"),
            (Grade::Max, 2500, "gpt-5.4 max \u{b7} main"),
            (Grade::Ultra, 0, "gpt-5.4 high \u{b7} main"),
            (Grade::Ultra, 300, "    gpt-5.4 high \u{b7} main"),
            (Grade::Ultra, 520, "                   gpt-5.4 high"),
            (Grade::Ultra, 680, "  U       L    T    R       A"),
            (Grade::Ultra, 820, "        U   L  T  R   A"),
            (Grade::Ultra, 1050, "           U L T R A"),
            (Grade::Ultra, 1320, "           U L T R A"),
            (Grade::Ultra, 1550, "           U L T R A"),
            (Grade::Ultra, 1800, "           U L T R A"),
            (Grade::Ultra, 2020, "gpt-5.4 ultra \u{b7} main"),
            (Grade::Ultra, 2250, "gpt-5.4 ultra \u{b7} main"),
            (Grade::Ultra, 2500, "gpt-5.4 ultra \u{b7} main"),
        ];
        let previous = line("gpt-5.4 high");
        for (tier, millis, text_at) in want {
            let current = match tier {
                Grade::Max => line("gpt-5.4 max"),
                Grade::Ultra => line("gpt-5.4 ultra"),
            };
            let frame = transition_spans(
                &Ink {
                    grade: tier,
                    label: match tier {
                        Grade::Max => "MAX",
                        Grade::Ultra => "ULTRA",
                    },
                    colors: Colors::default(),
                    measure: &width_of,
                },
                &previous,
                &current,
                Duration::from_millis(millis),
                32,
            );
            assert_eq!(text(&frame).trim_end(), text_at, "{tier:?} at {millis} ms");
        }
    }

    // oracle effort_status_line_tests.rs `palette_status_line_colors_survive_the_fade`
    #[test]
    fn palette_colours_survive_the_fade_and_dim() {
        for sample in palette_styles() {
            let (mut style, color) = (sample.clone(), sample.fg.unwrap());
            fade(&mut style, Grade::Ultra, 0.4, 1.0, Colors::default());
            assert_eq!(style.fg, Some(color));
            assert!(style.attrs.intersects(Attrs::DIM));
        }
    }

    // oracle effort_status_line_tests.rs `ultra_letters_start_wide_at_the_edges_and_converge_to_the_center`
    #[test]
    fn ultra_letters_start_wide_at_the_edges_and_converge() {
        let letters = |assemble: f32| {
            let ink = Ink {
                grade: Grade::Ultra,
                label: "ULTRA",
                colors: Colors::default(),
                measure: &width_of,
            };
            text(&label_line(&ink, 40, assemble, 1.0))
        };
        let scattered = letters(0.0);
        let positions: Vec<usize> = scattered
            .char_indices()
            .filter_map(|(at, letter)| letter.is_ascii_uppercase().then_some(at))
            .collect();
        let gaps: Vec<usize> = positions.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert_eq!(positions.len(), 5);
        assert!(gaps[0] > gaps[1]);
        assert!(gaps[3] > gaps[2]);
        assert_eq!(letters(1.0), "               U L T R A");
    }
}
