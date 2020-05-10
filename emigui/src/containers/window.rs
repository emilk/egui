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
                .handle_offset(Vec2::splat(4.0))
                .auto_shrink_width(true)
                .auto_expand_width(true)
                .auto_shrink_height(false)
                .auto_expand_height(false),
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

    /// This is quite a crap idea
    /// Usage: `Winmdow::new(...).mutate(|w| w.resize = w.resize.auto_expand_width(true))`
    pub fn mutate(mut self, mutate: impl Fn(&mut Self)) -> Self {
        mutate(&mut self);
        self
    }

    /// This is quite a crap idea
    /// Usage: `Winmdow::new(...).resize(|r| r.auto_expand_width(true))`
    pub fn resize(mut self, mutate: impl Fn(Resize) -> Resize) -> Self {
        self.resize = mutate(self.resize);
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

        // TODO: easier way to compose these
        area.show(ctx, |ui| {
            frame.show(ui, |ui| {
                resize.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // TODO: prettier close button, and to the right of the window
                        if let Some(open) = open {
                            if ui.add(Button::new("X")).clicked {
                                *open = false;
                            }
                        }
                        ui.add(title_label);
                    });
                    ui.add(Separator::new().line_width(1.0)); // TODO: nicer way to split window title from contents

                    if let Some(scroll) = scroll {
                        scroll.show(ui, add_contents)
                    } else {
                        add_contents(ui)
                    }
                })
            })
        })
    }
}
