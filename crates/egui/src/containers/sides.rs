use emath::Align;

use crate::{Layout, Ui, UiBuilder};

/// Put some widgets on the left and right sides of a ui.
///
/// The result will look like this:
/// ```text
///                        parent Ui
///  ______________________________________________________
/// |                    |           |                     |  ^
/// | -> left widgets -> |    gap    | <- right widgets <- |  | height
/// |____________________|           |_____________________|  v
/// |                                                      |
/// |                                                      |
/// ```
///
/// The width of the gap is dynamic, based on the max width of the parent [`Ui`].
/// When the parent is being auto-sized ([`Ui::is_sizing_pass`]) the gap will be as small as possible.
///
/// If the parent is not wide enough to fit all widgets, the parent will be expanded to the right.
///
/// The left widgets are first added to the ui, left-to-right.
/// Then the right widgets are added, right-to-left.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::containers::Sides::new().show(ui,
///     |ui| {
///         ui.label("Left");
///     },
///     |ui| {
///         ui.label("Right");
///     }
/// );
/// # });
/// ```
#[must_use = "You should call sides.show()"]
#[derive(Clone, Copy, Debug, Default)]
pub struct Sides {
    height: Option<f32>,
    spacing: Option<f32>,
}

impl Sides {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// The minimum height of the sides.
    ///
    /// The content will be centered vertically within this height.
    /// The default height is [`crate::Spacing::interact_size`]`.y`.
    #[inline]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The horizontal spacing between the left and right UIs.
    ///
    /// This is the minimum gap.
    /// The default is [`crate::Spacing::item_spacing`]`.x`.
    #[inline]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }

    pub fn show<RetL, RetR>(
        self,
        ui: &mut Ui,
        add_left: impl FnOnce(&mut Ui) -> RetL,
        add_right: impl FnOnce(&mut Ui) -> RetR,
    ) -> (RetL, RetR) {
        let Self { height, spacing } = self;
        let height = height.unwrap_or_else(|| ui.spacing().interact_size.y);
        let spacing = spacing.unwrap_or_else(|| ui.spacing().item_spacing.x);

        let mut top_rect = ui.available_rect_before_wrap();
        top_rect.max.y = top_rect.min.y + height;

        let result_left;
        let result_right;

        let left_rect = {
            let left_max_rect = top_rect;
            let mut left_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(left_max_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            result_left = add_left(&mut left_ui);
            left_ui.min_rect()
        };

        let right_rect = {
            let right_max_rect = top_rect.with_min_x(left_rect.max.x);
            let mut right_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(right_max_rect)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            result_right = add_right(&mut right_ui);
            right_ui.min_rect()
        };

        let mut final_rect = left_rect.union(right_rect);
        let min_width = left_rect.width() + spacing + right_rect.width();

        if ui.is_sizing_pass() {
            // Make as small as possible:
            final_rect.max.x = left_rect.min.x + min_width;
        } else {
            // If the rects overlap, make sure we expand the allocated rect so that the parent
            // ui knows we overflowed, and resizes:
            final_rect.max.x = final_rect.max.x.max(left_rect.min.x + min_width);
        }

        ui.advance_cursor_after_rect(final_rect);

        (result_left, result_right)
    }
}
