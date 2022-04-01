//! Table view with (optional) fixed header and scrolling body.
//! Cell widths are precalculated with given size hints so we can have tables like this:
//! | fixed size | all available space/minimum | 30% of available width | fixed size |
//! Takes all available height, so if you want something below the table, put it in a strip.

use crate::{
    layout::{CellDirection, CellSize},
    sizing::Sizing,
    Size, StripLayout,
};

use egui::{Response, Ui};

/// Builder for a [`Table`] with (optional) fixed header and scrolling body.
///
/// Cell widths are precalculated with given size hints so we can have tables like this:
///
/// | fixed size | all available space/minimum | 30% of available width | fixed size |
///
/// In contrast to normal egui behavior, columns/rows do *not* grow with its children!
/// Takes all available height, so if you want something below the table, put it in a strip.
///
/// ### Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui_extras::{TableBuilder, Size};
/// TableBuilder::new(ui)
///     .column(Size::remainder().at_least(100.0))
///     .column(Size::exact(40.0))
///     .header(20.0, |mut header| {
///         header.col(|ui| {
///             ui.heading("Growing");
///         });
///         header.col(|ui| {
///             ui.heading("Fixed");
///         });
///     })
///     .body(|mut body| {
///         body.row(30.0, |mut row| {
///             row.col(|ui| {
///                 ui.label("first row growing cell");
///             });
///             row.col_clip(|ui| {
///                 ui.button("action");
///             });
///         });
///     });
/// # });
/// ```
pub struct TableBuilder<'a> {
    ui: &'a mut Ui,
    sizing: Sizing,
    scroll: bool,
    striped: bool,
    resizable: bool,
}

impl<'a> TableBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        let sizing = Sizing::new();

        Self {
            ui,
            sizing,
            scroll: true,
            striped: false,
            resizable: false,
        }
    }

    /// Enable scrollview in body (default: true)
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    /// Enable striped row background (default: false)
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Make the columns resizable by dragging.
    ///
    /// Default is `false`.
    ///
    /// If you have multiple [`Table`]:s in the same [`Ui`]
    /// you will need to give them unique id:s with [`Ui::push_id`].
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Add size hint for column
    pub fn column(mut self, width: Size) -> Self {
        self.sizing.add(width);
        self
    }

    /// Add size hint for several columns at once.
    pub fn columns(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add(size);
        }
        self
    }

    fn available_width(&self) -> f32 {
        self.ui.available_rect_before_wrap().width()
            - if self.scroll {
                self.ui.spacing().item_spacing.x + self.ui.spacing().scroll_bar_width
            } else {
                0.0
            }
    }

    /// Create a header row which always stays visible and at the top
    pub fn header(self, height: f32, header: impl FnOnce(TableRow<'_, '_>)) -> Table<'a> {
        let available_width = self.available_width();

        let Self {
            ui,
            sizing,
            scroll,
            striped,
            resizable,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));
        let widths = if let Some(resize_id) = resize_id {
            ui.data().get_persisted(resize_id)
        } else {
            None
        };
        let widths = widths
            .unwrap_or_else(|| sizing.to_lengths(available_width, ui.spacing().item_spacing.x));

        let table_top = ui.min_rect().bottom();

        {
            let mut layout = StripLayout::new(ui, CellDirection::Horizontal);
            header(TableRow {
                layout: &mut layout,
                widths: &widths,
                striped: false,
                height,
                clicked: false,
            });
            layout.allocate_rect();
        }

        Table {
            ui,
            table_top,
            resize_id,
            sizing,
            widths,
            scroll,
            striped,
        }
    }

    /// Create table body without a header row
    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let available_width = self.available_width();

        let Self {
            ui,
            sizing,
            scroll,
            striped,
            resizable,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));
        let widths = if let Some(resize_id) = resize_id {
            ui.data().get_persisted(resize_id)
        } else {
            None
        };
        let widths = widths
            .unwrap_or_else(|| sizing.to_lengths(available_width, ui.spacing().item_spacing.x));

        let table_top = ui.min_rect().bottom();

        Table {
            ui,
            table_top,
            resize_id,
            sizing,
            widths,
            scroll,
            striped,
        }
        .body(body);
    }
}

/// Table struct which can construct a [`TableBody`].
///
/// Is created by [`TableBuilder`] by either calling [`TableBuilder::body`] or after creating a header row with [`TableBuilder::header`].
pub struct Table<'a> {
    ui: &'a mut Ui,
    table_top: f32,
    resize_id: Option<egui::Id>,
    sizing: Sizing,
    widths: Vec<f32>,
    scroll: bool,
    striped: bool,
}

impl<'a> Table<'a> {
    /// Create table body after adding a header row
    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let Table {
            ui,
            table_top,
            resize_id,
            sizing,
            widths,
            scroll,
            striped,
        } = self;

        let avail_rect = ui.available_rect_before_wrap();

        let mut new_widths = widths.clone();

        egui::ScrollArea::new([false, scroll])
            .auto_shrink([true; 2])
            .show(ui, move |ui| {
                let layout = StripLayout::new(ui, CellDirection::Horizontal);

                body(TableBody {
                    layout,
                    widths,
                    striped,
                    row_nr: 0,
                    start_y: avail_rect.top(),
                    end_y: avail_rect.bottom(),
                });
            });

        let bottom = ui.min_rect().bottom();

        // TODO: fix frame-delay by interacting before laying out (but painting later).
        if let Some(resize_id) = resize_id {
            let spacing_x = ui.spacing().item_spacing.x;
            let mut x = avail_rect.left() - spacing_x * 0.5;
            for (i, width) in new_widths.iter_mut().enumerate() {
                x += *width + spacing_x;

                let resize_id = ui.id().with("__panel_resize").with(i);

                let mut p0 = egui::pos2(x, table_top);
                let mut p1 = egui::pos2(x, bottom);
                let line_rect = egui::Rect::from_min_max(p0, p1)
                    .expand(ui.style().interaction.resize_grab_radius_side);
                let mouse_over_resize_line = ui.rect_contains_pointer(line_rect);

                if ui.input().pointer.any_pressed()
                    && ui.input().pointer.any_down()
                    && mouse_over_resize_line
                {
                    ui.memory().set_dragged_id(resize_id);
                }
                let is_resizing = ui.memory().is_being_dragged(resize_id);
                if is_resizing {
                    if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                        let new_width = *width + pointer.x - x;
                        let (min, max) = sizing.sizes[i].range();
                        let new_width = new_width.clamp(min, max);
                        let x = x - *width + new_width;
                        p0.x = x;
                        p1.x = x;

                        *width = new_width;
                    }
                }

                let dragging_something_else =
                    ui.input().pointer.any_down() || ui.input().pointer.any_pressed();
                let resize_hover = mouse_over_resize_line && !dragging_something_else;

                if resize_hover || is_resizing {
                    ui.output().cursor_icon = egui::CursorIcon::ResizeHorizontal;
                }

                let stroke = if is_resizing {
                    ui.style().visuals.widgets.active.bg_stroke
                } else if resize_hover {
                    ui.style().visuals.widgets.hovered.bg_stroke
                } else {
                    // ui.visuals().widgets.inactive.bg_stroke
                    ui.visuals().widgets.noninteractive.bg_stroke
                };
                ui.painter().line_segment([p0, p1], stroke);
            }

            ui.data().insert_persisted(resize_id, new_widths);
        }
    }
}

/// The body of a table.
/// Is created by calling `body` on a [`Table`] (after adding a header row) or [`TableBuilder`] (without a header row).
pub struct TableBody<'a> {
    layout: StripLayout<'a>,
    widths: Vec<f32>,
    striped: bool,
    row_nr: usize,
    start_y: f32,
    end_y: f32,
}

impl<'a> TableBody<'a> {
    /// Add rows with same height.
    ///
    /// Is a lot more performant than adding each individual row as non visible rows must not be rendered
    pub fn rows(mut self, height: f32, rows: usize, mut row: impl FnMut(usize, TableRow<'_, '_>)) {
        let delta = self.layout.current_y() - self.start_y;
        let mut start = 0;

        if delta < 0.0 {
            start = (-delta / height).floor() as usize;

            let skip_height = start as f32 * height;
            TableRow {
                layout: &mut self.layout,
                widths: &self.widths,
                striped: false,
                height: skip_height,
                clicked: false,
            }
            .col(|_| ()); // advances the cursor
        }

        let max_height = self.end_y - self.start_y;
        let count = (max_height / height).ceil() as usize;
        let end = rows.min(start + count);

        for idx in start..end {
            row(
                idx,
                TableRow {
                    layout: &mut self.layout,
                    widths: &self.widths,
                    striped: self.striped && idx % 2 == 0,
                    height,
                    clicked: false,
                },
            );
        }

        if rows - end > 0 {
            let skip_height = (rows - end) as f32 * height;

            TableRow {
                layout: &mut self.layout,
                widths: &self.widths,
                striped: false,
                height: skip_height,
                clicked: false,
            }
            .col(|_| ()); // advances the cursor
        }
    }

    /// Add row with individual height
    pub fn row(&mut self, height: f32, row: impl FnOnce(TableRow<'a, '_>)) {
        row(TableRow {
            layout: &mut self.layout,
            widths: &self.widths,
            striped: self.striped && self.row_nr % 2 == 0,
            height,
            clicked: false,
        });

        self.row_nr += 1;
    }
}

impl<'a> Drop for TableBody<'a> {
    fn drop(&mut self) {
        self.layout.allocate_rect();
    }
}

/// The row of a table.
/// Is created by [`TableRow`] for each created [`TableBody::row`] or each visible row in rows created by calling [`TableBody::rows`].
pub struct TableRow<'a, 'b> {
    layout: &'b mut StripLayout<'a>,
    widths: &'b [f32],
    striped: bool,
    height: f32,
    clicked: bool,
}

impl<'a, 'b> TableRow<'a, 'b> {
    /// Check if row was clicked
    pub fn clicked(&self) -> bool {
        self.clicked
    }

    /// Add column, content is wrapped
    pub fn col(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        self.column(false, add_contents)
    }

    /// Add column, content is clipped
    pub fn col_clip(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        self.column(true, add_contents)
    }

    fn column(&mut self, clip: bool, add_contents: impl FnOnce(&mut Ui)) -> Response {
        assert!(
            !self.widths.is_empty(),
            "Tried using more table columns than available."
        );

        let width = self.widths[0];
        self.widths = &self.widths[1..];
        let width = CellSize::Absolute(width);
        let height = CellSize::Absolute(self.height);

        let response;

        if self.striped {
            response = self.layout.add_striped(width, height, clip, add_contents);
        } else {
            response = self.layout.add(width, height, clip, add_contents);
        }

        if response.clicked() {
            self.clicked = true;
        }

        response
    }
}

impl<'a, 'b> Drop for TableRow<'a, 'b> {
    fn drop(&mut self) {
        self.layout.end_line();
    }
}
