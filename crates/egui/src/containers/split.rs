use emath::{Align, NumExt as _, Rangef, Rect};

use crate::{Layout, Ui, UiBuilder};

/// Put some widgets at the start and some at the end of a ui, with a gap in between.
///
/// [`Split::horizontal`] (the default) puts the start widgets on the left and the end widgets on the right:
/// ```text
///                        parent Ui
///  ______________________________________________________
/// |                    |           |                     |  ^
/// | -> start widgets ->|    gap    |<- end widgets <-    |  | cross size
/// |____________________|           |_____________________|  v
/// |                                                      |
/// |                                                      |
/// ```
///
/// [`Split::vertical`] puts the start widgets at the top and the end widgets at the bottom.
///
/// The size of the gap is dynamic, based on the max size of the parent [`Ui`] along the main axis.
/// When the parent is being auto-sized ([`Ui::is_sizing_pass`]) the gap will be as small as possible.
///
/// If the parent is too small to fit all widgets, the parent will be expanded along the main axis.
///
/// The start widgets are added left-to-right (or top-down).
/// The end widgets are added right-to-left (or bottom-up).
///
/// Which side is first depends on the configuration:
///  - [`Split::extend`] - start widgets are added first
///  - [`Split::shrink_start`] - end widgets are added first
///  - [`Split::shrink_end`] - start widgets are added first
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::containers::Split::new().show(ui,
///     |ui| {
///         ui.label("Left");
///     },
///     |ui| {
///         ui.label("Right");
///     }
/// );
/// # });
/// ```
#[must_use = "You should call split.show()"]
#[derive(Clone, Copy, Debug, Default)]
pub struct Split {
    vertical: bool,
    cross_size: Option<f32>,
    spacing: Option<f32>,
    kind: SplitKind,
    wrap_mode: Option<crate::TextWrapMode>,
}

/// Renamed to [`Split`].
#[deprecated = "Renamed to `Split`"]
pub type Sides = Split;

#[derive(Clone, Copy, Debug, Default)]
enum SplitKind {
    #[default]
    Extend,
    ShrinkStart,
    ShrinkEnd,
}

impl Split {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Start widgets on the left, end widgets on the right.
    ///
    /// This is the default.
    #[inline]
    pub fn horizontal() -> Self {
        Self::default()
    }

    /// Start widgets at the top, end widgets at the bottom.
    ///
    /// The parent needs a finite available height,
    /// so don't use this in a vertical [`crate::ScrollArea`].
    #[inline]
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            ..Self::default()
        }
    }

    /// The minimum size of the split across the main axis.
    ///
    /// For a horizontal split, this is the height, and the content is centered vertically within it.
    /// The default is [`crate::Spacing::interact_size`]`.y`.
    ///
    /// For a vertical split, this is the width, and the content is left-aligned within it.
    /// The default is the available width.
    #[inline]
    pub fn cross_size(mut self, cross_size: f32) -> Self {
        self.cross_size = Some(cross_size);
        self
    }

    #[deprecated = "Renamed to `cross_size`"]
    #[inline]
    pub fn height(self, height: f32) -> Self {
        self.cross_size(height)
    }

    /// The spacing between the start and end UIs.
    ///
    /// This is the minimum gap.
    /// The default is [`crate::Spacing::item_spacing`] along the main axis.
    #[inline]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Try to shrink widgets at the start.
    ///
    /// End widgets will be added first. The start [`Ui`]s max rect will be limited to the
    /// remaining space.
    #[inline]
    pub fn shrink_start(mut self) -> Self {
        self.kind = SplitKind::ShrinkStart;
        self
    }

    /// Try to shrink widgets at the end.
    ///
    /// Start widgets will be added first. The end [`Ui`]s max rect will be limited to the
    /// remaining space.
    #[inline]
    pub fn shrink_end(mut self) -> Self {
        self.kind = SplitKind::ShrinkEnd;
        self
    }

    #[deprecated = "Renamed to `shrink_start`"]
    #[inline]
    pub fn shrink_left(self) -> Self {
        self.shrink_start()
    }

    #[deprecated = "Renamed to `shrink_end`"]
    #[inline]
    pub fn shrink_right(self) -> Self {
        self.shrink_end()
    }

    /// Extend the start and end UIs to fill the available space.
    ///
    /// This is the default behavior.
    /// The start widgets will be added first, followed by the end widgets.
    #[inline]
    pub fn extend(mut self) -> Self {
        self.kind = SplitKind::Extend;
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

    pub fn show<RetS, RetE>(
        self,
        ui: &mut Ui,
        add_start: impl FnOnce(&mut Ui) -> RetS,
        add_end: impl FnOnce(&mut Ui) -> RetE,
    ) -> (RetS, RetE) {
        let Self {
            vertical,
            cross_size,
            spacing,
            mut kind,
            mut wrap_mode,
        } = self;

        let mut full_rect = ui.available_rect_before_wrap();
        let (start_layout, end_layout) = if vertical {
            if let Some(width) = cross_size {
                full_rect.max.x = full_rect.min.x + width;
            }
            (
                Layout::top_down(Align::Min),
                Layout::bottom_up(Align::Min),
            )
        } else {
            let height = cross_size.unwrap_or_else(|| ui.spacing().interact_size.y);
            full_rect.max.y = full_rect.min.y + height;
            (
                Layout::left_to_right(Align::Center),
                Layout::right_to_left(Align::Center),
            )
        };
        let spacing = spacing.unwrap_or_else(|| {
            let item_spacing = ui.spacing().item_spacing;
            if vertical {
                item_spacing.y
            } else {
                item_spacing.x
            }
        });

        let main = |rect: Rect| main_range(vertical, rect);
        let with_main = |rect: Rect, range: Rangef| with_main_range(vertical, rect, range);
        let full = main(full_rect);

        if ui.is_sizing_pass() {
            kind = SplitKind::Extend;
            wrap_mode = None;
        }

        match kind {
            SplitKind::ShrinkStart => {
                let (end_rect, result_end) =
                    Self::create_ui(ui, full_rect, end_layout, add_end, None);
                let available = full.span() - main(end_rect).span() - spacing;
                let start_rect_constraint = with_main(
                    full_rect,
                    Rangef::new(full.min, full.min + available.at_least(0.0)),
                );
                let (start_rect, result_start) = Self::create_ui(
                    ui,
                    start_rect_constraint,
                    start_layout,
                    add_start,
                    wrap_mode,
                );

                ui.advance_cursor_after_rect(start_rect | end_rect);
                (result_start, result_end)
            }
            SplitKind::ShrinkEnd => {
                let (start_rect, result_start) =
                    Self::create_ui(ui, full_rect, start_layout, add_start, None);
                let end_rect_constraint = with_main(
                    full_rect,
                    Rangef::new(main(start_rect).max + spacing, full.max),
                );
                let (end_rect, result_end) =
                    Self::create_ui(ui, end_rect_constraint, end_layout, add_end, wrap_mode);

                ui.advance_cursor_after_rect(start_rect | end_rect);
                (result_start, result_end)
            }
            SplitKind::Extend => {
                let (start_rect, result_start) =
                    Self::create_ui(ui, full_rect, start_layout, add_start, None);
                let end_max_rect =
                    with_main(full_rect, Rangef::new(main(start_rect).max, full.max));
                let (end_rect, result_end) =
                    Self::create_ui(ui, end_max_rect, end_layout, add_end, None);

                let final_rect = start_rect | end_rect;
                let mut final_range = main(final_rect);
                let min_size = main(start_rect).span() + spacing + main(end_rect).span();
                let min_end = main(start_rect).min + min_size;

                if ui.is_sizing_pass() {
                    final_range.max = min_end;
                } else {
                    final_range.max = final_range.max.max(min_end);
                }

                ui.advance_cursor_after_rect(with_main(final_rect, final_range));
                (result_start, result_end)
            }
        }
    }

    fn create_ui<Ret>(
        ui: &mut Ui,
        max_rect: Rect,
        layout: Layout,
        add_content: impl FnOnce(&mut Ui) -> Ret,
        wrap_mode: Option<crate::TextWrapMode>,
    ) -> (Rect, Ret) {
        let mut child_ui = ui.new_child(UiBuilder::new().max_rect(max_rect).layout(layout));
        if let Some(wrap_mode) = wrap_mode {
            child_ui.style_mut().wrap_mode = Some(wrap_mode);
        }
        let result = add_content(&mut child_ui);
        (child_ui.min_rect(), result)
    }
}

/// The range of `rect` along the main axis.
fn main_range(vertical: bool, rect: Rect) -> Rangef {
    if vertical {
        rect.y_range()
    } else {
        rect.x_range()
    }
}

/// `rect` with its range along the main axis replaced.
fn with_main_range(vertical: bool, rect: Rect, range: Rangef) -> Rect {
    if vertical {
        Rect::from_x_y_ranges(rect.x_range(), range)
    } else {
        Rect::from_x_y_ranges(range, rect.y_range())
    }
}
