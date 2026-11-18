<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Images

PNG and JPEG pictures, in the terminal's graphics or in block characters.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Image

A PNG or JPEG fitted to its cells. Where the terminal has a graphics protocol (kitty, sixel, iTerm2) its pixels are placed over the cells; where it has none, or with forceBlocks, as here, it is drawn in half blocks or quadrants.

- `rtwk_rich::image::Image` (struct) in `crates/rtwk-rich/src/image/widget.rs:144`: An image in a box of cells, shown with the terminal's graphics protocol (rtwk_core::graphics::GraphicsCaps::protocol: kitty, sixel or iTerm2, from the GraphicsScope above it) and drawn in block characters (block_mode) where it has none, or always with force_blocks.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| picture | lake.jpg, logo.png | lake.jpg |
| fit | contain, cover, fill, scaleDown | contain |
| alignment | center, topLeft, centerLeft, centerRight, bottomRight | center |
| blocks | half, quadrant | half |
| width | 8 cells, 9 cells, 10 cells, 11 cells, 12 cells, 13 cells, 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells, 51 cells, 52 cells, 53 cells, 54 cells, 55 cells, 56 cells, 57 cells, 58 cells, 59 cells, 60 cells, 61 cells, 62 cells, 63 cells, 64 cells, 65 cells, 66 cells, 67 cells, 68 cells, 69 cells, 70 cells, 71 cells, 72 cells, 73 cells, 74 cells, 75 cells, 76 cells, 77 cells, 78 cells, 79 cells, 80 cells | 44 cells |
| height | 3 rows, 4 rows, 5 rows, 6 rows, 7 rows, 8 rows, 9 rows, 10 rows, 11 rows, 12 rows, 13 rows, 14 rows, 15 rows, 16 rows, 17 rows, 18 rows, 19 rows, 20 rows, 21 rows, 22 rows, 23 rows, 24 rows, 25 rows, 26 rows, 27 rows, 28 rows, 29 rows, 30 rows | 12 rows |
