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
/// ~If the parent is not wide enough to fit all widgets, the parent will be expanded to the right.~
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
    kind: SidesKind,
    wrap_mode: Option<crate::TextWrapMode>,
}

#[derive(Clone, Copy, Debug, Default)]
enum SidesKind {
    #[default]
    Extend,
    ShrinkLeft,
    ShrinkRight,
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

    pub fn shrink_left(mut self) -> Self {
        self.kind = SidesKind::ShrinkLeft;
        self
    }

    pub fn shrink_right(mut self) -> Self {
        self.kind = SidesKind::ShrinkRight;
        self
    }

    pub fn extend(mut self) -> Self {
        self.kind = SidesKind::Extend;
        self
    }

    /// The text wrap mode for the shrinking side.
    ///
    /// Does nothing if the kind is [`SidesKind::Extend`].
    pub fn wrap_mode(mut self, wrap_mode: crate::TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Truncate the text on the shrinking side.
    ///
    /// This is a shortcut for [`Self::wrap_mode`].
    /// Does nothing if the kind is [`SidesKind::Extend`].
    pub fn truncate(mut self) -> Self {
        self.wrap_mode = Some(crate::TextWrapMode::Truncate);
        self
    }

    /// Wrap the text on the shrinking side.
    ///
    /// This is a shortcut for [`Self::wrap_mode`].
    /// Does nothing if the kind is [`SidesKind::Extend`].
    pub fn wrap(mut self) -> Self {
        self.wrap_mode = Some(crate::TextWrapMode::Wrap);
        self
    }

    pub fn show<RetL, RetR>(
        self,
        ui: &mut Ui,
        add_left: impl FnOnce(&mut Ui) -> RetL,
        add_right: impl FnOnce(&mut Ui) -> RetR,
    ) -> (RetL, RetR) {
        let Self {
            height,
            spacing,
            mut kind,
            mut wrap_mode,
        } = self;
        let height = height.unwrap_or_else(|| ui.spacing().interact_size.y);
        let spacing = spacing.unwrap_or_else(|| ui.spacing().item_spacing.x);

        let mut top_rect = ui.available_rect_before_wrap();
        top_rect.max.y = top_rect.min.y + height;

        let result_left;
        let result_right;

        if ui.is_sizing_pass() {
            kind = SidesKind::Extend;
            wrap_mode = None;
        }

        match kind {
            SidesKind::ShrinkLeft => {
                // Draw right side first, then limit left side width
                let right_rect = {
                    let right_max_rect = top_rect;
                    let mut right_ui = ui.new_child(
                        UiBuilder::new()
                            .max_rect(right_max_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    result_right = add_right(&mut right_ui);
                    right_ui.min_rect()
                };

                let left_rect = {
                    let available_width = top_rect.width() - right_rect.width() - spacing;
                    let left_max_rect =
                        top_rect.with_max_x(top_rect.min.x + available_width.max(0.0));
                    let mut left_ui = ui.new_child(
                        UiBuilder::new()
                            .max_rect(left_max_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    if let Some(wrap_mode) = wrap_mode {
                        left_ui.style_mut().wrap_mode = Some(wrap_mode);
                    }
                    result_left = add_left(&mut left_ui);
                    left_ui.min_rect()
                };

                let final_rect = left_rect.union(right_rect);
                ui.advance_cursor_after_rect(final_rect);
            }
            SidesKind::ShrinkRight => {
                // Draw left side first, then limit right side width
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
                    let available_width = top_rect.width() - left_rect.width() - spacing;
                    let right_max_rect = top_rect
                        .with_min_x(left_rect.max.x + spacing)
                        .with_max_x(top_rect.max.x);
                    let right_max_rect =
                        right_max_rect.with_max_x(right_max_rect.min.x + available_width.max(0.0));
                    let mut right_ui = ui.new_child(
                        UiBuilder::new()
                            .max_rect(right_max_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    if let Some(wrap_mode) = wrap_mode {
                        right_ui.style_mut().wrap_mode = Some(wrap_mode);
                    }
                    result_right = add_right(&mut right_ui);
                    right_ui.min_rect()
                };

                let final_rect = left_rect.union(right_rect);
                ui.advance_cursor_after_rect(final_rect);
            }
            SidesKind::Extend => {
                // Original behavior: left first, then right, then final_rect calculations
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
            }
        }

        (result_left, result_right)
    }
}
