<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Screen readers

[All widgets](../widgets.md)

## Semantics

What a widget is to a screen reader: a role, a label, a value and a state, read as a line of text. TWK_SCREEN_READER=1 reads an app out so.

API:

- `rtwk::semantics::Semantics` (struct, `crates/rtwk-widgets/src/semantics/widgets.rs:75`) Gives what `child` shows a node of the semantics (`SemanticsNode::of`): a `role`, a `label`, a `value`, a `state` and a `hint` (twk's `Semantics`).
  - public fields: `child: AnyWidget`, `role: Option<SemanticsRole>`, `label: Option<Str>`, `value: Option<Str>`, `value_fn: Option<ValueFn>`, `state: SemanticsState`, `hint: Option<Str>`, `nodes: Option<Rc<[SemanticsNode]>>`

### Code

The gallery's Code tab at the initial options (route `#/screen-readers/semantics`, `crates/rtwk-gallery/src/pages/semantics_debug.rs:30`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| view | beside, reader | beside |
| block | on, off | true |
| label | on, off | true |
| exclude | on, off | false |

### Methods

#### `Semantics`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Semantics` - Gives what `child` shows a node of no role, label, value, state or hint: the builder methods give them.

#### `Semantics`: builder methods (take and return `self`)

- `pub fn role(mut self, role: SemanticsRole) -> Semantics` - Of `role`.
- `pub fn label(mut self, label: impl Into<Str>) -> Semantics` - Called `label`.
- `pub fn value(mut self, value: impl Into<Str>) -> Semantics` - Holding `value`.
- `pub fn value_with(mut self, value: ValueFn) -> Semantics` - Holding what `value` answers when the semantics are read (a field's text), in place of `value`.
- `pub fn state(mut self, state: SemanticsState) -> Semantics` - In `state`.
- `pub fn hint(mut self, hint: impl Into<Str>) -> Semantics` - With `hint`, read last.
- `pub fn nodes(mut self, nodes: impl Into<Rc<[SemanticsNode]>>) -> Semantics` - Read as `nodes`, in place of what is below it.

### Also used in the code

- `rtwk::semantics::ExcludeSemantics` (struct, `crates/rtwk-widgets/src/semantics/widgets.rs:277`) Leaves out of the semantics what is below it while `excluding` (`SemanticsNode::of`): its `Semantics` and the texts of its `Text`s, as though it were not there, the label a `Semantics` above takes of the texts below it ...
  - public fields: `child: AnyWidget`, `excluding: bool`
- `rtwk::semantics::SemanticsRole` (enum, `crates/rtwk-widgets/src/semantics/role.rs:16`) What a node of the semantics is, as its line says it (`word`): ARIA's roles (twk's `SemanticsRole`).
  - variants: `Alert`, `Button`, `Checkbox`, `Dialog`, `Heading`, `Link`, `List`, `ListItem`, `Menu`, `MenuBar`, `MenuItem`, `ProgressBar`, `Radio`, `Slider`, `Status`, `Tab`, `TabList`, `TextBox`, `Toggle`
- `rtwk::animation::ProgressBar` (struct, `crates/rtwk-animation/src/progress_bar.rs:43`) A bar filled to `value`, from 0 (empty) to 1 (full), in eighths of a cell — braille halves where block elements are two columns wide, the theme's shades where its glyphs have no partial blocks — as wide as it may be and ...
  - public fields: `value: Option<f64>`, `style: Option<Style>`, `semantics_label: Option<Str>`
- `rtwk::semantics::BlockSemantics` (struct, `crates/rtwk-widgets/src/semantics/widgets.rs:225`) Leaves out of the semantics the nodes read before `child` in the node it is in (`SemanticsNode::of`): what is under it, as under a modal barrier (twk's `BlockSemantics`).
  - public fields: `child: AnyWidget`, `blocking: bool`
