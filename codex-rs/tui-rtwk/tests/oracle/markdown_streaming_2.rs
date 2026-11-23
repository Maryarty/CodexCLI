//! Shard `markdown_streaming_2` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use codex_tui_core::app::facts::Rendering;

use crate::kit::{self, Owner, Screen};

/// The live chat draws an agent message as its plain text: no markdown renderer yet.
/// The chat draws math as its source text: no unicode math renderer yet.
const MATH: &str = "G61: rtwk-rich MarkdownView has no math renderer; the chat draws math sources as their text: no unicode math renderer (inline, display, named delimiters) is built in the chat's markdown yet";
/// `text` as the body of a JSON string.
fn json_string(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// One agent answer of the live chat whose message is `markdown` (the fixture's turn).
fn answer(markdown: &str) -> String {
    include_str!("fixtures/markdown_streaming_2/answer.jsonl")
        .replace("{MARKDOWN}", &json_string(markdown))
}

/// The live chat at `width` x `height` after the answer `markdown` has streamed in. The
/// oracle's own display of a web link is the one for a terminal not known to show hyperlinks
/// (its tests are no terminal): `label (https://…)`.
fn chat(width: u16, height: u16, markdown: &str) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(width, height)
        .live()
        .open();
    s.feed(&answer(markdown));
    s
}

/// `chat` with one `tui.rendering` renderer off: `Rendering` with the field `off` named false.
fn chat_without(off: &str, width: u16, height: u16, markdown: &str) -> Screen {
    let rendering = Rendering {
        mermaid: off != "mermaid",
        math: off != "math",
        tables: off != "tables",
        lists: off != "lists",
    };
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(width, height)
        .live()
        .open();
    s.rendering(rendering);
    s.feed(&answer(markdown));
    s
}

/// The host's fact for a terminal that shows hyperlinks (`WebLinkDisplay::LabelOnly`): a
/// `screen` step of the `cells` directory.
const HYPERLINKS: &str =
    r#"{"step":"screen","dir":"cells","ev":{"kind":"link_display","display":"label_only"}}"#;

/// `chat` on a terminal that shows hyperlinks: a web link is its label alone.
fn hyperlink_chat(width: u16, height: u16, markdown: &str) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(width, height)
        .live()
        .open();
    s.feed(HYPERLINKS);
    s.feed(&answer(markdown));
    s
}

// ---- oracle: markdown_render/math_tests.rs (6 screen-level tests here; 2 state tests left to chat-live)
// ORACLE markdown_render/math_tests.rs:16 unicode_math_inline_snapshot kind=snap owner=chat-live snap=unicode_math_inline_snapshot
#[test]
fn unicode_math_inline_snapshot() {
    kit::pending(Owner::ChatLive, MATH);
    let s = chat(
        80,
        24,
        r"Inline $\alpha^2 + \beta_{10}$ on $\mathbb{R}^n$.
Root $\sqrt{x^2+y^2}$ and fraction $a/\frac{b}{c}$.
Percent: $3.8\%$ and $100000 \times 3.8\% \times \frac{360}{365} \approx 3747.95$.
Unsupported: $\unknown{x_y}$ and $\frac{a}{b}^2$.
Angles: <$x$>, <\(\alpha\)>, and <$\unknown{x}$>.
Code: `$\alpha$`; money: $5 and $10; shell: $HOME.",
    );
    // The oracle's snapshot is the rendered lines alone, compared from the transcript's top-left.
    s.assert_markdown_snap("unicode_math_inline_snapshot");
}

// ORACLE markdown_render/math_tests.rs:29 unicode_math_inline_narrow_snapshot kind=snap owner=chat-live snap=unicode_math_inline_narrow_snapshot
#[test]
fn unicode_math_inline_narrow_snapshot() {
    kit::pending(Owner::ChatLive, MATH);
    let s = chat(
        16,
        24,
        r"Words $\alpha^2 + \beta_1$ more words and $\sqrt{x}$.",
    );
    s.assert_markdown_snap("unicode_math_inline_narrow_snapshot");
}

// ORACLE markdown_render/math_tests.rs:37 unicode_math_accents_and_symbols_snapshot kind=snap owner=chat-live snap=unicode_math_accents_and_symbols_snapshot
#[test]
fn unicode_math_accents_and_symbols_snapshot() {
    kit::pending(Owner::ChatLive, MATH);
    let s = chat(
        80,
        24,
        r"Operators: $\hat H^2 + \hbar^2$, $\ell$, $A\dagger$, $B\ddagger$.
Accents: $\bar{x}_1$, $\tilde{x}^2$, $\vec{v}_i$, $\dot{x}$, $\ddot{x}$.
Greek accents: $\hat{\Psi}^2$, $\bar{\varrho}$, $\tilde{\varsigma}$.
Other symbols: $\varkappa+\varpi$, $\Re z+\Im z$, $\aleph_0$, $\hat{\imath}$.
Relations: $a\propto b\sim c\simeq d\ll e\gg f$.
Geometry: $a\perp b\parallel c$, $\angle ABC\cong\angle DEF$.
Bounds: $a\lesssim b\gtrsim c$; operators: $a\oplus b\otimes c\odot d$.
Sets: $A\supseteq B\supset C\ni x$, $A\setminus B=\varnothing$.
Logic: $\neg P\land Q\lor R\implies S\iff T\impliedby U$, $\nexists x$.
Arrows: $a\leftrightarrow b\mapsto c\Leftarrow d$, $\uparrow\downarrow\updownarrow$.
Integrals: $\iint f$, $\iiint g$, $\oint h$.
Collections: $\coprod A$, $\bigcup B$, $\bigcap C$.
Punctuation: $x\prime$, $a\circ b\bullet c$, $\vdots\ddots$.",
    );
    s.assert_markdown_snap("unicode_math_accents_and_symbols_snapshot");
}

// ORACLE markdown_render/math_tests.rs:57 unicode_math_named_delimiters_snapshot kind=snap owner=chat-live snap=unicode_math_named_delimiters_snapshot
#[test]
fn unicode_math_named_delimiters_snapshot() {
    kit::pending(Owner::ChatLive, MATH);
    let s = chat(
        80,
        24,
        r"Inner product: $\left\langle\hat H\right\rangle$.
Rounding: $\lfloor x\rfloor + \left\lceil y\right\rceil$.
Norms: $\left\lVert v\right\rVert$, $\lvert x\rvert$, $\left\|w\right\|$.
Sets: $\left\{x\right\}$, $\lbrace y\rbrace$, $\lbrack z\rbrack$.
Angles: $\left<x\right>$.",
    );
    s.assert_markdown_snap("unicode_math_named_delimiters_snapshot");
}

// ORACLE markdown_render/math_tests.rs:69 unicode_math_accents_reject_ambiguous_arguments kind=ui owner=chat-live
#[test]
fn unicode_math_accents_reject_ambiguous_arguments() {
    kit::pending(Owner::ChatLive, MATH);
    // The oracle's sources, each one paragraph of the answer. The renderer's `None`
    // for both math modes is not drawn; the source shown as its own text is.
    let sources = [
        r"\hat{xy}",
        r"\bar{x+y}",
        r"\tilde{x^2}",
        r"\vec{\frac{x}{y}}",
        r"\dot{}",
        r"\ddot{ }",
        r"\hat",
        "\\hat{\u{0302}}",
    ];
    let markdown = sources
        .iter()
        .map(|source| format!("\\({source}\\)"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let s = chat(80, 24, &markdown);
    for source in sources {
        s.assert_has_row(&format!("\\({source}\\)"));
    }
}

// ORACLE markdown_render/math_tests.rs:121 unicode_math_bounds_and_unsupported_input kind=ui owner=chat-live
#[test]
fn unicode_math_bounds_and_unsupported_input() {
    kit::pending(Owner::ChatLive, MATH);
    // The oracle's unsupported sources, each one paragraph; the `None` results and
    // the 40-brace / 300-letter / 100-`\sqrt` inputs are not drawn, the raw text is.
    let sources = [
        r"\unknown{x}",
        r"\frac{a}",
        "{x",
        "x}",
        "^2",
        "x^2^3",
        "{a+b}^2",
        "{x^2}^3",
        r"\frac{a}{b}^2",
        r"\sqrt[3]{x}",
        r"\sqrt [3]{x}",
        r"\left x",
        r"\left\alpha x\right\rangle",
        r"\left\ ",
        r"\text{\alpha}",
        r"\begin{matrix}a&b\end{matrix}",
    ];
    let markdown = sources
        .iter()
        .map(|source| format!("\\({source}\\)"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let s = chat(80, 24, &markdown);
    for source in sources {
        s.assert_has_row(&format!("\\({source}\\)"));
    }
}

// ---- oracle: markdown_render/mermaid_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render/mermaid_tests.rs:20 mermaid_fences_use_native_renderer_for_every_family kind=snap owner=chat-live
#[test]
fn mermaid_fences_use_native_renderer_for_every_family() {
    kit::pending(
        Owner::ChatLive,
        "G62: rtwk-rich MarkdownView has no mermaid renderer; a mermaid fence draws as its source text: the native mermaid renderer is not built in the chat's markdown yet; probe: the body passes without it (it checks less than the oracle claims)",
    );
    // The oracle compares each diagram with `codex_mermaid::render(source, 100)`, which
    // the kit cannot state as rows; the visible half is that the fence is not drawn as
    // its source. The diagram rows are the native renderer's, to be added with it.
    for source in [
        "%% heading\nflowchart TD; A --> B",
        "graph LR; A --> B",
        "flowchart; A -- send --> B & C; B <-.-> C",
        "sequenceDiagram; A->>B: request; B-->>A: response",
        "stateDiagram-v2; [*] --> Active; Active --> [*]",
        "stateDiagram; [*] --> Active; Active --> [*]",
        "classDiagram; Order \"1\" *-- \"many\" Item : contains",
        "erDiagram; CUSTOMER ||--o{ ORDER : places",
    ] {
        let s = chat(
            100,
            24,
            &format!("```mermaid title=example\n{source}\n```\n"),
        );
        s.assert_absent("```mermaid");
    }
}

// ---- oracle: markdown_render/preferences_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render/preferences_tests.rs:10 disabled_renderers_preserve_source_independently kind=snap owner=chat-live snap=disabled_renderers_preserve_source_independently
#[test]
fn disabled_renderers_preserve_source_independently() {
    // The oracle's snapshot joins four preference states under a Debug header each: one
    // renderer off, its sample drawn as the source it was written.
    let s = chat_without(
        "mermaid",
        80,
        24,
        "```mermaid\nflowchart LR\nA[Request] --> B[Reply]\n```\n",
    );
    s.assert_in_order(&["flowchart LR", "A[Request] --> B[Reply]"]);
    // math off: the `$..$` span stays as written (the chat has no math renderer, G61, and the
    // parser still reads the `\(..\)` and `\[..\]` delimiters as escapes: the rows of those
    // are the gap of the math renderer, not of the switch)
    let s = chat_without("math", 80, 24, "Inline $\\alpha_1$ and $x^{2}$.\n");
    s.assert_in_order(&["Inline $\\alpha_1$ and $x^{2}$."]);
    // tables off: the table is its source, markup and escapes included
    let table = "| **Name** | Value |\n|---|---|\n| `a` | path\\_1 &amp; \\* |\n";
    let s = chat_without("tables", 80, 24, table);
    s.assert_in_order(&[
        "| **Name** | Value |",
        "|---|---|",
        "| `a` | path\\_1 &amp; \\* |",
    ]);
    // and drawn as a table while the renderer is on
    let on = chat(80, 24, table);
    on.assert_absent("|---|---|");
    // lists off: a task item's box is the text it was written
    let s = chat_without("lists", 80, 24, "- [ ] Pending\n- [x] Done\n- Ordinary\n");
    s.assert_in_order(&["[ ] Pending", "[x] Done", "Ordinary"]);
}

// ORACLE markdown_render/preferences_tests.rs:57 disabled_mermaid_keeps_unsupported_source_without_notice kind=ui owner=chat-live
#[test]
fn disabled_mermaid_keeps_unsupported_source_without_notice() {
    // With mermaid off the fence renders as the unknown fence does: its source lines, no notice.
    let source = "```mermaid\npie\n\"Cats\": 2\n```\n";
    let off = chat_without("mermaid", 80, 24, source);
    off.assert_in_order(&["pie", "\"Cats\": 2"]);
    off.assert_same_rows(&chat(80, 24, &source.replacen("mermaid", "unknown", 1)).shot());
}

// ORACLE markdown_render/preferences_tests.rs:72 disabled_tables_keep_markdown_fences_and_cell_markup kind=snap owner=chat-live snap=disabled_tables_keep_mixed_quote_spacing
#[test]
fn disabled_tables_keep_markdown_fences_and_cell_markup() {
    for fence in [
        "```md",
        "```markdown",
        "~~~markdown",
        "```MARKDOWN title=example",
    ] {
        let closer = &fence[..3];
        let source = format!("{fence}\n| A | B |\n|---|---|\n| **a** | $x^2$ |\n{closer}\n");
        let s = chat_without("tables", 80, 24, &source);
        s.assert_in_order(&[fence, "| A | B |", "|---|---|", "| **a** | $x^2$ |", closer]);
    }
    let quoted =
        "> ```md\n> Intro\n>\n>| A | B |\n> |---|---|\n> | a | b |\n>\n>> literal\n> ```\n";
    // The oracle's snapshot draws the quote with the source's `>` marker (G73 item 3: rtwk's
    // MarkdownView draws a `│` bar), so the rows are compared by their text: the literal block
    // keeps the fence, the blank rows of the quote and the nested `> literal` line.
    let s = chat_without("tables", 80, 24, quoted);
    s.assert_in_order(&[
        "```md",
        "Intro",
        "| A | B |",
        "|---|---|",
        "| a | b |",
        "> literal",
        "```",
    ]);
}

// ---- oracle: markdown_render/task_list_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render/task_list_tests.rs:10 lists_preserve_layout kind=snap owner=chat-live snap=codex_tui__markdown_render__task_list_tests__lists_preserve_layout
#[test]
fn lists_preserve_layout() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 4, the oracle draws a task item with `☐` / `☑` in place of the bullet and the next ordinary item as `• Ordinary`; rtwk draws `[ ]` / `[x]` and pads the ordinary bullet to the width of the checkbox (`•   Ordinary`)",
    );
    // The oracle joins twelve sources at width 24 into one snapshot; this ports the
    // first three stages (rows of the snapshot), the others follow the same pattern.
    let stage = chat(
        24,
        24,
        "- [ ] Unchecked\n- [x] Checked\n- [X] Uppercase\n- Ordinary\n",
    );
    stage.assert_in_order(&["☐ Unchecked", "☑ Checked", "☑ Uppercase", "• Ordinary"]);
    let stage = chat(
        24,
        24,
        "- [ ] Parent task with enough words to wrap\n  - [x] Nested task\n  - Ordinary child\n- [x] Done\n",
    );
    stage.assert_in_order(&[
        "☐ Parent task with",
        "enough words to wrap",
        "☑ Nested task",
        "• Ordinary child",
        "☑ Done",
    ]);
    let stage = chat(
        24,
        24,
        "9. [ ] Numbered task with enough words to wrap\n10. [x] Done\n11. Ordinary\n",
    );
    stage.assert_in_order(&["9. ☐ Numbered task with", "10. ☑ Done", "11. Ordinary"]);
}

// ---- oracle: markdown_render/web_links_tests.rs (9 screen-level tests here; 1 state tests left to chat-live)
// ORACLE markdown_render/web_links_tests.rs:22 bare_urls_are_styled_without_coloring_surrounding_prose kind=snap owner=chat-live snap=bare_urls_are_styled_without_coloring_surrounding_prose
#[test]
fn bare_urls_are_styled_without_coloring_surrounding_prose() {
    kit::pending(
        Owner::ChatLive,
        "G-new (bare URL style): the oracle draws a URL written in the text underlined in the text's own colour (`fg=Reset underlined`); rtwk-rich parses it as a link and the theme's link colour applies (`Rgb(137, 180, 250)`), the link style being one for every link; the prose around it matches",
    );
    let s = chat(
        100,
        24,
        "界 (https://example.com/a); https://example.com/b! **https://example.com/c**",
    );
    s.assert_has_row("界 (https://example.com/a); https://example.com/b! https://example.com/c");
    s.assert_link("https://example.com/a", "https://example.com/a");
    s.assert_link("https://example.com/b", "https://example.com/b");
    s.assert_link("https://example.com/c", "https://example.com/c");
    s.assert_text_style("https://example.com/a", "fg=Reset underlined");
    s.assert_text_style("https://example.com/b", "fg=Reset underlined");
    s.assert_text_style("https://example.com/c", "fg=Reset bold underlined");
}

// ORACLE markdown_render/web_links_tests.rs:59 bare_urls_keep_link_style_when_wrapped_in_tables kind=snap owner=chat-live snap=bare_urls_keep_link_style_when_wrapped_in_tables
#[test]
fn bare_urls_keep_link_style_when_wrapped_in_tables() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle wraps a table cell inside its column (the URL on its own rows, every row linked); rtwk's table cuts the cell with `…` (`https://example.com…`) and the target is gone from the screen",
    );
    let s = chat(
        20,
        24,
        "| Resource |\n| --- |\n| https://example.com/docs |",
    );
    s.assert_in_order(&["Resource"]);
    s.assert_link("https://example.com/docs", "https://example.com/docs");
}

// ORACLE markdown_render/web_links_tests.rs:89 supporting_terminals_render_only_the_styled_label_and_keep_its_target kind=ui owner=chat-live
#[test]
fn supporting_terminals_render_only_the_styled_label_and_keep_its_target() {
    // On a supporting terminal the label is the only text; the target is the link.
    let s = hyperlink_chat(80, 24, "[label](https://example.com)");
    s.assert_link("label", "https://example.com");
    s.assert_absent("(https://example.com)");
}

// ORACLE markdown_render/web_links_tests.rs:148 unknown_terminals_and_multiplexers_keep_visible_destinations kind=ui owner=chat-live
#[test]
fn unknown_terminals_and_multiplexers_keep_visible_destinations() {
    // On an unknown terminal or a multiplexer the destination stays visible next to the label.
    let s = chat(80, 24, "[label](https://example.com)");
    s.assert_in_order(&["label", "https://example.com"]);
    s.assert_link("label", "https://example.com");
}

// ORACLE markdown_render/web_links_tests.rs:177 label_only_policy_keeps_non_web_links_and_literal_urls_unchanged kind=ui owner=chat-live
#[test]
fn label_only_policy_keeps_non_web_links_and_literal_urls_unchanged() {
    // The oracle compares the label-only and destination renderings for equality; the
    // visible half is that the non-web links, the literal URL and the code span stay as text.
    let s = chat(
        80,
        24,
        "[file](./src/main.rs) [mail](mailto:a@example.com)\n\nhttps://example.com\n\n`https://example.com/code`",
    );
    s.assert_in_order(&[
        "file",
        "mail",
        "https://example.com",
        "https://example.com/code",
    ]);
}

// ORACLE markdown_render/web_links_tests.rs:186 empty_web_link_labels_keep_a_visible_destination kind=ui owner=chat-live
#[test]
fn empty_web_link_labels_keep_a_visible_destination() {
    for markdown in [
        "[](https://example.com)",
        "[ ](https://example.com)",
        "[\u{200d}](https://example.com)",
    ] {
        let s = chat(80, 24, markdown);
        s.assert_in_order(&["https://example.com"]);
    }
}

// ORACLE markdown_render/web_links_tests.rs:200 label_only_web_links_keep_targets_after_prose_and_table_wrapping kind=ui owner=chat-live
#[test]
fn label_only_web_links_keep_targets_after_prose_and_table_wrapping() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, a table cell wider than the width: the oracle wraps it (width 12: the labels on several rows, every row linked), rtwk's table cuts the cell with `…` (`plain label…`) and the other two targets are gone; prose at both widths and the table at 40 match",
    );
    let links = "[plain label](https://example.com/plain) [`code label`](https://example.com/code) [<b>](https://example.com/html)";
    for width in [12, 40] {
        for markdown in [
            links.to_string(),
            format!("| Links |\n| --- |\n| {links} |"),
        ] {
            // The oracle collects the targets of the rendered lines: a label that wraps
            // (width 12) still carries its target on every row.
            let s = hyperlink_chat(width, 24, &markdown);
            let grid = s.grid();
            let targets: std::collections::BTreeSet<String> = (0..grid.height)
                .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
                .filter_map(|(x, y)| grid.cell(x, y).link.as_deref().map(str::to_string))
                .collect();
            assert_eq!(
                targets.iter().map(String::as_str).collect::<Vec<_>>(),
                [
                    "https://example.com/code",
                    "https://example.com/html",
                    "https://example.com/plain"
                ],
                "width {width}, {markdown}:\n{}",
                s.text()
            );
            s.assert_absent("https://");
        }
    }
}

// ORACLE markdown_render/web_links_tests.rs:231 streaming_and_full_render_agree_with_label_only_links kind=ui owner=chat-live
#[test]
fn streaming_and_full_render_agree_with_label_only_links() {
    // The oracle compares the streamed render of every prefix with the full render; this
    // ports the finished frame (the visible result). The per-prefix frames need a feed per
    // prefix and are not written yet.
    let s = hyperlink_chat(
        40,
        24,
        "See [**label**](https://example.com).\n\n| Resource |\n| --- |\n| [`code`](https://example.com/code) |\n\nTrailing text.",
    );
    s.assert_link("label", "https://example.com");
    s.assert_link("code", "https://example.com/code");
    s.assert_has_row("Trailing text.");
}

// ORACLE markdown_render/web_links_tests.rs:250 label_only_and_fallback_presentations_snapshot kind=snap owner=chat-live snap=label_only_and_fallback_presentations_snapshot
#[test]
fn label_only_and_fallback_presentations_snapshot() {
    // The oracle's snapshot holds both presentations as Debug text; the label-only one is
    // the chat's, asserted here as its links (the destination presentation is the other).
    let s = hyperlink_chat(
        40,
        24,
        "plain [plain](https://example.com) `code` [`code`](https://example.com)\n\n| Resource |\n| --- |\n| [docs](https://example.com/docs) |",
    );
    // Row 0 reads `plain plain code code`: the first word is prose, the second the label, and
    // likewise the code words (the oracle's snapshot has the link on the labelled ones).
    let grid = s.grid();
    let (x, _, y) = grid
        .locate("plain plain code code")
        .unwrap_or_else(|| panic!("no label-only row:\n{}", s.text()));
    let linked = |from: usize, len: usize| {
        (from..from + len).all(|c| grid.cell(c, y).link.as_deref() == Some("https://example.com"))
    };
    assert!(!linked(x, 5), "the prose `plain` links");
    assert!(linked(x + 6, 5), "the label `plain` has no link");
    assert!(!linked(x + 12, 4), "the prose `code` links");
    assert!(linked(x + 17, 4), "the code label has no link");
    s.assert_link("docs", "https://example.com/docs");
}

// ---- oracle: markdown_render_tests.rs (40 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render_tests.rs:36 bare_url_with_tilde_keeps_complete_hyperlink kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__bare_url_with_tilde_keeps_complete_hyperlink
#[test]
fn bare_url_with_tilde_keeps_complete_hyperlink() {
    // The oracle's snapshot is the Debug of the rows and their hyperlink columns: the row and its link.
    let destination = "https://www.cs.tufts.edu/~nr/cs257/archive/olin-shivers/dissertation.pdf";
    let s = chat(80, 24, destination);
    s.assert_has_row(destination);
    s.assert_link(destination, destination);
}

// ORACLE markdown_render_tests.rs:65 table_url_with_tilde_keeps_complete_hyperlink kind=ui owner=chat-live
#[test]
fn table_url_with_tilde_keeps_complete_hyperlink() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle wraps the URL inside the cell at width 32 and keeps the whole destination on every row; rtwk's table cuts the cell with `…` (`https://www.cs.tufts.edu/~nr/cs…`)",
    );
    let destination = "https://www.cs.tufts.edu/~nr/cs257/archive/olin-shivers/dissertation.pdf";
    let markdown = format!("| URL |\n| --- |\n| {destination} |\n");
    let s = chat(32, 24, &markdown);
    s.assert_link(destination, destination);
}

// ORACLE markdown_render_tests.rs:84 links_keep_their_columns_after_styled_fragments kind=ui owner=chat-live
#[test]
fn links_keep_their_columns_after_styled_fragments() {
    // The oracle hides every destination here (`&|_| true`): the labels only.
    let s = hyperlink_chat(
        80,
        24,
        "**界** [a](https://example.com/a) _b_ [`c`](https://example.com/c) [d](https://example.com/d)\n\n[e](https://example.com/e)",
    );
    s.assert_link("a", "https://example.com/a");
    s.assert_link("c", "https://example.com/c");
    s.assert_link("d", "https://example.com/d");
    s.assert_link("e", "https://example.com/e");
}

// ORACLE markdown_render_tests.rs:109 merged_text_events_preserve_entity_decoding kind=ui owner=chat-live
#[test]
fn merged_text_events_preserve_entity_decoding() {
    let destination = "https://example.com/a&b~c";
    let s = chat(80, 24, "https://example.com/a&amp;b~c");
    s.assert_has_row(destination);
    s.assert_link(destination, destination);
}

// ORACLE markdown_render_tests.rs:145 empty kind=snap owner=chat-live snap=empty
#[test]
fn empty() {
    kit::na(
        Owner::ChatLive,
        "asserts that render_markdown_text(\"\") is the default Text, a returned value: an empty answer draws no row, and the snapshot is a Debug of that value, not a screen",
    );
}

// ORACLE markdown_render_tests.rs:150 paragraph_single kind=ui owner=chat-live
#[test]
fn paragraph_single() {
    let s = chat(80, 24, "Hello, world!");
    s.assert_has_row("Hello, world!");
}

// ORACLE markdown_render_tests.rs:158 paragraph_soft_break kind=ui owner=chat-live
#[test]
fn paragraph_soft_break() {
    let s = chat(80, 24, "Hello\nWorld");
    // The oracle's two lines, one under the other.
    let rows = s.grid().rows(false);
    let at = rows
        .iter()
        .position(|row| row == "Hello")
        .unwrap_or_else(|| panic!("no `Hello` row:\n{}", s.text()));
    assert_eq!(rows.get(at + 1).map(String::as_str), Some("World"));
}

// ORACLE markdown_render_tests.rs:166 paragraph_multiple kind=ui owner=chat-live
#[test]
fn paragraph_multiple() {
    let s = chat(80, 24, "Paragraph 1\n\nParagraph 2");
    s.assert_in_order(&["Paragraph 1", "Paragraph 2"]);
}

// ORACLE markdown_render_tests.rs:174 headings kind=ui owner=chat-live
#[test]
fn headings() {
    let s = chat(
        80,
        24,
        "# Heading 1\n## Heading 2\n### Heading 3\n#### Heading 4\n##### Heading 5\n###### Heading 6\n",
    );
    s.assert_in_order(&[
        "# Heading 1",
        "## Heading 2",
        "### Heading 3",
        "#### Heading 4",
        "##### Heading 5",
        "###### Heading 6",
    ]);
    s.assert_text_style("Heading 1", "bold underlined");
    s.assert_text_style("Heading 2", "bold");
    s.assert_text_style("Heading 3", "bold italic");
    s.assert_text_style("Heading 4", "italic");
    s.assert_text_style("Heading 5", "italic");
    s.assert_text_style("Heading 6", "italic");
}

// ORACLE markdown_render_tests.rs:195 blockquote_single kind=ui owner=chat-live
#[test]
fn blockquote_single() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> Blockquote");
    s.assert_has_row("> Blockquote");
    s.assert_text_style("> Blockquote", "fg=Green");
}

// ORACLE markdown_render_tests.rs:203 blockquote_soft_break kind=ui owner=chat-live
#[test]
fn blockquote_soft_break() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle repeats the source marker `> ` on every row of a quote; rtwk's MarkdownView draws the `│ ` bar (the soft break itself is a line break as in the oracle)",
    );
    let s = chat(80, 24, "> This is a blockquote\nwith a soft break\n");
    s.assert_in_order(&["> This is a blockquote", "> with a soft break"]);
}

// ORACLE markdown_render_tests.rs:226 blockquote_multiple_with_break kind=ui owner=chat-live
#[test]
fn blockquote_multiple_with_break() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> Blockquote 1\n\n> Blockquote 2\n");
    s.assert_in_order(&["> Blockquote 1", "> Blockquote 2"]);
    s.assert_text_style("> Blockquote 1", "fg=Green");
    s.assert_text_style("> Blockquote 2", "fg=Green");
}

// ORACLE markdown_render_tests.rs:238 blockquote_three_paragraphs_short_lines kind=ui owner=chat-live
#[test]
fn blockquote_three_paragraphs_short_lines() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> one\n>\n> two\n>\n> three\n");
    s.assert_in_order(&["> one", "> two", "> three"]);
    s.assert_text_style("> one", "fg=Green");
}

// ORACLE markdown_render_tests.rs:253 blockquote_nested_two_levels kind=ui owner=chat-live
#[test]
fn blockquote_nested_two_levels() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> Level 1\n>> Level 2\n");
    s.assert_in_order(&["> Level 1", "> > Level 2"]);
}

// ORACLE markdown_render_tests.rs:266 blockquote_with_list_items kind=ui owner=chat-live
#[test]
fn blockquote_with_list_items() {
    kit::pending(
        Owner::ChatLive,
        "G73: items 1 and 4, the chat's Markdown now reads the syntax theme (the empty name is the oracle's `Theme::default()`, no scope), but rtwk's `MarkdownTheme.marker` is one style for ordered numbers and bullets (the oracle: numbers light blue, bullets the theme's scope or the quote's green) and the quote is a `│` bar, not `> `: the bullet reads LightBlue and the rows `│ • item 1`",
    );
    // the oracle sets `Theme::default()`: the empty name is a theme with no scope
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .syntax_theme("")
        .live()
        .open();
    s.feed(&answer("> - item 1\n> - item 2\n"));
    s.assert_in_order(&["> • item 1", "> • item 2"]);
    s.assert_text_style("> • item 1", "fg=Green");
}

// ORACLE markdown_render_tests.rs:278 blockquote_with_ordered_list kind=ui owner=chat-live
#[test]
fn blockquote_with_ordered_list() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> 1. first\n> 2. second\n");
    s.assert_in_order(&["> 1. first", "> 2. second"]);
    s.assert_text_style("1. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:300 blockquote_list_then_nested_blockquote kind=ui owner=chat-live
#[test]
fn blockquote_list_then_nested_blockquote() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> - parent\n>   > child\n");
    s.assert_in_order(&["> • parent", ">   > child"]);
}

// ORACLE markdown_render_tests.rs:312 list_item_with_inline_blockquote_on_same_line kind=ui owner=chat-live
#[test]
fn list_item_with_inline_blockquote_on_same_line() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "1. > quoted\n");
    s.assert_has_row("1. > quoted");
}

// ORACLE markdown_render_tests.rs:323 blockquote_surrounded_by_blank_lines kind=ui owner=chat-live
#[test]
fn blockquote_surrounded_by_blank_lines() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "foo\n\n> bar\n\nbaz\n");
    s.assert_in_order(&["foo", "> bar", "baz"]);
}

// ORACLE markdown_render_tests.rs:349 blockquote_in_ordered_list_on_next_line kind=ui owner=chat-live
#[test]
fn blockquote_in_ordered_list_on_next_line() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "1.\n   > quoted\n");
    s.assert_has_row("1. > quoted");
}

// ORACLE markdown_render_tests.rs:368 blockquote_in_unordered_list_on_next_line kind=ui owner=chat-live
#[test]
fn blockquote_in_unordered_list_on_next_line() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "-\n  > quoted\n");
    s.assert_has_row("• > quoted");
}

// ORACLE markdown_render_tests.rs:387 blockquote_two_paragraphs_inside_ordered_list_has_blank_line kind=ui owner=chat-live
#[test]
fn blockquote_two_paragraphs_inside_ordered_list_has_blank_line() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "1.\n   > para 1\n   >\n   > para 2\n");
    s.assert_in_order(&["1. > para 1", "   > para 2"]);
}

// ORACLE markdown_render_tests.rs:413 blockquote_inside_nested_list kind=ui owner=chat-live
#[test]
fn blockquote_inside_nested_list() {
    kit::pending(
        Owner::ChatLive,
        "G64: MarkdownView indents a nested list by its parent's marker (2 columns), the oracle by 4 (flutter_markdown listIndent)",
    );
    let s = chat(80, 24, "1. A\n    - B\n      > inner\n");
    s.assert_in_order(&["1. A", "    • B", "      > inner"]);
}

// ORACLE markdown_render_tests.rs:430 list_item_text_then_blockquote kind=ui owner=chat-live
#[test]
fn list_item_text_then_blockquote() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "1. before\n   > quoted\n");
    s.assert_in_order(&["1. before", "   > quoted"]);
}

// ORACLE markdown_render_tests.rs:447 list_item_blockquote_then_text kind=ui owner=chat-live
#[test]
fn list_item_blockquote_then_text() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws the quote inside a list item with its `> ` source marker (`1. > quoted`, `   > after`); rtwk draws `1. │ quoted`",
    );
    let s = chat(80, 24, "1.\n   > quoted\n   after\n");
    s.assert_in_order(&["1. > quoted", "   > after"]);
}

// ORACLE markdown_render_tests.rs:464 list_item_text_blockquote_text kind=ui owner=chat-live
#[test]
fn list_item_text_blockquote_text() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws the quote inside a list item with its `> ` source marker (`   > quoted`); rtwk draws `   │ quoted`",
    );
    let s = chat(80, 24, "1. before\n   > quoted\n   after\n");
    s.assert_in_order(&["1. before", "   > quoted", "   > after"]);
}

// ORACLE markdown_render_tests.rs:481 blockquote_with_heading_and_paragraph kind=ui owner=chat-live
#[test]
fn blockquote_with_heading_and_paragraph() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> # Heading\n> paragraph text\n");
    s.assert_in_order(&["> # Heading", "> paragraph text"]);
}

// ORACLE markdown_render_tests.rs:506 blockquote_heading_inherits_heading_style kind=ui owner=chat-live
#[test]
fn blockquote_heading_inherits_heading_style() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> # test header\n> in blockquote\n");
    s.assert_in_order(&["> # test header", "> in blockquote"]);
    s.assert_text_style("test header", "bold underlined");
    s.assert_text_style("> in blockquote", "fg=Green");
}

// ORACLE markdown_render_tests.rs:525 blockquote_with_code_block kind=ui owner=chat-live
#[test]
fn blockquote_with_code_block() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> ```\n> code\n> ```\n");
    s.assert_has_row("> code");
}

// ORACLE markdown_render_tests.rs:542 blockquote_with_multiline_code_block kind=ui owner=chat-live
#[test]
fn blockquote_with_multiline_code_block() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(80, 24, "> ```\n> first\n> second\n> ```\n");
    s.assert_in_order(&["> first", "> second"]);
}

// ORACLE markdown_render_tests.rs:559 nested_blockquote_with_inline_and_fenced_code kind=ui owner=chat-live
#[test]
fn nested_blockquote_with_inline_and_fenced_code() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = chat(
        80,
        24,
        r#"> Nested quote with code:
> > Inner quote and `inline code`
> >
> > ```
> > # fenced code inside a quote
> > echo "hello from a quote"
> > ```
"#,
    );
    s.assert_in_order(&[
        "> Nested quote with code:",
        "> > Inner quote and inline code",
        "> > # fenced code inside a quote",
        "> > echo \"hello from a quote\"",
    ]);
}

// ORACLE markdown_render_tests.rs:602 list_unordered_single kind=ui owner=chat-live
#[test]
fn list_unordered_single() {
    let s = chat(80, 24, "- List item 1\n");
    s.assert_has_row("• List item 1");
}

// ORACLE markdown_render_tests.rs:610 list_unordered_multiple kind=ui owner=chat-live
#[test]
fn list_unordered_multiple() {
    let s = chat(80, 24, "- List item 1\n- List item 2\n");
    s.assert_in_order(&["• List item 1", "• List item 2"]);
}

// ORACLE markdown_render_tests.rs:621 list_ordered kind=ui owner=chat-live
#[test]
fn list_ordered() {
    let s = chat(80, 24, "1. List item 1\n2. List item 2\n");
    s.assert_in_order(&["1. List item 1", "2. List item 2"]);
    s.assert_text_style("1. ", "fg=LightBlue");
    s.assert_text_style("2. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:631 empty_list_items_keep_their_markers kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__empty_list_items
#[test]
fn empty_list_items_keep_their_markers() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 4 and G-new (empty item): the oracle draws an empty item's marker on its own row and the nested one below it, four cells in (`•` / `    •`); rtwk puts the nested marker on the parent's row (`• ◦`)",
    );
    // The oracle's snapshot is the Debug of the rows of three sources; this ports the
    // first two (the `8.` marker row and the nested empty bullets), the third follows.
    let s = chat(80, 24, "8.");
    s.assert_has_row("8.");
    let s = chat(80, 24, "-\n  -");
    s.assert_in_order(&["•", "    •"]);
}

// ORACLE markdown_render_tests.rs:638 list_nested kind=ui owner=chat-live
#[test]
fn list_nested() {
    kit::pending(
        Owner::ChatLive,
        "G64: MarkdownView indents a nested list by its parent's marker (2 columns), the oracle by 4 (flutter_markdown listIndent)",
    );
    let s = chat(80, 24, "- List item 1\n  - Nested list item 1\n");
    s.assert_in_order(&["• List item 1", "    • Nested list item 1"]);
}

// ORACLE markdown_render_tests.rs:649 ordered_list_markers_use_terminal_palette_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__ordered_list_markers_use_terminal_palette_snapshot
#[test]
fn ordered_list_markers_use_terminal_palette_snapshot() {
    // The oracle's snapshot is the Debug of the Text: the marker in the palette's light
    // blue, the first link's label and target; the second link is the same shape.
    let s = chat(
        80,
        24,
        "1. plain [plain](https://example.com) `code` [`code`](https://example.com)",
    );
    s.assert_text_style("1. ", "fg=LightBlue");
    // `plain` stands twice (the text, then the link's label): the label is the second.
    let grid = s.grid();
    let (x, _, y) = grid
        .locate("plain plain")
        .expect("the two words on the screen");
    for x in x + "plain ".len()..x + "plain plain".len() {
        assert_eq!(grid.cell(x, y).link.as_deref(), Some("https://example.com"));
    }
    s.assert_no_link("1. plain");
}

// ORACLE markdown_render_tests.rs:657 list_ordered_custom_start kind=ui owner=chat-live
#[test]
fn list_ordered_custom_start() {
    let s = chat(80, 24, "3. First\n4. Second\n");
    s.assert_in_order(&["3. First", "4. Second"]);
    s.assert_text_style("3. ", "fg=LightBlue");
    s.assert_text_style("4. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:667 nested_unordered_in_ordered kind=ui owner=chat-live
#[test]
fn nested_unordered_in_ordered() {
    kit::pending(
        Owner::ChatLive,
        "G64: MarkdownView indents a nested list by its parent's marker (2 columns), the oracle by 4 (flutter_markdown listIndent)",
    );
    let s = chat(80, 24, "1. Outer\n    - Inner A\n    - Inner B\n2. Next\n");
    s.assert_in_order(&["1. Outer", "    • Inner A", "    • Inner B", "2. Next"]);
}

// ORACLE markdown_render_tests.rs:682 nested_ordered_in_unordered kind=ui owner=chat-live
#[test]
fn nested_ordered_in_unordered() {
    kit::pending(
        Owner::ChatLive,
        "G64: MarkdownView indents a nested list by its parent's marker (2 columns), the oracle by 4 (flutter_markdown listIndent)",
    );
    let s = chat(80, 24, "- Outer\n    1. One\n    2. Two\n- Last\n");
    s.assert_in_order(&["• Outer", "    1. One", "    2. Two", "• Last"]);
}
