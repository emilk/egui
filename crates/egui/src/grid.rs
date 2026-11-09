use alloc::sync::Arc;

use emath::GuiRounding as _;

use crate::{
    Align2, AsIdSalt, Color32, Context, Id, IdSalt, InnerResponse, NumExt as _, Painter, Rect,
    Region, Style, Ui, UiBuilder, Vec2, pos2, vec2,
};

#[cfg(debug_assertions)]
use crate::Stroke;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct State {
    col_widths: Vec<f32>,
    row_heights: Vec<f32>,
}

impl State {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_temp(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        // We don't persist Grids, because
        // A) there are potentially a lot of them, using up a lot of space (and therefore serialization time)
        // B) if the code changes, the grid _should_ change, and not remember old sizes
        ctx.data_mut(|d| d.insert_temp(id, self));
    }

    fn set_min_col_width(&mut self, col: usize, width: f32) {
        self.col_widths
            .resize(self.col_widths.len().max(col + 1), 0.0);
        self.col_widths[col] = self.col_widths[col].max(width);
    }

    fn set_min_row_height(&mut self, row: usize, height: f32) {
        self.row_heights
            .resize(self.row_heights.len().max(row + 1), 0.0);
        self.row_heights[row] = self.row_heights[row].max(height);
    }

    fn col_width(&self, col: usize) -> Option<f32> {
        self.col_widths.get(col).copied()
    }

    fn row_height(&self, row: usize) -> Option<f32> {
        self.row_heights.get(row).copied()
    }

    fn full_width(&self, x_spacing: f32) -> f32 {
        self.col_widths.iter().sum::<f32>()
            + (self.col_widths.len().at_least(1) - 1) as f32 * x_spacing
    }
}

// ----------------------------------------------------------------------------

// type alias for boxed function to determine row color during grid generation
type ColorPickerFn = Box<dyn Send + Sync + Fn(usize, &Style) -> Option<Color32>>;

pub(crate) struct GridLayout {
    ctx: Context,
    style: Arc<Style>,
    id: Id,

    /// First frame (no previous know state).
    is_first_frame: bool,

    /// State previous frame (if any).
    /// This can be used to predict future sizes of cells.
    prev_state: State,

    /// State accumulated during the current frame.
    curr_state: State,
    initial_available: Rect,

    /// Are we inside an enclosing sizing pass (e.g. [`crate::Resize`] measuring
    /// the minimum content width)? If so we must not remember the (narrow) sizes
    /// we measure during it.
    sizing_pass: bool,

    // Options:
    num_columns: Option<usize>,
    spacing: Vec2,
    min_cell_size: Vec2,
    max_cell_size: Vec2,
    color_picker: Option<ColorPickerFn>,

    // Cursor:
    col: usize,
    row: usize,

    /// Columns advance from the reading-start edge (right under RTL) toward the reading end.
    is_rtl: bool,
}

impl GridLayout {
    pub(crate) fn new(ui: &Ui, id: Id, prev_state: Option<State>) -> Self {
        let is_first_frame = prev_state.is_none();

        // An outer sizing pass, we should render as small as possible.
        let sizing_pass = ui.is_sizing_pass();

        let prev_state = prev_state.unwrap_or_default();

        let initial_available = ui.placer().max_rect().intersect(ui.cursor());

        // The grid follows the direction of the layout it lives in, so cells are placed
        // and advanced from the reading-start edge (right under RTL):
        let is_rtl = ui.placer().prefer_right_to_left(ui.is_rtl());

        ui.ctx().check_for_id_clash(id, initial_available, "Grid");

        Self {
            ctx: ui.ctx().clone(),
            style: Arc::clone(ui.style()),
            id,
            is_first_frame,
            prev_state,
            curr_state: State::default(),
            initial_available,
            sizing_pass,

            num_columns: None,
            spacing: ui.spacing().item_spacing,
            min_cell_size: ui.spacing().interact_size,
            max_cell_size: Vec2::INFINITY,
            color_picker: None,

            col: 0,
            row: 0,
            is_rtl,
        }
    }
}

impl GridLayout {
    fn prev_col_width(&self, col: usize) -> f32 {
        self.prev_state
            .col_width(col)
            .unwrap_or(self.min_cell_size.x)
    }

    fn prev_row_height(&self, row: usize) -> f32 {
        self.prev_state
            .row_height(row)
            .unwrap_or(self.min_cell_size.y)
    }

    pub(crate) fn wrap_text(&self) -> bool {
        self.max_cell_size.x.is_finite()
    }

    pub(crate) fn available_rect(&self, region: &Region) -> Rect {
        let is_last_column = Some(self.col + 1) == self.num_columns;

        let width = if is_last_column {
            // The first frame we don't really know the widths of the previous columns,
            // so returning a big available width here can cause trouble.
            if self.is_first_frame {
                self.curr_state
                    .col_width(self.col)
                    .unwrap_or(self.min_cell_size.x)
            } else if self.is_rtl {
                // Remaining space between the reading-end edge and the current position:
                (region.cursor.max.x - self.initial_available.min.x).at_most(self.max_cell_size.x)
            } else {
                (self.initial_available.right() - region.cursor.left())
                    .at_most(self.max_cell_size.x)
            }
        } else if self.max_cell_size.x.is_finite() {
            // TODO(emilk): should probably heed `prev_state` here too
            self.max_cell_size.x
        } else {
            // If we want to allow width-filling widgets like [`Separator`] in one of the first cells
            // then we need to make sure they don't spill out of the first cell:
            self.prev_state
                .col_width(self.col)
                .or_else(|| self.curr_state.col_width(self.col))
                .unwrap_or(self.min_cell_size.x)
        };

        // If something above was wider, we can be wider:
        let width = width.max(self.curr_state.col_width(self.col).unwrap_or(0.0));

        let available = region.max_rect.intersect(region.cursor);

        let height = region.max_rect.max.y - available.top();
        let height = height
            .at_least(self.min_cell_size.y)
            .at_most(self.max_cell_size.y);

        if self.is_rtl {
            // The available span grows from the cell's right edge toward the reading end.
            Rect::from_min_size(
                pos2(available.max.x - width, available.min.y),
                vec2(width, height),
            )
        } else {
            Rect::from_min_size(available.min, vec2(width, height))
        }
    }

    pub(crate) fn next_cell(&self, cursor: Rect, child_size: Vec2) -> Rect {
        let width = if self.sizing_pass {
            0.0
        } else {
            self.prev_state.col_width(self.col).unwrap_or(0.0)
        };
        let height = self.prev_row_height(self.row);
        let size = child_size.max(vec2(width, height));
        if self.is_rtl {
            // The cell's right edge sits at the cursor and it grows toward the reading end.
            Rect::from_min_size(pos2(cursor.max.x - size.x, cursor.min.y), size).round_ui()
        } else {
            Rect::from_min_size(cursor.min, size).round_ui()
        }
    }

    pub(crate) fn align_size_within_rect(&self, size: Vec2, frame: Rect) -> Rect {
        // TODO(emilk): allow this alignment to be customized
        let align = if self.is_rtl {
            Align2::RIGHT_CENTER
        } else {
            Align2::LEFT_CENTER
        };
        align.align_size_within_rect(size, frame).round_ui()
    }

    pub(crate) fn justify_and_align(&self, frame: Rect, size: Vec2) -> Rect {
        self.align_size_within_rect(size, frame)
    }

    pub(crate) fn advance(&mut self, cursor: &mut Rect, _frame_rect: Rect, widget_rect: Rect) {
        #[cfg(debug_assertions)]
        {
            let debug_expand_width = self.style.debug.show_expand_width;
            let debug_expand_height = self.style.debug.show_expand_height;
            if debug_expand_width || debug_expand_height {
                let rect = widget_rect;
                let too_wide = rect.width() > self.prev_col_width(self.col);
                let too_high = rect.height() > self.prev_row_height(self.row);

                if (debug_expand_width && too_wide) || (debug_expand_height && too_high) {
                    let painter = self.ctx.debug_painter();
                    painter.rect_stroke(
                        rect,
                        0.0,
                        (1.0, Color32::LIGHT_BLUE),
                        crate::StrokeKind::Inside,
                    );

                    let stroke = Stroke::new(2.5, Color32::from_rgb(200, 0, 0));
                    let paint_line_seg = |a, b| painter.line_segment([a, b], stroke);

                    if debug_expand_width && too_wide {
                        paint_line_seg(rect.left_top(), rect.left_bottom());
                        paint_line_seg(rect.left_center(), rect.right_center());
                        paint_line_seg(rect.right_top(), rect.right_bottom());
                    }
                }
            }
        }

        self.curr_state
            .set_min_col_width(self.col, widget_rect.width().max(self.min_cell_size.x));
        self.curr_state
            .set_min_row_height(self.row, widget_rect.height().max(self.min_cell_size.y));

        if self.is_rtl {
            cursor.max.x -= self.prev_col_width(self.col) + self.spacing.x;
        } else {
            cursor.min.x += self.prev_col_width(self.col) + self.spacing.x;
        }
        self.col += 1;
    }

    fn paint_row(&self, cursor: &Rect, painter: &Painter) {
        // handle row color painting based on color-picker function
        let Some(color_picker) = self.color_picker.as_ref() else {
            return;
        };
        let Some(row_color) = color_picker(self.row, &self.style) else {
            return;
        };
        let Some(height) = self.prev_state.row_height(self.row) else {
            return;
        };
        // Paint background for coming row:
        let size = Vec2::new(self.prev_state.full_width(self.spacing.x), height);
        let rect_min = if self.is_rtl {
            pos2(cursor.max.x - size.x, cursor.min.y)
        } else {
            cursor.min
        };
        let rect = Rect::from_min_size(rect_min, size);
        let rect = rect.expand2(0.5 * self.spacing.y * Vec2::Y);
        let rect = rect.expand2(2.0 * Vec2::X); // HACK: just looks better with some spacing on the sides

        painter.rect_filled(rect, 2.0, row_color);
    }

    pub(crate) fn end_row(&mut self, cursor: &mut Rect, painter: &Painter) {
        if self.is_rtl {
            // RTL resets the reading-start (max) edge, mirroring the engine's wrap `end_row`
            // (which resets `max.x` to `max_rect.right()`); `cursor.min.x` stays `-INFINITY`
            // exactly as `initial_cursor` set it.
            cursor.max.x = self.initial_available.max.x;
        } else {
            cursor.min.x = self.initial_available.min.x;
        }
        cursor.min.y += self.spacing.y;
        cursor.min.y += self
            .curr_state
            .row_height(self.row)
            .unwrap_or(self.min_cell_size.y);

        self.col = 0;
        self.row += 1;

        self.paint_row(cursor, painter);
    }

    pub(crate) fn save(&self) {
        // We need to always save state on the first frame, otherwise request_discard
        // would be called repeatedly (see #5132)
        if self.curr_state != self.prev_state || self.is_first_frame {
            self.curr_state.clone().store(&self.ctx, self.id);
            self.ctx.request_repaint();
        }
    }
}

// ----------------------------------------------------------------------------

/// A simple grid layout.
///
/// The cells are laid out in reading order: left to right in an LTR layout, right to left in an
/// RTL layout. The contents of each cell are aligned to the reading-start edge and centered
/// vertically.
///
/// If you want to add multiple widgets to a cell you need to group them with
/// [`Ui::horizontal`], [`Ui::vertical`] etc.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Grid::new("some_unique_id").show(ui, |ui| {
///     ui.label("First row, first column");
///     ui.label("First row, second column");
///     ui.end_row();
///
///     ui.label("Second row, first column");
///     ui.label("Second row, second column");
///     ui.label("Second row, third column");
///     ui.end_row();
///
///     ui.horizontal(|ui| { ui.label("Same"); ui.label("cell"); });
///     ui.label("Third row, second column");
///     ui.end_row();
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
pub struct Grid {
    id_salt: IdSalt,
    num_columns: Option<usize>,
    min_col_width: Option<f32>,
    min_row_height: Option<f32>,
    max_cell_size: Vec2,
    spacing: Option<Vec2>,
    start_row: usize,
    color_picker: Option<ColorPickerFn>,
}

impl Grid {
    /// Create a new [`Grid`] with a locally unique identifier.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            num_columns: None,
            min_col_width: None,
            min_row_height: None,
            max_cell_size: Vec2::INFINITY,
            spacing: None,
            start_row: 0,
            color_picker: None,
        }
    }

    /// Setting this will allow for dynamic coloring of rows of the grid object
    #[inline]
    pub fn with_row_color<F>(mut self, color_picker: F) -> Self
    where
        F: Send + Sync + Fn(usize, &Style) -> Option<Color32> + 'static,
    {
        self.color_picker = Some(Box::new(color_picker));
        self
    }

    /// Setting this will allow the last column to expand to take up the rest of the space of the parent [`Ui`].
    #[inline]
    pub fn num_columns(mut self, num_columns: usize) -> Self {
        self.num_columns = Some(num_columns);
        self
    }

    /// If `true`, add a subtle background color to every other row.
    ///
    /// This can make a table easier to read.
    /// Default is whatever is in [`crate::Visuals::striped`].
    pub fn striped(self, striped: bool) -> Self {
        if striped {
            self.with_row_color(striped_row_color)
        } else {
            // Explicitly set the row color to nothing.
            // Needed so that when the style.visuals.striped value is checked later on,
            // it is clear that the user does not want stripes on this specific Grid.
            self.with_row_color(|_row: usize, _style: &Style| None)
        }
    }

    /// Set minimum width of each column.
    /// Default: [`crate::style::Spacing::interact_size`]`.x`.
    #[inline]
    pub fn min_col_width(mut self, min_col_width: f32) -> Self {
        self.min_col_width = Some(min_col_width);
        self
    }

    /// Set minimum height of each row.
    /// Default: [`crate::style::Spacing::interact_size`]`.y`.
    #[inline]
    pub fn min_row_height(mut self, min_row_height: f32) -> Self {
        self.min_row_height = Some(min_row_height);
        self
    }

    /// Set soft maximum width (wrapping width) of each column.
    #[inline]
    pub fn max_col_width(mut self, max_col_width: f32) -> Self {
        self.max_cell_size.x = max_col_width;
        self
    }

    /// Set spacing between columns/rows.
    /// Default: [`crate::style::Spacing::item_spacing`].
    #[inline]
    pub fn spacing(mut self, spacing: impl Into<Vec2>) -> Self {
        self.spacing = Some(spacing.into());
        self
    }

    /// Change which row number the grid starts on.
    /// This can be useful when you have a large [`crate::Grid`] inside of [`crate::ScrollArea::show_rows`].
    #[inline]
    pub fn start_row(mut self, start_row: usize) -> Self {
        self.start_row = start_row;
        self
    }
}

impl Grid {
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.show_dyn(ui, Box::new(add_contents))
    }

    fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let Self {
            id_salt,
            num_columns,
            min_col_width,
            min_row_height,
            max_cell_size,
            spacing,
            start_row,
            mut color_picker,
        } = self;
        let min_col_width = min_col_width.unwrap_or_else(|| ui.spacing().interact_size.x);
        let min_row_height = min_row_height.unwrap_or_else(|| ui.spacing().interact_size.y);
        let spacing = spacing.unwrap_or_else(|| ui.spacing().item_spacing);
        if color_picker.is_none() && ui.visuals().striped {
            color_picker = Some(Box::new(striped_row_color));
        }

        let id = ui.make_persistent_id(id_salt);
        let prev_state = State::load(ui.ctx(), id);

        // Each grid cell is aligned to the reading-start edge (LEFT_CENTER under LTR,
        // RIGHT_CENTER under RTL).
        // If somebody wants to wrap more things inside a cell,
        // then we should pick a default layout that matches that alignment,
        // which we do here:
        let max_rect = ui.cursor().intersect(ui.max_rect());

        let mut ui_builder = UiBuilder::new().max_rect(max_rect);
        if prev_state.is_none() {
            // The initial frame will be glitchy, because we don't know the sizes of things to come.

            if ui.is_visible() {
                // Try to cover up the glitchy initial frame:
                ui.request_discard("new Grid");
            }

            // Hide the ui this frame, and make things as narrow as possible:
            ui_builder = ui_builder.sizing_pass().invisible();
        }

        ui.scope_builder(ui_builder, |ui| {
            ui.horizontal(|ui| {
                let is_color = color_picker.is_some();
                let grid = GridLayout {
                    num_columns,
                    color_picker,
                    min_cell_size: vec2(min_col_width, min_row_height),
                    max_cell_size,
                    spacing,
                    row: start_row,
                    ..GridLayout::new(ui, id, prev_state)
                };

                // paint first incoming row
                if is_color {
                    let cursor = ui.cursor();
                    let painter = ui.painter();
                    grid.paint_row(&cursor, painter);
                }

                ui.set_grid(grid);
                let r = add_contents(ui);
                ui.save_grid();
                r
            })
            .inner
        })
    }
}

fn striped_row_color(row: usize, style: &Style) -> Option<Color32> {
    if row % 2 == 1 {
        return Some(style.visuals.faint_bg_color);
    }
    None
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use crate::{
        Align, Color32, Context, DragValue, FullOutput, Layout, Pos2, RawInput, Rect, Ui, Vec2,
        epaint::{RectShape, Shape},
        vec2,
    };

    use super::Grid;

    const SCREEN: Vec2 = vec2(400.0, 300.0);

    fn input() -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            ..Default::default()
        }
    }

    /// How right-to-left is triggered in a scenario.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Trigger {
        /// An explicit [`Layout::right_to_left`] on the enclosing ui.
        ExplicitLayout,

        /// [`crate::Style::right_to_left`], set through [`Context::set_right_to_left`].
        StyleFlag,
    }

    /// Run the scenario twice (a grid's first frame is an invisible sizing pass, see
    /// `show_dyn`) and return the second pass's output. The first pass doubles as the
    /// no-panic guard.
    fn run_two_passes(rtl: bool, trigger: Trigger, body: &mut dyn FnMut(&mut Ui)) -> FullOutput {
        let ctx = Context::default();
        if rtl && trigger == Trigger::StyleFlag {
            ctx.set_right_to_left(true);
        }
        let mut body_wrapper = |ui: &mut Ui| match trigger {
            Trigger::ExplicitLayout => {
                let layout = if rtl {
                    Layout::right_to_left(Align::Min)
                } else {
                    Layout::left_to_right(Align::Min)
                };
                ui.with_layout(layout, |ui| body(ui));
            }
            Trigger::StyleFlag => body(ui),
        };
        let mut first = ctx.run_ui(input(), &mut body_wrapper);
        // The first pass's output is discarded; its texture deltas must be cleared
        // before the drop guard in `TexturesDelta` sees them:
        first.textures_delta.clear();
        drop(first);
        let mut output = ctx.run_ui(input(), &mut body_wrapper);
        output.textures_delta.clear();
        output
    }

    #[test]
    fn grid_follows_layout_direction() {
        for rtl in [false, true] {
            for trigger in [Trigger::ExplicitLayout, Trigger::StyleFlag] {
                let label_rect = RefCell::new(Rect::NOTHING);
                let editor_rect = RefCell::new(Rect::NOTHING);
                let offered = RefCell::new(Rect::NOTHING);
                let mut text = String::from("value");
                run_two_passes(rtl, trigger, &mut |ui| {
                    *offered.borrow_mut() = ui.max_rect();
                    Grid::new("follows-layout-direction").show(ui, |ui| {
                        let label = ui.label("Name");
                        *label_rect.borrow_mut() = label.rect;
                        let editor = ui.text_edit_singleline(&mut text);
                        *editor_rect.borrow_mut() = editor.rect;
                        ui.end_row();
                    });
                });

                let label_rect = *label_rect.borrow();
                let editor_rect = *editor_rect.borrow();
                assert!(
                    label_rect.is_finite() && label_rect.is_positive(),
                    "label rect must be finite and positive (rtl = {rtl}, trigger = {trigger:?}): \
                     {label_rect:?}"
                );
                assert!(
                    editor_rect.is_finite() && editor_rect.is_positive(),
                    "editor rect must be finite and positive (rtl = {rtl}, trigger = {trigger:?}): \
                     {editor_rect:?}"
                );

                let offered = *offered.borrow();
                assert!(
                    offered.contains_rect(label_rect),
                    "label must be inside the offered rect {offered:?} (rtl = {rtl}, \
                     trigger = {trigger:?}): {label_rect:?}"
                );
                assert!(
                    offered.contains_rect(editor_rect),
                    "editor must be inside the offered rect {offered:?} (rtl = {rtl}, \
                     trigger = {trigger:?}): {editor_rect:?}"
                );

                if rtl {
                    assert!(
                        label_rect.right() > editor_rect.right(),
                        "in RTL the label (first cell) must be right of the editor \
                         (trigger = {trigger:?}): label = {label_rect:?}, editor = {editor_rect:?}"
                    );
                } else {
                    assert!(
                        label_rect.left() < editor_rect.left(),
                        "in LTR the label (first cell) must be left of the editor \
                         (trigger = {trigger:?}): label = {label_rect:?}, editor = {editor_rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn grid_cell_order_mirrors_in_rtl() {
        for rtl in [false, true] {
            let cells = RefCell::new(Vec::<Rect>::new());
            let mut count = 0_i32;
            run_two_passes(rtl, Trigger::ExplicitLayout, &mut |ui| {
                Grid::new("cell-order").show(ui, |ui| {
                    cells.borrow_mut().clear();
                    let label = ui.label("Label");
                    cells.borrow_mut().push(label.rect);
                    let button = ui.button("Click");
                    cells.borrow_mut().push(button.rect);
                    let drag = ui.add(DragValue::new(&mut count));
                    cells.borrow_mut().push(drag.rect);
                    ui.end_row();
                });
            });

            let cells = cells.borrow().clone();
            assert_eq!(cells.len(), 3);
            if rtl {
                assert!(
                    cells[0].right() > cells[1].right() && cells[1].right() > cells[2].right(),
                    "in RTL cell right edges must decrease in call order (rtl = {rtl}): \
                     {cells:?}"
                );
            } else {
                assert!(
                    cells[0].left() < cells[1].left() && cells[1].left() < cells[2].left(),
                    "in LTR cell left edges must increase in call order (rtl = {rtl}): {cells:?}"
                );
            }
        }
    }

    #[test]
    fn grid_rtl_rows_share_reading_start_anchor() {
        for rtl in [false, true] {
            let cells = RefCell::new(Vec::<Rect>::new()); // row-major: two rows of two cells
            run_two_passes(rtl, Trigger::ExplicitLayout, &mut |ui| {
                Grid::new("row-anchor").show(ui, |ui| {
                    cells.borrow_mut().clear();
                    for row in 0..2 {
                        let first = ui.label(format!("Row {row} A"));
                        let second = ui.label(format!("Row {row} B"));
                        ui.end_row();
                        let mut cells = cells.borrow_mut();
                        cells.push(first.rect);
                        cells.push(second.rect);
                    }
                });
            });

            let cells = cells.borrow().clone();
            assert_eq!(cells.len(), 4);
            let row1_first = cells[0];
            let row2_first = cells[2];

            // Rows stack downward as ever:
            assert!(
                row2_first.top() > row1_first.top(),
                "row 2 must be below row 1 (rtl = {rtl}): row 1 = {row1_first:?}, \
                 row 2 = {row2_first:?}"
            );

            // Every row anchors at the same reading-start edge:
            if rtl {
                assert!(
                    (row2_first.right() - row1_first.right()).abs() <= 1.0,
                    "both rows must share the right (reading-start) anchor (rtl = {rtl}): \
                     row 1 = {row1_first:?}, row 2 = {row2_first:?}"
                );
            } else {
                assert!(
                    (row2_first.left() - row1_first.left()).abs() <= 1.0,
                    "both rows must share the left (reading-start) anchor (rtl = {rtl}): \
                     row 1 = {row1_first:?}, row 2 = {row2_first:?}"
                );
            }
        }
    }

    #[test]
    fn grid_rtl_striping_covers_the_row() {
        // Stripes exist only for odd rows (`striped_row_color`), so build two rows; and they
        // need the persisted row heights of the previous frame, so assert on the second pass
        // (which `run_two_passes` returns).
        let scenario = |rtl: bool| {
            let cells = RefCell::new(Vec::<Rect>::new()); // row-major: two rows of two cells
            let faint = RefCell::new(Color32::TRANSPARENT);
            let output = run_two_passes(rtl, Trigger::ExplicitLayout, &mut |ui| {
                *faint.borrow_mut() = ui.style().visuals.faint_bg_color;
                Grid::new("striped").striped(true).show(ui, |ui| {
                    cells.borrow_mut().clear();
                    for row in 0..2 {
                        let first = ui.label(format!("Row {row} A"));
                        let second = ui.label(format!("Row {row} B"));
                        ui.end_row();
                        let mut cells = cells.borrow_mut();
                        cells.push(first.rect);
                        cells.push(second.rect);
                    }
                });
            });

            let mut stripes = Vec::new();
            for clipped in &output.shapes {
                if let Shape::Rect(RectShape { rect, fill, .. }) = &clipped.shape
                    && *fill == *faint.borrow()
                {
                    stripes.push(*rect);
                }
            }
            (stripes, cells.borrow().clone())
        };

        let (ltr_stripes, ltr_cells) = scenario(false);
        let (rtl_stripes, rtl_cells) = scenario(true);

        // Both scenarios must paint the same non-zero number of stripes:
        assert!(
            !ltr_stripes.is_empty(),
            "the LTR scenario must paint stripes"
        );
        assert_eq!(
            rtl_stripes.len(),
            ltr_stripes.len(),
            "stripe count must not depend on direction"
        );

        // Stripe width is direction-independent:
        for (ltr_stripe, rtl_stripe) in core::iter::zip(&ltr_stripes, &rtl_stripes) {
            assert!(
                ltr_stripe.is_finite() && rtl_stripe.is_finite(),
                "stripes must be finite: ltr = {ltr_stripe:?}, rtl = {rtl_stripe:?}"
            );
            assert_eq!(
                rtl_stripe.width(),
                ltr_stripe.width(),
                "stripe width must be direction-independent: ltr = {ltr_stripe:?}, \
                 rtl = {rtl_stripe:?}"
            );
        }

        // Each stripe spans its row, anchored at the reading-start edge:
        let first_cell = rtl_cells[2]; // first cell of the striped (second) row
        let last_cell = rtl_cells[3]; // last cell of the striped (second) row
        for stripe in &rtl_stripes {
            assert!(
                stripe.right() >= first_cell.right(),
                "the RTL stripe must reach the reading-start edge of the row: \
                 stripe = {stripe:?}, first cell = {first_cell:?}"
            );
            assert!(
                stripe.left() <= last_cell.left(),
                "the RTL stripe must cover the row through its last cell: \
                 stripe = {stripe:?}, last cell = {last_cell:?}"
            );
        }
        let first_cell = ltr_cells[2];
        let last_cell = ltr_cells[3];
        for stripe in &ltr_stripes {
            assert!(
                stripe.left() <= first_cell.left(),
                "the LTR stripe must reach the reading-start edge of the row: \
                 stripe = {stripe:?}, first cell = {first_cell:?}"
            );
            assert!(
                stripe.right() >= last_cell.right(),
                "the LTR stripe must cover the row through its last cell: \
                 stripe = {stripe:?}, last cell = {last_cell:?}"
            );
        }
    }

    #[test]
    fn grid_ltr_output_is_unchanged() {
        // The plain LTR baseline: a default context (no flag, no explicit layout).
        let label_rect = RefCell::new(Rect::NOTHING);
        let editor_rect = RefCell::new(Rect::NOTHING);
        let origin = RefCell::new(f32::NAN);
        let mut text = String::from("value");
        run_two_passes(false, Trigger::StyleFlag, &mut |ui| {
            *origin.borrow_mut() = ui.cursor().min.x;
            Grid::new("ltr-golden").show(ui, |ui| {
                let label = ui.label("Name");
                *label_rect.borrow_mut() = label.rect;
                let editor = ui.text_edit_singleline(&mut text);
                *editor_rect.borrow_mut() = editor.rect;
                ui.end_row();
            });
        });

        let label_rect = *label_rect.borrow();
        let editor_rect = *editor_rect.borrow();

        // Columns advance left to right in call order, respecting the column spacing:
        assert!(
            label_rect.left() < editor_rect.left(),
            "in LTR the label (first cell) must be left of the editor: \
             label = {label_rect:?}, editor = {editor_rect:?}"
        );
        assert!(
            label_rect.right() <= editor_rect.left(),
            "the editor must start after the label's column spacing: \
             label = {label_rect:?}, editor = {editor_rect:?}"
        );

        // Same row: the shorter label is vertically centered within the taller editor's band.
        assert!(
            editor_rect.top() <= label_rect.top() && label_rect.bottom() <= editor_rect.bottom(),
            "both widgets must share the same row band: label = {label_rect:?}, \
             editor = {editor_rect:?}"
        );

        // The grid anchors at the parent cursor origin:
        assert!(
            (label_rect.left() - *origin.borrow()).abs() <= 1.0,
            "the first cell must start at the parent cursor origin: label = {label_rect:?}, \
             origin = {:?}",
            origin.borrow()
        );
    }
}
