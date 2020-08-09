use std::sync::Arc;

use crate::{paint::*, widgets::*, *};

use super::*;

/// A floating window which can be moved, closed, collapsed, resized and scrolled.
pub struct Window<'open> {
    pub title_label: Label,
    open: Option<&'open mut bool>,
    pub area: Area,
    pub frame: Option<Frame>,
    pub resize: Resize,
    pub scroll: Option<ScrollArea>,
    pub collapsible: bool,
}

impl<'open> Window<'open> {
    // TODO: Into<Label>
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        let area = Area::new(&title);
        let title_label = Label::new(title)
            .text_style(TextStyle::Heading)
            .multiline(false);
        Self {
            title_label,
            open: None,
            area,
            frame: None,
            resize: Resize::default()
                .outline(false)
                .min_size([96.0, 32.0])
                .default_size([280.0, 400.0]),
            scroll: Some(
                ScrollArea::default()
                    .always_show_scroll(false)
                    .max_height(f32::INFINITY),
            ), // As large as we can be
            collapsible: true,
        }
    }

    /// If the given bool is false, the window will not be visible.
    /// If the given bool is true, the window will have a close button that sets this bool to false.
    pub fn open(mut self, open: &'open mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Usage: `Winmdow::new(...).mutate(|w| w.resize = w.resize.auto_expand_width(true))`
    /// Not sure this is a good interface for this.
    pub fn mutate(mut self, mutate: impl Fn(&mut Self)) -> Self {
        mutate(&mut self);
        self
    }

    /// Usage: `Winmdow::new(...).resize(|r| r.auto_expand_width(true))`
    /// Not sure this is a good interface for this.
    pub fn resize(mut self, mutate: impl Fn(Resize) -> Resize) -> Self {
        self.resize = mutate(self.resize);
        self
    }

    /// Usage: `Winmdow::new(...).frame(|f| f.fill(Some(BLUE)))`
    /// Not sure this is a good interface for this.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn default_pos(mut self, default_pos: impl Into<Pos2>) -> Self {
        self.area = self.area.default_pos(default_pos);
        self
    }

    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.resize = self.resize.default_size(default_size);
        self
    }

    pub fn default_width(mut self, default_width: f32) -> Self {
        self.resize = self.resize.default_width(default_width);
        self
    }

    pub fn default_height(mut self, default_height: f32) -> Self {
        self.resize = self.resize.default_height(default_height);
        self
    }

    pub fn default_rect(self, rect: Rect) -> Self {
        self.default_pos(rect.min).default_size(rect.size())
    }

    /// Positions the window and prevents it from being moved
    pub fn fixed_pos(mut self, pos: impl Into<Pos2>) -> Self {
        self.area = self.area.fixed_pos(pos);
        self
    }

    pub fn fixed_size(mut self, size: impl Into<Vec2>) -> Self {
        self.resize = self.resize.fixed_size(size);
        self
    }

    /// Can you resize it with the mouse?
    /// Note that a window can still auto-resize
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resize = self.resize.resizable(resizable);
        self
    }

    /// Can the window be collapsed by clicking on its title?
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Not resizable, just takes the size of its contents.
    pub fn auto_sized(mut self) -> Self {
        self.resize = self.resize.auto_sized();
        self.scroll = None;
        self
    }

    pub fn scroll(mut self, scroll: bool) -> Self {
        if !scroll {
            self.scroll = None;
        }
        self
    }
}

impl<'open> Window<'open> {
    pub fn show(
        self,
        ctx: &Arc<Context>,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Option<InteractInfo> {
        self.show_impl(ctx, Box::new(add_contents))
    }

    fn show_impl<'c>(
        self,
        ctx: &Arc<Context>,
        add_contents: Box<dyn FnOnce(&mut Ui) + 'c>,
    ) -> Option<InteractInfo> {
        let Window {
            title_label,
            open,
            area,
            frame,
            resize,
            scroll,
            collapsible,
        } = self;

        if matches!(open, Some(false)) {
            return None;
        }

        let window_id = Id::new(title_label.text());
        let area_layer = area.layer();
        let resize_id = window_id.with("resize");
        let collapsing_id = window_id.with("collapsing");

        let possible = PossibleInteractions {
            movable: area.is_movable(),
            resizable: resize.is_resizable()
                && collapsing_header::State::is_open(ctx, collapsing_id).unwrap_or_default(),
        };

        let area = area.movable(false); // We move it manually
        let resize = resize.resizable(false); // We move it manually

        let resize = resize.id(resize_id);

        let frame = frame.unwrap_or_else(|| Frame::window(&ctx.style()));

        let mut area = area.begin(ctx);

        // First interact (move etc) to avoid frame delay:
        let last_frame_outer_rect = area.state().rect();
        let interaction = if possible.movable || possible.resizable {
            let title_bar_height =
                title_label.font_height(ctx.fonts()) + 1.0 * ctx.style().item_spacing.y; // this could be better
            let margins = 2.0 * frame.margin + vec2(0.0, title_bar_height);

            window_interaction(
                ctx,
                possible,
                area_layer,
                window_id.with("frame_resize"),
                last_frame_outer_rect,
            )
            .and_then(|window_interaction| {
                interact(
                    window_interaction,
                    ctx,
                    margins,
                    area_layer,
                    area.state_mut(),
                    resize_id,
                )
            })
        } else {
            None
        };
        let hover_interaction = resize_hover(ctx, possible, area_layer, last_frame_outer_rect);

        let mut area_content_ui = area.content_ui(ctx);

        {
            // BEGIN FRAME --------------------------------
            let mut frame = frame.begin(&mut area_content_ui);

            let default_expanded = true;
            let mut collapsing = collapsing_header::State::from_memory_with_default_open(
                &frame.content_ui,
                collapsing_id,
                default_expanded,
            );
            let show_close_button = open.is_some();
            let title_bar = show_title_bar(
                &mut frame.content_ui,
                title_label,
                show_close_button,
                collapsing_id,
                &mut collapsing,
                collapsible,
            );

            let content_rect = collapsing
                .add_contents(&mut frame.content_ui, |ui| {
                    resize.show(ui, |ui| {
                        // Add some spacing between title and content:
                        ui.allocate_space(ui.style().item_spacing);

                        if let Some(scroll) = scroll {
                            scroll.show(ui, add_contents)
                        } else {
                            add_contents(ui)
                        }
                    })
                })
                .map(|ri| ri.1);

            let outer_rect = frame.end(&mut area_content_ui);
            // END FRAME --------------------------------

            title_bar.ui(
                &mut area_content_ui,
                outer_rect,
                content_rect,
                open,
                &mut collapsing,
                collapsible,
            );

            area_content_ui
                .memory()
                .collapsing_headers
                .insert(collapsing_id, collapsing);

            if let Some(interaction) = interaction {
                paint_frame_interaction(
                    &mut area_content_ui,
                    outer_rect,
                    interaction,
                    ctx.style().interact.active,
                );
            } else if let Some(hover_interaction) = hover_interaction {
                paint_frame_interaction(
                    &mut area_content_ui,
                    outer_rect,
                    hover_interaction,
                    ctx.style().interact.hovered,
                );
            }
        }
        let full_interact = area.end(ctx, area_content_ui);

        Some(full_interact)
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct PossibleInteractions {
    movable: bool,
    resizable: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WindowInteraction {
    pub(crate) area_layer: Layer,
    pub(crate) start_rect: Rect,
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) top: bool,
    pub(crate) bottom: bool,
}

impl WindowInteraction {
    pub fn set_cursor(&self, ctx: &Context) {
        if (self.left && self.top) || (self.right && self.bottom) {
            ctx.output().cursor_icon = CursorIcon::ResizeNwSe;
        } else if (self.right && self.top) || (self.left && self.bottom) {
            ctx.output().cursor_icon = CursorIcon::ResizeNeSw;
        } else if self.left || self.right {
            ctx.output().cursor_icon = CursorIcon::ResizeHorizontal;
        } else if self.bottom || self.top {
            ctx.output().cursor_icon = CursorIcon::ResizeVertical;
        }
    }

    pub fn is_resize(&self) -> bool {
        self.left || self.right || self.top || self.bottom
    }

    pub fn is_pure_move(&self) -> bool {
        !self.is_resize()
    }
}

fn interact(
    window_interaction: WindowInteraction,
    ctx: &Context,
    margins: Vec2,
    area_layer: Layer,
    area_state: &mut area::State,
    resize_id: Id,
) -> Option<WindowInteraction> {
    let new_rect = resize_window(ctx, &window_interaction)?;

    let new_rect = ctx.round_rect_to_pixels(new_rect);
    // TODO: add this to a Window state instead as a command "move here next frame"

    area_state.pos = new_rect.min;

    if window_interaction.is_resize() {
        let mut resize_state = ctx.memory().resize.get(&resize_id).cloned().unwrap();
        resize_state.requested_size = Some(new_rect.size() - margins);
        ctx.memory().resize.insert(resize_id, resize_state);
    }

    ctx.memory().areas.move_to_top(area_layer);
    Some(window_interaction)
}

fn resize_window(ctx: &Context, window_interaction: &WindowInteraction) -> Option<Rect> {
    window_interaction.set_cursor(ctx);
    let mouse_pos = ctx.input().mouse.pos?;
    let mut rect = window_interaction.start_rect; // prevent drift

    if window_interaction.is_resize() {
        if window_interaction.left {
            rect.min.x = ctx.round_to_pixel(mouse_pos.x);
        } else if window_interaction.right {
            rect.max.x = ctx.round_to_pixel(mouse_pos.x);
        }

        if window_interaction.top {
            rect.min.y = ctx.round_to_pixel(mouse_pos.y);
        } else if window_interaction.bottom {
            rect.max.y = ctx.round_to_pixel(mouse_pos.y);
        }
    } else {
        // movevement
        rect = rect.translate(mouse_pos - ctx.input().mouse.press_origin?);
    }

    Some(rect)
}

fn window_interaction(
    ctx: &Context,
    possible: PossibleInteractions,
    area_layer: Layer,
    id: Id,
    rect: Rect,
) -> Option<WindowInteraction> {
    {
        let drag_id = ctx.memory().interaction.drag_id;

        if drag_id.is_some() && drag_id != Some(id) {
            return None;
        }
    }

    let mut window_interaction = { ctx.memory().window_interaction };

    if window_interaction.is_none() {
        if let Some(hover_window_interaction) = resize_hover(ctx, possible, area_layer, rect) {
            hover_window_interaction.set_cursor(ctx);
            if ctx.input().mouse.pressed {
                ctx.memory().interaction.drag_id = Some(id);
                ctx.memory().interaction.drag_is_window = true;
                window_interaction = Some(hover_window_interaction);
                ctx.memory().window_interaction = window_interaction;
            }
        }
    }

    if let Some(window_interaction) = window_interaction {
        let is_active = ctx.memory().interaction.drag_id == Some(id);

        if is_active && window_interaction.area_layer == area_layer {
            return Some(window_interaction);
        }
    }

    None
}

fn resize_hover(
    ctx: &Context,
    possible: PossibleInteractions,
    area_layer: Layer,
    rect: Rect,
) -> Option<WindowInteraction> {
    if let Some(mouse_pos) = ctx.input().mouse.pos {
        if let Some(top_layer) = ctx.layer_at(mouse_pos) {
            if top_layer != area_layer && top_layer.order != Order::Background {
                return None; // Another window is on top here
            }
        }

        if ctx.memory().interaction.drag_interest {
            // Another widget will become active if we drag here
            return None;
        }

        let side_interact_radius = ctx.style().resize_interact_radius_side;
        let corner_interact_radius = ctx.style().resize_interact_radius_corner;
        if rect.expand(side_interact_radius).contains(mouse_pos) {
            let (mut left, mut right, mut top, mut bottom) = Default::default();
            if possible.resizable {
                right = (rect.right() - mouse_pos.x).abs() <= side_interact_radius;
                bottom = (rect.bottom() - mouse_pos.y).abs() <= side_interact_radius;

                if rect.right_bottom().distance(mouse_pos) < corner_interact_radius {
                    right = true;
                    bottom = true;
                }

                if possible.movable {
                    left = (rect.left() - mouse_pos.x).abs() <= side_interact_radius;
                    top = (rect.top() - mouse_pos.y).abs() <= side_interact_radius;

                    if rect.right_top().distance(mouse_pos) < corner_interact_radius {
                        right = true;
                        top = true;
                    }
                    if rect.left_top().distance(mouse_pos) < corner_interact_radius {
                        left = true;
                        top = true;
                    }
                    if rect.left_bottom().distance(mouse_pos) < corner_interact_radius {
                        left = true;
                        bottom = true;
                    }
                }
            }
            let any_resize = left || right || top || bottom;

            if !any_resize && !possible.movable {
                return None;
            }

            if any_resize || possible.movable {
                Some(WindowInteraction {
                    area_layer,
                    start_rect: rect,
                    left,
                    right,
                    top,
                    bottom,
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Fill in parts of the window frame when we resize by dragging that part
fn paint_frame_interaction(
    ui: &mut Ui,
    rect: Rect,
    interaction: WindowInteraction,
    style: style::WidgetStyle,
) {
    let cr = ui.style().window.corner_radius;
    let Rect { min, max } = rect;

    let mut path = Path::default();

    if interaction.right && !interaction.bottom && !interaction.top {
        path.add_line_segment([pos2(max.x, min.y + cr), pos2(max.x, max.y - cr)]);
    }
    if interaction.right && interaction.bottom {
        path.add_line_segment([pos2(max.x, min.y + cr), pos2(max.x, max.y - cr)]);
        path.add_circle_quadrant(pos2(max.x - cr, max.y - cr), cr, 0.0);
    }
    if interaction.bottom {
        path.add_line_segment([pos2(max.x - cr, max.y), pos2(min.x + cr, max.y)]);
    }
    if interaction.left && interaction.bottom {
        path.add_circle_quadrant(pos2(min.x + cr, max.y - cr), cr, 1.0);
    }
    if interaction.left {
        path.add_line_segment([pos2(min.x, max.y - cr), pos2(min.x, min.y + cr)]);
    }
    if interaction.left && interaction.top {
        path.add_circle_quadrant(pos2(min.x + cr, min.y + cr), cr, 2.0);
    }
    if interaction.top {
        path.add_line_segment([pos2(min.x + cr, min.y), pos2(max.x - cr, min.y)]);
    }
    if interaction.right && interaction.top {
        path.add_circle_quadrant(pos2(max.x - cr, min.y + cr), cr, 3.0);
        path.add_line_segment([pos2(max.x, min.y + cr), pos2(max.x, max.y - cr)]);
    }
    ui.painter().add(PaintCmd::Path {
        path,
        closed: false,
        fill: None,
        outline: style.rect_outline,
    });
}

// ----------------------------------------------------------------------------

struct TitleBar {
    title_label: Label,
    title_galley: font::Galley,
    title_rect: Rect,
    rect: Rect,
}

fn show_title_bar(
    ui: &mut Ui,
    title_label: Label,
    show_close_button: bool,
    collapsing_id: Id,
    collapsing: &mut collapsing_header::State,
    collapsible: bool,
) -> TitleBar {
    let title_bar_and_rect = ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
        ui.set_desired_height(title_label.font_height(ui.fonts()));

        let item_spacing = ui.style().item_spacing;
        let button_size = ui.style().start_icon_width;

        if collapsible {
            // TODO: make clickable radius larger
            ui.allocate_space(vec2(0.0, 0.0)); // HACK: will add left spacing

            let rect = ui.allocate_space(Vec2::splat(button_size));
            let collapse_button_interact = ui.interact(rect, collapsing_id, Sense::click());
            if collapse_button_interact.clicked {
                collapsing.toggle(ui);
            }
            collapsing.paint_icon(ui, &collapse_button_interact);
        }

        let title_galley = title_label.layout(ui);
        let title_rect = ui.allocate_space(title_galley.size);

        if show_close_button {
            // Reserve space for close button which will be added later:
            let close_max_x = title_rect.right() + item_spacing.x + button_size + item_spacing.x;
            let close_max_x = close_max_x.max(ui.rect_finite().right());
            let close_rect = Rect::from_min_size(
                pos2(
                    close_max_x - button_size,
                    title_rect.center().y - 0.5 * button_size,
                ),
                Vec2::splat(button_size),
            );
            ui.expand_to_include_child(close_rect);
        }

        TitleBar {
            title_label,
            title_galley,
            title_rect,
            rect: Default::default(), // Will be filled in later
        }
    });

    TitleBar {
        rect: title_bar_and_rect.1,
        ..title_bar_and_rect.0
    }
}

impl TitleBar {
    fn ui(
        mut self,
        ui: &mut Ui,
        outer_rect: Rect,
        content_rect: Option<Rect>,
        open: Option<&mut bool>,
        collapsing: &mut collapsing_header::State,
        collapsible: bool,
    ) {
        if let Some(content_rect) = content_rect {
            // Now we know how large we got to be:
            self.rect.max.x = self.rect.max.x.max(content_rect.max.x);
        }

        if let Some(open) = open {
            // Add close button now that we know our full width:
            if self.close_button_ui(ui).clicked {
                *open = false;
            }
        }

        // TODO: pick style for title based on move interaction
        self.title_label
            .paint_galley(ui, self.title_rect.min, self.title_galley);

        if let Some(content_rect) = content_rect {
            // paint separator between title and content:
            let left = outer_rect.left();
            let right = outer_rect.right();
            let y = content_rect.top() + ui.style().item_spacing.y * 0.5;
            ui.painter().add(PaintCmd::LineSegment {
                points: [pos2(left, y), pos2(right, y)],
                style: ui.style().interact.inactive.rect_outline.unwrap(),
            });
        }

        let title_bar_id = ui.make_child_id("title_bar");
        if ui
            .interact(self.rect, title_bar_id, Sense::click())
            .double_clicked
            && collapsible
        {
            collapsing.toggle(ui);
        }
    }

    fn close_button_ui(&self, ui: &mut Ui) -> InteractInfo {
        let button_size = ui.style().start_icon_width;
        let button_rect = Rect::from_min_size(
            pos2(
                self.rect.right() - ui.style().item_spacing.x - button_size,
                self.rect.center().y - 0.5 * button_size,
            ),
            Vec2::splat(button_size),
        );

        close_button(ui, button_rect)
    }
}

fn close_button(ui: &mut Ui, rect: Rect) -> InteractInfo {
    let close_id = ui.make_child_id("window_close_button");
    let interact = ui.interact(rect, close_id, Sense::click());
    ui.expand_to_include_child(interact.rect);

    let stroke_color = ui.style().interact(&interact).stroke_color;
    let stroke_width = ui.style().interact(&interact).stroke_width;
    ui.painter().add(PaintCmd::line_segment(
        [rect.left_top(), rect.right_bottom()],
        stroke_color,
        stroke_width,
    ));
    ui.painter().add(PaintCmd::line_segment(
        [rect.right_top(), rect.left_bottom()],
        stroke_color,
        stroke_width,
    ));
    interact
}
