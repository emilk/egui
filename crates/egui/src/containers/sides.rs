use emath::{Align, NumExt as _};

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
/// The two sides are laid out in reading order: the left widgets occupy the reading-start edge
/// and the right widgets the reading-end edge, so in a right-to-left layout (see
/// [`crate::Style::right_to_left`]) the left widgets appear on the physical right and the right
/// widgets on the physical left, the same way [`crate::Grid`] cells are mirrored. Which side
/// shrinks stays tied to the reading order: [`Self::shrink_left`] always shrinks the
/// reading-start (left) content and [`Self::shrink_right`] the reading-end (right) content.
///
/// If the parent is not wide enough to fit all widgets, the parent will be expanded away from
/// the reading-start edge (to the right in an LTR layout, to the left in an RTL one).
///
/// The left widgets are added left-to-right (right-to-left in an RTL layout).
/// The right widgets are added right-to-left (left-to-right in an RTL layout).
///
/// Which side is first depends on the configuration:
///  - [`Sides::extend`] - left widgets are added first
///  - [`Sides::shrink_left`] - right widgets are added first
///  - [`Sides::shrink_right`] - left widgets are added first
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

    /// Try to shrink widgets on the left side.
    ///
    /// Right widgets will be added first. The left [`Ui`]s max rect will be limited to the
    /// remaining space.
    #[inline]
    pub fn shrink_left(mut self) -> Self {
        self.kind = SidesKind::ShrinkLeft;
        self
    }

    /// Try to shrink widgets on the right side.
    ///
    /// Left widgets will be added first. The right [`Ui`]s max rect will be limited to the
    /// remaining space.
    #[inline]
    pub fn shrink_right(mut self) -> Self {
        self.kind = SidesKind::ShrinkRight;
        self
    }

    /// Extend the left and right sides to fill the available space.
    ///
    /// This is the default behavior.
    /// The left widgets will be added first, followed by the right widgets.
    #[inline]
    pub fn extend(mut self) -> Self {
        self.kind = SidesKind::Extend;
        self
    }

    /// The text wrap mode for the shrinking side.
    ///
    /// Does nothing if [`Self::extend`] is used (the default).
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: crate::TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Truncate the text on the shrinking side.
    ///
    /// This is a shortcut for [`Self::wrap_mode`].
    /// Does nothing if [`Self::extend`] is used (the default).
    #[inline]
    pub fn truncate(mut self) -> Self {
        self.wrap_mode = Some(crate::TextWrapMode::Truncate);
        self
    }

    /// Wrap the text on the shrinking side.
    ///
    /// This is a shortcut for [`Self::wrap_mode`].
    /// Does nothing if [`Self::extend`] is used (the default).
    #[inline]
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

        // The two slots are reading-order roles: `add_left` is the reading-start side and
        // `add_right` the reading-end side, so in an RTL layout their flow and placement are
        // the point reflection of the LTR layout (the same convention `Grid` uses for its
        // cells):
        let rtl = ui.placer().prefer_right_to_left(ui.is_rtl());
        let start_layout = if rtl {
            Layout::right_to_left(Align::Center)
        } else {
            Layout::left_to_right(Align::Center)
        };
        let end_layout = if rtl {
            Layout::left_to_right(Align::Center)
        } else {
            Layout::right_to_left(Align::Center)
        };

        let mut top_rect = ui.available_rect_before_wrap();
        top_rect.max.y = top_rect.min.y + height;

        if ui.is_sizing_pass() {
            kind = SidesKind::Extend;
            wrap_mode = None;
        }

        match kind {
            SidesKind::ShrinkLeft => {
                let (right_rect, result_right) =
                    Self::create_ui(ui, top_rect, end_layout, add_right, None);
                let available_width = top_rect.width() - right_rect.width() - spacing;
                let left_rect_constraint = if rtl {
                    top_rect.with_min_x(top_rect.max.x - available_width.at_least(0.0))
                } else {
                    top_rect.with_max_x(top_rect.min.x + available_width.at_least(0.0))
                };
                let (left_rect, result_left) =
                    Self::create_ui(ui, left_rect_constraint, start_layout, add_left, wrap_mode);

                ui.advance_cursor_after_rect(left_rect | right_rect);
                (result_left, result_right)
            }
            SidesKind::ShrinkRight => {
                let (left_rect, result_left) =
                    Self::create_ui(ui, top_rect, start_layout, add_left, None);
                let right_rect_constraint = if rtl {
                    top_rect.with_max_x(left_rect.min.x - spacing)
                } else {
                    top_rect.with_min_x(left_rect.max.x + spacing)
                };
                let (right_rect, result_right) =
                    Self::create_ui(ui, right_rect_constraint, end_layout, add_right, wrap_mode);

                ui.advance_cursor_after_rect(left_rect | right_rect);
                (result_left, result_right)
            }
            SidesKind::Extend => {
                let (left_rect, result_left) =
                    Self::create_ui(ui, top_rect, start_layout, add_left, None);
                let right_max_rect = if rtl {
                    top_rect.with_max_x(left_rect.min.x)
                } else {
                    top_rect.with_min_x(left_rect.max.x)
                };
                let (right_rect, result_right) =
                    Self::create_ui(ui, right_max_rect, end_layout, add_right, None);

                let mut final_rect = left_rect | right_rect;
                let min_width = left_rect.width() + spacing + right_rect.width();

                if ui.is_sizing_pass() {
                    if rtl {
                        final_rect.min.x = left_rect.max.x - min_width;
                    } else {
                        final_rect.max.x = left_rect.min.x + min_width;
                    }
                } else if rtl {
                    final_rect.min.x = final_rect.min.x.min(left_rect.max.x - min_width);
                } else {
                    final_rect.max.x = final_rect.max.x.max(left_rect.min.x + min_width);
                }

                ui.advance_cursor_after_rect(final_rect);
                (result_left, result_right)
            }
        }
    }

    fn create_ui<Ret>(
        ui: &mut Ui,
        max_rect: emath::Rect,
        layout: Layout,
        add_content: impl FnOnce(&mut Ui) -> Ret,
        wrap_mode: Option<crate::TextWrapMode>,
    ) -> (emath::Rect, Ret) {
        let mut child_ui = ui.new_child(UiBuilder::new().max_rect(max_rect).layout(layout));
        if let Some(wrap_mode) = wrap_mode {
            child_ui.style_mut().wrap_mode = Some(wrap_mode);
        }
        let result = add_content(&mut child_ui);
        (child_ui.min_rect(), result)
    }
}
