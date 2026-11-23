use codex_tui_core::threads::stream_preview::*;

/// What `visible` shows after `text` arrived piece by piece.
fn after(pieces: &[&str]) -> String {
    let mut text = String::new();
    for piece in pieces {
        text.push_str(piece);
    }
    visible(&text).to_string()
}

/// `opener` arrives at once after `head` (as the oracle's own tests push it),
/// then every character of `tail`: the shown text stays `head`.
fn stays(head: &str, opener: &str, tail: &str) {
    let mut text = format!("{head}{opener}");
    assert_eq!(visible(&text), head, "after {text:?}");
    for character in tail.chars() {
        text.push(character);
        assert_eq!(visible(&text), head, "after {text:?}");
    }
}

#[test]
fn a_complete_line_shows_in_full_and_the_tail_of_prose_shows_as_it_grows() {
    assert_eq!(after(&["one\ntwo"]), "one\ntwo");
    assert_eq!(after(&["one\n"]), "one\n");
    assert_eq!(after(&["a plain line"]), "a plain line");
}

#[test]
fn an_unfinished_link_destination_waits_for_its_closing_parenthesis() {
    // oracle unfinished_link_destinations_do_not_expand_the_preview
    assert_eq!(
        after(&["See [documentation](https://example.com/unfinished"]),
        "See [documentation]"
    );
    assert_eq!(
        after(&["See [documentation](https://example.com/a)"]),
        "See [documentation](https://example.com/a)"
    );
    // ... and flushes with the newline.
    assert_eq!(
        after(&["See [documentation](https://example.com/a/very/long/path\n"]),
        "See [documentation](https://example.com/a/very/long/path\n"
    );
}

#[test]
fn a_destination_stays_hidden_through_nesting_escapes_titles_and_angle_brackets() {
    // oracle unfinished_link_destinations_do_not_expand_the_preview, every character split.
    for destination in [
        "https://example.com/a/very/long/path/to/the/guide",
        "https://example.com/a_(nested_(path))",
        r"https://example.com/escaped\)parenthesis",
        "<https://example.com/unbalanced)parenthesis>",
        " <https://example.com/unbalanced)parenthesis>",
        "https://example.com/guide \"a title with ) parentheses\"",
        "https://example.com/guide (x < y)",
    ] {
        let mut text = String::from("See [documentation]");
        for character in format!("({destination}").chars() {
            text.push(character);
            assert_eq!(
                visible(&text),
                "See [documentation]",
                "{destination}: {text:?}"
            );
        }
        text.push_str(") and continue.");
        assert_eq!(visible(&text), text, "{destination}");
    }
    // A destination far past any window stays held.
    let long = format!(
        "See [documentation](https://example.com/{}more-path",
        "界".repeat(3000)
    );
    assert_eq!(visible(&long), "See [documentation]");
}

#[test]
fn escapes_code_and_closed_links_show_as_written() {
    // oracle link_preview_respects_code_escapes_and_raw_mode (the rich half).
    for source in [
        r"See \[label](https://example.com/unfinished",
        r"See [label\](https://example.com/unfinished",
        "See `[label](https://example.com/unfinished`",
        "See ``[la`bel](https://example.com/unfinished``",
        "See [label](https://example.com/complete) and prose",
        r"See \:codex-followup[label]{prompt=unfinished",
        "See `:codex-followup[label]{prompt=unfinished`",
    ] {
        assert_eq!(visible(source), source, "{source}");
    }
}

#[test]
fn code_before_an_unfinished_link_shows_and_a_pipe_does_not_release_the_hold() {
    // oracle link_preview_respects_code_escapes_and_raw_mode (the code line) and
    // link_holdback_stays_rich_only_when_a_pipe_freezes_the_preview.
    assert_eq!(
        visible("See `[code](literal` then [label](https://example.com/unfinished"),
        "See `[code](literal` then [label]"
    );
    assert_eq!(visible("See [label](https://example.com"), "See [label]");
    assert_eq!(
        visible("See [label](https://example.com|suffix)"),
        "See [label]"
    );
}

#[test]
fn a_follow_up_directive_stays_hidden_until_it_closes() {
    // oracle followup_preview_hides_prompts_until_complete
    stays(
        "Before ",
        ":codex-followup[",
        r#"Inspect `items[0]`]{prompt="Use \\"}\" and `ticks`""#,
    );
    let done =
        r#"Before :codex-followup[Inspect `items[0]`]{prompt="Use \\"}\" and `ticks`"} after."#;
    assert_eq!(visible(done), done);
    // The oracle's second walk: a long prompt, then it closes.
    let long = format!(
        "Before :codex-followup[Long]{{prompt=\"{}more prompt",
        "界".repeat(3000)
    );
    assert_eq!(visible(&long), "Before ");
}

#[test]
fn follow_up_attribute_quotes_open_only_at_value_boundaries() {
    // oracle followup_preview_only_opens_quotes_at_value_boundaries
    for attributes in [
        "prompt=don't",
        "prompt=foo='bar",
        "prompt=don't other = \"quote } here\"",
        "prompt = 'quote } here'other=don't",
    ] {
        stays(
            "Before ",
            ":codex-followup[",
            &["Open]{", attributes].concat(),
        );
        let done = ["Before :codex-followup[Open]{", attributes, "} after."].concat();
        assert_eq!(visible(&done), done, "{attributes}");
    }
}

#[test]
fn surrounding_unfinished_markdown_keeps_its_text_and_hides_the_prompt() {
    // oracle followup_preview_handles_surrounding_unfinished_markdown
    stays(
        "[unfinished ",
        ":codex-followup[",
        "Inspect]{prompt=\"secret\"",
    );
    for source in [
        ":codex-followup[not a directive] continuing text",
        ":codex-followup[not a directive] `code`",
    ] {
        assert_eq!(visible(source), source, "{source}");
    }
}

#[test]
fn an_unmatched_code_opener_does_not_expose_a_prompt() {
    // oracle unmatched_code_openers_do_not_expose_followup_prompts
    assert_eq!(
        visible("Type ` then :codex-followup[Open]{prompt=secret}"),
        "Type ` then "
    );
    assert_eq!(
        visible("Type `x` then :codex-followup[Open]{prompt=secret}"),
        "Type `x` then :codex-followup[Open]{prompt=secret}"
    );
}

#[test]
fn a_tail_that_may_be_a_table_a_quote_or_code_waits_for_its_newline() {
    assert_eq!(after(&["| a | b |"]), "");
    assert_eq!(after(&["text | more"]), "text ");
    assert_eq!(after(&["    indented"]), "");
    assert_eq!(after(&["> quote"]), "");
    assert_eq!(after(&["```rust"]), "");
    assert_eq!(after(&["`"]), "");
    assert_eq!(after(&["`c"]), "`c");
    assert_eq!(after(&["done\n| a | b |"]), "done\n");
    assert_eq!(after(&["| a | b |\n"]), "| a | b |\n");
}
