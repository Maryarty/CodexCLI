//! The `[tui]` settings a screen test turns on after the screen is open (the host sends them
//! once at the start: `AppEv::Tui`).

use std::sync::Arc;

use codex_tui_core::app::effects::{Ask, Done, ImagePreview, ImageSize};
use codex_tui_core::app::facts::{Rendering, TuiSettings};
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::screens::pets::{
    Ambient, Anchor, AssetAsk, AssetDone, Clip, ClipTransition, Motion, Pet, Picker, Unavailable,
};
use codex_tui_rtwk::app::UiApp;

use super::Screen;

/// The `tui.rendering` switches a test names: each Markdown renderer on (the default) or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Renderers {
    pub mermaid: bool,
    pub math: bool,
    pub tables: bool,
    pub lists: bool,
}

impl Default for Renderers {
    fn default() -> Self {
        Self {
            mermaid: true,
            math: true,
            tables: true,
            lists: true,
        }
    }
}

impl Screen {
    /// The local host's catalog for the first supported `/pets` visit.
    pub fn pets_catalog(&mut self) {
        let fixture = include_str!("../../src/sheets/pets/fixtures/picker.jsonl");
        let row = fixture.lines().last().expect("pet catalog fixture");
        let seed: serde_json::Value = serde_json::from_str(row).expect("pet catalog seed");
        let pets: Vec<Pet> = serde_json::from_value(seed["ev"]["pets"].clone()).expect("pet rows");
        self.pet_answer(
            |ask| matches!(ask, AssetAsk::Catalog { graphics: true }),
            AssetDone::Catalog(Ok(Picker::build(None, pets))),
        );
    }

    /// The catalog cannot be opened in this terminal. The fake still sees the real catalog ask.
    pub fn pets_unavailable(&mut self, terminal: &str) {
        let reason = match terminal {
            "tmux" => Unavailable::Tmux,
            "iTerm2" => Unavailable::OldIterm,
            "terminal" => Unavailable::Terminal,
            other => panic!("unknown pet terminal fixture: {other}"),
        };
        self.pet_answer(
            |ask| matches!(ask, AssetAsk::Catalog { graphics: false }),
            AssetDone::Catalog(Err(reason)),
        );
    }

    /// Return a real static PNG to the preview/selection read the app asked for.
    pub fn pets_preview(&mut self, selector: &str) {
        let file = super::image_file_sized("pet-preview", 192, 208);
        let png = std::fs::read(file).expect("pet preview PNG");
        self.pet_answer(
            |ask| matches!(ask, AssetAsk::Preview { selector: id } if id == selector),
            AssetDone::Preview(Ok(ImagePreview {
                png: png.into(),
                size: ImageSize {
                    width: 192,
                    height: 208,
                },
            })),
        );
    }

    pub fn pets_save_accepted(&mut self, selector: &str) {
        self.pet_answer(
            |ask| matches!(ask, AssetAsk::Save { selector: id } if id == selector),
            AssetDone::Saved(Ok(())),
        );
    }

    /// Resolve the ambient read from configuration or from a confirmed local selection.
    pub fn pets_ambient(&mut self, selector: Option<&str>, screen_bottom: bool) {
        let file = super::image_file_sized("ambient-pet", 192, 208);
        let png = std::fs::read(file).expect("ambient pet PNG");
        let ambient = Ambient {
            size: ImageSize {
                width: 192,
                height: 208,
            },
            anchor: if screen_bottom {
                Anchor::ScreenBottom
            } else {
                Anchor::Composer
            },
            clips: std::array::from_fn(|_| Clip {
                data: png.clone().into(),
                after: None,
            }),
        };
        self.pet_answer(
            |ask| matches!(ask, AssetAsk::Ambient { selector: id } if id.as_deref() == selector),
            AssetDone::Ambient(Some(Arc::new(ambient))),
        );
    }

    pub fn pets_animated_ambient(&mut self) {
        let data = pet_gif(&[30, 90, 150, 210], 125, true);
        self.pets_ambient_clips(std::array::from_fn(|_| Clip {
            data: data.clone(),
            after: None,
        }));
    }

    pub fn pets_running_fallback(&mut self) {
        let file = super::image_file_sized("ambient-pet-idle", 192, 208);
        let still: Arc<[u8]> = std::fs::read(file).expect("ambient pet PNG").into();
        let waiting = Arc::new(Clip {
            data: pet_gif(&[48, 49], 125, true),
            after: None,
        });
        self.pets_ambient_clips(Motion::ALL.map(|motion| match motion {
            Motion::Running => Clip {
                data: pet_gif(&[56, 57], 125, false),
                after: Some(ClipTransition {
                    delay: crate::common::ms(250),
                    next: waiting.clone(),
                }),
            },
            Motion::Waiting => waiting.as_ref().clone(),
            _ => Clip {
                data: still.clone(),
                after: None,
            },
        }));
    }

    fn pets_ambient_clips(&mut self, clips: [Clip; 5]) {
        self.pet_answer(
            |ask| matches!(ask, AssetAsk::Ambient { selector: None }),
            AssetDone::Ambient(Some(Arc::new(Ambient {
                size: ImageSize {
                    width: 192,
                    height: 208,
                },
                anchor: Anchor::Composer,
                clips,
            }))),
        );
    }

    pub fn ambient_kitty_transfers(&mut self, millis: u64) -> usize {
        const TRANSMIT_AND_PLACE: &str = "\x1b_Ga=T";
        let before = self.h.t.written().matches(TRANSMIT_AND_PLACE).count();
        self.h.t.pump_for(crate::common::ms(millis));
        self.h.t.written().matches(TRANSMIT_AND_PLACE).count() - before
    }

    pub fn ambient_frame_counts(&mut self, millis: u64) -> (u64, u64, u64) {
        let sample = |this: &Self| {
            let loop_ = this.h.t.frame_loop();
            (
                loop_.frames_run(),
                loop_.stats().frames,
                loop_.stats().bytes,
            )
        };
        let before = sample(self);
        self.h.t.pump_for(crate::common::ms(millis));
        let after = sample(self);
        (after.0 - before.0, after.1 - before.1, after.2 - before.2)
    }

    pub fn assert_single_ambient_fetch(&self) {
        let asks = self
            .commands()
            .into_iter()
            .filter(|command| {
                matches!(
                    command,
                    Command::Effect {
                        ask: Ask::Pet(AssetAsk::Ambient { .. }),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(asks, 1);
    }

    /// Native terminal protocol placements are independent of text cells.
    pub fn assert_ambient_pet(&mut self, rect: Option<(usize, usize, usize, usize)>) {
        self.h.settle();
        let images = self.h.t.images();
        let got: Vec<_> = images
            .iter()
            .map(|image| (image.x, image.y, image.cols, image.rows))
            .collect();
        assert_eq!(
            got,
            rect.into_iter().collect::<Vec<_>>(),
            "{images:?}\n{}",
            self.text()
        );
    }

    pub fn assert_pet_saves(&mut self, selectors: &[&str]) {
        self.h.settle();
        let saves: Vec<_> = self
            .h
            .fake
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Effect {
                    ask: Ask::Pet(AssetAsk::Save { selector }),
                    ..
                } => Some(selector),
                _ => None,
            })
            .collect();
        assert_eq!(saves, selectors);
    }

    pub fn assert_pet_previews(&mut self, count: usize) {
        self.h.settle();
        let images = self.h.t.images();
        assert_eq!(images.len(), count, "{images:?}\n{}", self.text());
        assert!(images.iter().all(|image| image.rows <= 4), "{images:?}");
    }

    fn pet_answer(&mut self, wanted: impl Fn(&AssetAsk) -> bool, done: AssetDone) {
        let mut request = None;
        self.h
            .wait_until("an observed pet capability request", |h| {
                request = h
                    .fake
                    .commands()
                    .into_iter()
                    .rev()
                    .find_map(|command| match command {
                        Command::Effect {
                            request,
                            ask: Ask::Pet(ask),
                            ..
                        } if wanted(&ask) => Some(request),
                        _ => None,
                    });
                request.is_some()
            });
        self.h.inject([AppEv::Effect {
            request: request.expect("pet request"),
            done: Done::Pet(done),
        }]);
    }

    /// The host's explicit rows per native wheel event.
    pub fn mouse_scroll_speed(&mut self, speed: Option<f64>) -> &mut Screen {
        let settings = self.h.t.read_app(|app: &UiApp| app.core.tui());
        self.h.inject([AppEv::Tui(TuiSettings {
            wheel_rows_per_event: speed,
            ..settings
        })]);
        self
    }

    /// The clipboard policy for mouse selections.
    pub fn selection_copy_policy(&mut self, copy_on_select: bool, deselect_after_copy: bool) {
        let settings = self.h.t.read_app(|app: &UiApp| app.core.tui());
        self.h.inject([AppEv::Tui(TuiSettings {
            copy_on_select,
            deselect_after_copy,
            ..settings
        })]);
    }

    /// Seed the host's local X11 clipboard capability.
    pub fn primary_selection(&mut self) {
        let settings = self.h.t.read_app(|app: &UiApp| app.core.tui());
        self.h.inject([AppEv::Tui(TuiSettings {
            primary_selection: true,
            ..settings
        })]);
    }

    /// `tui.collapse_turn_details`: the tool calls of a finished turn fold into one row each
    /// run, and the messages before the answer behind the prompt.
    pub fn collapse_turn_details(&mut self) -> &mut Screen {
        self.h.inject([AppEv::Tui(TuiSettings {
            collapse_turn_details: true,
            ..TuiSettings::default()
        })]);
        self
    }

    /// `tui.rendering`: the Markdown renderers that are on (a table, list, mermaid or math
    /// renderer that is off keeps the source text).
    pub fn rendering(&mut self, rendering: Renderers) -> &mut Screen {
        let settings = self.h.t.read_app(|app: &UiApp| app.core.tui());
        self.h.inject([AppEv::Tui(TuiSettings {
            rendering: Rendering {
                mermaid: rendering.mermaid,
                math: rendering.math,
                tables: rendering.tables,
                lists: rendering.lists,
            },
            ..settings
        })]);
        self
    }
}

fn pet_gif(colors: &[u8], delay_ms: u32, repeat: bool) -> Arc<[u8]> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame, Rgba, RgbaImage};

    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        if repeat {
            encoder
                .set_repeat(Repeat::Infinite)
                .expect("repeat animation");
        }
        for &red in colors {
            let pixels = RgbaImage::from_pixel(9, 10, Rgba([red, 0, 255, 255]));
            encoder
                .encode_frame(Frame::from_parts(
                    pixels,
                    0,
                    0,
                    Delay::from_numer_denom_ms(delay_ms, 1),
                ))
                .expect("encode test animation frame");
        }
    }
    bytes.into()
}
