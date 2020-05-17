use std::sync::Arc;

use crate::{widgets::*, *};

use super::*;

/// A wrapper around other containers for things you often want in a window
pub struct Window<'open> {
    pub title_label: Label,
    open: Option<&'open mut bool>,
    pub area: Area,
    pub frame: Option<Frame>,
    pub resize: Resize,
    pub scroll: Option<ScrollArea>,
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
                .auto_expand_height(false)
                .auto_expand_width(true)
                .auto_shrink_height(false)
                .auto_shrink_width(true)
                .handle_offset(Vec2::splat(4.0))
                .outline(false),
            scroll: Some(
                ScrollArea::default()
                    .always_show_scroll(false)
                    .max_height(f32::INFINITY),
            ), // As large as we can be
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

    /// Usage: `Winmdow::new(...).frame(|f| f.fill_color(Some(BLUE)))`
    /// Not sure this is a good interface for this.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn default_pos(mut self, default_pos: Pos2) -> Self {
        self.area = self.area.default_pos(default_pos);
        self
    }

    pub fn default_size(mut self, default_size: Vec2) -> Self {
        self.resize = self.resize.default_size(default_size);
        self
    }

    pub fn default_rect(self, rect: Rect) -> Self {
        self.default_pos(rect.min).default_size(rect.size())
    }

    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.resize = self.resize.min_size(min_size);
        self
    }

    pub fn max_size(mut self, max_size: Vec2) -> Self {
        self.resize = self.resize.max_size(max_size);
        self
    }

    pub fn fixed_size(mut self, size: Vec2) -> Self {
        self.resize = self.resize.fixed_size(size);
        self
    }

    /// Can you resize it with the mouse?
    /// Note that a window can still auto-resize
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resize = self.resize.resizable(resizable);
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
        let Window {
            title_label,
            open,
            area,
            frame,
            resize,
            scroll,
        } = self;

        if matches!(open, Some(false)) {
            return None;
        }

        let frame = frame.unwrap_or_else(|| Frame::window(&ctx.style()));

        Some(area.show(ctx, |ui| {
            frame.show(ui, |ui| {
                let collapsing_id = ui.make_child_id("collapsing");
                let default_expanded = true;
                let mut collapsing = collapsing_header::State::from_memory_with_default_open(
                    ui,
                    collapsing_id,
                    default_expanded,
                );
                let show_close_button = open.is_some();
                let title_bar = show_title_bar(
                    ui,
                    title_label,
                    show_close_button,
                    collapsing_id,
                    &mut collapsing,
                );
                ui.memory()
                    .collapsing_headers
                    .insert(collapsing_id, collapsing);

                let content = collapsing.add_contents(ui, |ui| {
                    resize.show(ui, |ui| {
                        ui.add(Separator::new().line_width(1.0)); // TODO: nicer way to split window title from contents
                        if let Some(scroll) = scroll {
                            scroll.show(ui, add_contents)
                        } else {
                            add_contents(ui)
                        }
                    })
                });

                if let Some(open) = open {
                    // Add close button now that we know our full width:

                    let right = content
                        .map(|c| c.rect.right())
                        .unwrap_or(title_bar.rect.right());

                    let button_size = ui.style().start_icon_width;
                    let button_rect = Rect::from_min_size(
                        pos2(
                            right - ui.style().item_spacing.x - button_size,
                            title_bar.rect.center().y - 0.5 * button_size,
                        ),
                        Vec2::splat(button_size),
                    );

                    if close_button(ui, button_rect).clicked {
                        *open = false;
                    }
                }
            })
        }))
    }
}

fn show_title_bar(
    ui: &mut Ui,
    title_label: Label,
    show_close_button: bool,
    collapsing_id: Id,
    collapsing: &mut collapsing_header::State,
) -> InteractInfo {
    ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
        ui.set_desired_height(title_label.font_height(ui));

        let item_spacing = ui.style().item_spacing;
        let button_size = ui.style().start_icon_width;

        {
            // TODO: make clickable radius larger
            ui.reserve_space(vec2(0.0, 0.0), None); // HACK: will add left spacing

            let collapse_button_interact =
                ui.reserve_space(Vec2::splat(button_size), Some(collapsing_id));
            if collapse_button_interact.clicked {
                // TODO: also do this when double-clicking window title
                collapsing.toggle(ui);
            }
            collapsing.paint_icon(ui, &collapse_button_interact);
        }

        let title_rect = ui.add(title_label).rect;

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
    })
}

fn close_button(ui: &mut Ui, rect: Rect) -> InteractInfo {
    let close_id = ui.make_child_id("window_close_button");
    let interact = ui.interact_rect(rect, close_id);
    ui.expand_to_include_child(interact.rect);

    let stroke_color = ui.style().interact(&interact).stroke_color;
    let stroke_width = ui.style().interact(&interact).stroke_width;
    ui.add_paint_cmd(PaintCmd::line_segment(
        [rect.left_top(), rect.right_bottom()],
        stroke_color,
        stroke_width,
    ));
    ui.add_paint_cmd(PaintCmd::line_segment(
        [rect.right_top(), rect.left_bottom()],
        stroke_color,
        stroke_width,
    ));
    interact
}
