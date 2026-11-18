<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Screen readers

What the widgets are to a screen reader.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Semantics

What a widget is to a screen reader: a role, a label, a value and a state, read as a line of text. TWK_SCREEN_READER=1 reads an app out so.

- `rtwk::semantics::Semantics` (struct) in `crates/rtwk-widgets/src/semantics/widgets.rs:75`: Gives what child shows a node of the semantics (SemanticsNode::of): a role, a label, a value, a state and a hint (twk's `Semantics`).

### Code

```rust
use rtwk::animation::ProgressBar;
use rtwk::semantics::{BlockSemantics, ExcludeSemantics, Semantics, SemanticsRole};
use rtwk::text::Text;
use rtwk::{AnyWidget, column};

pub fn build() -> AnyWidget {
    let exclude_heading = false;
    let block_covered_text = true;
    let label_progress = true;

    let heading =
        ExcludeSemantics::new(Semantics::new(Text::new("Sign in")).role(SemanticsRole::Heading))
            .excluding(exclude_heading);

    let progress = ProgressBar::new(Some(0.4));
    let progress = if label_progress {
        progress.semantics_label("Upload")
    } else {
        progress
    };

    let notice = Text::new("the notice on top");
    let notice = if block_covered_text {
        AnyWidget::from(BlockSemantics::new(notice))
    } else {
        notice.into()
    };
    let popup = Semantics::new(column![Text::new("the page under it"), notice])
        .role(SemanticsRole::Dialog)
        .label("Popup");

    column![heading, progress, popup].into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| view | beside the form, TWK_SCREEN_READER=1 | beside the form |
| BlockSemantics | false, true | true |
| progress label | false, true | true |
| ExcludeSemantics | false, true | false |
