use epaint::{
    emath::{lerp, Align},
    pos2, vec2,
};

use crate::{egui_assert, Layout, Response, Sense, Ui};

/// A splitter which can separate the UI into 2 parts either vertically or horizontally.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Splitter::vertical().show(ui |ui_left, ui_right| {
///     ui_left.label("I'm on the left!");
///     ui_right.label("I'm on the right!");
/// })
/// # });
/// ```
#[must_use = "You should call .show()"]
pub struct Splitter {
    orientation: SplitterOrientation,
    ratio: f32,
}

impl Splitter {
    /// Create a new splitter with the given orientation and a ratio of 0.5.
    pub fn with_orientation(orientation: SplitterOrientation) -> Self {
        Self {
            orientation,
            ratio: 0.5,
        }
    }

    /// Create a new vertical splitter with a ratio of 0.5.
    #[inline]
    pub fn vertical() -> Self {
        Self::with_orientation(SplitterOrientation::Vertical)
    }

    /// Create a new horizontal splitter with a ratio of 0.5.
    #[inline]
    pub fn horizontal() -> Self {
        Self::with_orientation(SplitterOrientation::Horizontal)
    }

    /// Set the ratio of the splitter.
    ///
    /// The ratio sets where the splitter splits the current UI, where, depending on the
    /// orientation, 0.0 would mean split at the very top/left and 1.0 would mean split at the very
    /// bottom/right respectively. The ratio must be in the range 0.0..=1.0.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    #[inline]
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui, &mut Ui) -> R,
    ) -> SplitterResponse<R> {
        self.show_dyn(ui, Box::new(add_contents))
    }

    pub fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui, &mut Ui) -> R + 'c>,
    ) -> SplitterResponse<R> {
        let Self { orientation, ratio } = self;

        egui_assert!((0.0..=1.0).contains(&ratio));

        let (rect, splitter_response) =
            ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());

        let line_pos_1 = match orientation {
            SplitterOrientation::Vertical => pos2(lerp(rect.min.x..=rect.max.x, ratio), rect.min.y),
            SplitterOrientation::Horizontal => {
                pos2(rect.min.x, lerp(rect.min.y..=rect.max.y, ratio))
            }
        };

        let line_pos_2 = match orientation {
            SplitterOrientation::Vertical => line_pos_1 + vec2(0.0, rect.height()),
            SplitterOrientation::Horizontal => line_pos_1 + vec2(rect.width(), 0.0),
        };

        let line_pos_1 = ui.painter().round_pos_to_pixels(line_pos_1);
        let line_pos_2 = ui.painter().round_pos_to_pixels(line_pos_2);

        ui.painter().line_segment(
            [line_pos_1, line_pos_2],
            ui.visuals().widgets.noninteractive.bg_stroke,
        );

        let top_left_rect = match orientation {
            SplitterOrientation::Vertical => {
                let mut rect = rect;
                rect.max.x = line_pos_1.x - ui.style().spacing.item_spacing.x;
                rect
            }
            SplitterOrientation::Horizontal => {
                let mut rect = rect;
                rect.max.y = line_pos_1.y - ui.style().spacing.item_spacing.y;
                rect
            }
        };

        let bottom_right_rect = match orientation {
            SplitterOrientation::Vertical => {
                let mut rect = rect;
                rect.min.x = line_pos_1.x + ui.style().spacing.item_spacing.x;
                rect
            }
            SplitterOrientation::Horizontal => {
                let mut rect = rect;
                rect.min.y = line_pos_1.y + ui.style().spacing.item_spacing.y;
                rect
            }
        };

        let mut top_left_ui = ui.child_ui(top_left_rect, Layout::top_down(Align::Min));
        let mut bottom_right_ui = ui.child_ui(bottom_right_rect, Layout::top_down(Align::Min));

        let body_returned = add_contents(&mut top_left_ui, &mut bottom_right_ui);

        SplitterResponse {
            splitter_response,
            body_returned,
            top_left_response: ui.interact(top_left_rect, top_left_ui.id(), Sense::hover()),
            bottom_right_response: ui.interact(
                bottom_right_rect,
                bottom_right_ui.id(),
                Sense::hover(),
            ),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SplitterOrientation {
    Horizontal,
    Vertical,
}

/// The response of showing a Splitter
pub struct SplitterResponse<R> {
    /// The return value of the closure passed into show.
    pub body_returned: R,
    /// The response of the top or left UI depending on the splitter's orientation.
    pub top_left_response: Response,
    /// The response of the bottom or right UI depending on the splitter's orientation.
    pub bottom_right_response: Response,
    /// The response of the whole splitter widget.
    pub splitter_response: Response,
}
