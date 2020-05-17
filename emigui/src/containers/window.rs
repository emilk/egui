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
    pub fn show(self, ctx: &Arc<Context>, add_contents: impl FnOnce(&mut Ui)) -> InteractInfo {
        let Window {
            title_label,
            open,
            area,
            frame,
            resize,
            scroll,
        } = self;

        if matches!(open, Some(false)) {
            return Default::default();
        }

        let frame = frame.unwrap_or_else(|| Frame::window(&ctx.style()));

        if true {
            // TODO: easier way to compose these
            area.show(ctx, |ui| {
                frame.show(ui, |ui| {
                    resize.show(ui, |ui| {
                        show_title_bar(ui, title_label, open);
                        if let Some(scroll) = scroll {
                            scroll.show(ui, add_contents)
                        } else {
                            add_contents(ui)
                        }
                    })
                })
            })
        } else {
            // TODO: something like this, with collapsing contents
            area.show(ctx, |ui| {
                frame.show(ui, |ui| {
                    CollapsingHeader::new(title_label.text()).show(ui, |ui| {
                        resize.show(ui, |ui| {
                            if let Some(scroll) = scroll {
                                scroll.show(ui, add_contents)
                            } else {
                                add_contents(ui)
                            }
                        })
                    });
                })
            })
        }
    }
}

fn show_title_bar(ui: &mut Ui, title_label: Label, open: Option<&mut bool>) {
    let button_size = ui.style().clickable_diameter;

    // TODO: show collapse button

    let title_rect = ui.add(title_label).rect;

    if let Some(open) = open {
        let close_max_x = title_rect.right() + ui.style().item_spacing.x + button_size;
        let close_max_x = close_max_x.max(ui.rect_finite().right());
        let close_rect = Rect::from_min_size(
            pos2(
                close_max_x - button_size,
                title_rect.center().y - 0.5 * button_size,
            ),
            Vec2::splat(button_size),
        );
        if close_button(ui, close_rect).clicked {
            *open = false;
        }
    }

    ui.add(Separator::new().line_width(1.0)); // TODO: nicer way to split window title from contents
}

fn close_button(ui: &mut Ui, rect: Rect) -> InteractInfo {
    let close_id = ui.make_child_id("window_close_button");
    let interact = ui.interact_rect(rect, close_id);
    ui.expand_to_include_child(interact.rect);

    // ui.add_paint_cmd(PaintCmd::Rect {
    //     corner_radius: ui.style().interact(&interact).corner_radius,
    //     fill_color: ui.style().interact(&interact).bg_fill_color,
    //     outline: ui.style().interact(&interact).rect_outline,
    //     rect: interact.rect,
    // });

    let rect = rect.expand(-4.0);

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
