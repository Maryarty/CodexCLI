<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Images

[All widgets](../widgets.md)

## Image

A PNG or JPEG fitted to its cells. Where the terminal has a graphics protocol (kitty, sixel, iTerm2) its pixels are placed over the cells; where it has none, or with forceBlocks, as here, it is drawn in half blocks or quadrants.

API:

- `rtwk_rich::image::Image` (struct, `crates/rtwk-rich/src/image/widget.rs:144`) An image in a box of cells, shown with the terminal's graphics protocol (`rtwk_core::graphics::GraphicsCaps::protocol`: kitty, sixel or iTerm2, from the `GraphicsScope` above it) and drawn in block characters ...
  - public fields: `source: ImageSource`, `width: Option<i32>`, `height: Option<i32>`, `fit: ImageFit`, `alignment: Alignment`, `block_mode: BlockMode`, `force_blocks: bool`
- `rtwk_rich::image::ImageFit` (enum, `crates/rtwk-rich/src/image/scale.rs:84`) How an image fills a box of cells.
  - variants: `Contain`, `Cover`, `Fill`, `ScaleDown`
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`

### Code

The gallery's Code tab at the initial options (route `#/images/image`, `crates/rtwk-gallery/src/pages/media_terminal.rs:76`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::layout::Alignment;
use rtwk_rich::image::{BlockMode, Image, ImageFit};

fn preview() -> Image {
    Image::file("assets/lake.jpg")
        .width(44)
        .height(12)
        .fit(ImageFit::Contain)
        .alignment(Alignment::CENTER)
        .block_mode(BlockMode::Half)
        .force_blocks(true)
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| picture | lake, logo | lake |
| fit | contain, cover, fill, scaleDown | contain |
| alignment | center, topLeft, centerLeft, centerRight, bottomRight | center |
| mode | half, quadrant | half |
| width | 8..80 (a slider) | 44 cells |
| height | 3..30 (a slider) | 12 rows |

### Methods

#### `Image`: constructors and associated functions

- `pub fn new(source: ImageSource) -> Image` - The image of `source`.
- `pub fn memory(bytes: impl Into<Arc<[u8]>>) -> Image` - The PNG or JPEG file `bytes`.
- `pub fn file(path: impl Into<PathBuf>) -> Image` - The PNG or JPEG file at `path`, read when first laid out.
- `pub fn bitmap(bitmap: impl Into<Arc<Bitmap>>) -> Image` - The pixels of `bitmap`.

#### `Image`: builder methods (take and return `self`)

- `pub fn width(mut self, width: i32) -> Image` - The box is `width` cells wide.
- `pub fn height(mut self, height: i32) -> Image` - The box is `height` cells tall.
- `pub fn fit(mut self, fit: ImageFit) -> Image` - The image fills the box as `fit` says.
- `pub fn alignment(mut self, alignment: Alignment) -> Image` - The image goes in the cells the box leaves as `alignment` says.
- `pub fn block_mode(mut self, mode: BlockMode) -> Image` - Block characters draw the image as `mode` says.
- `pub fn force_blocks(mut self, force: bool) -> Image` - Draws the image in block characters even where the terminal shows images.

#### `ImageFit`: builder methods (take and return `self`)

- `pub const fn name(self) -> &'static str` - Its name in twk (`contain`, `cover`, `fill`, `scaleDown`).

#### `Alignment`: constructors and associated functions

- `pub const fn new(x: i32, y: i32) -> Self` - Along the columns at `x` and along the rows at `y`, each -1 for the start, 0 for the middle or 1 for the end.

#### `Alignment`: methods

- `pub const fn dx(&self, free: i32) -> i32` - The columns left of a child when `free` columns are left over around it (fewer than 0 when the child is the wider one).
- `pub const fn dy(&self, free: i32) -> i32` - The rows above a child when `free` rows are left over around it.

### Also used in the code

- `rtwk_rich::image::BlockMode` (enum, `crates/rtwk-rich/src/image/blocks.rs:11`) How the cells of a `BlockImage` draw its pixels.
  - variants: `Half`, `Quadrant`
