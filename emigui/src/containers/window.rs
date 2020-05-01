use std::sync::Arc;

use crate::{widgets::*, *};

use super::*;

/// A wrapper around other containers for things you often want in a window
#[derive(Clone, Debug)]
pub struct Window {
    pub title: String,
    pub floating: Floating,
    pub frame: Frame,
    pub resize: Resize,
    pub scroll: ScrollArea,
}

impl Window {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            title: title.clone(),
            floating: Floating::new(title),
            frame: Frame::default(),
            resize: Resize::default()
                .handle_offset(Vec2::splat(4.0))
                .auto_shrink_width(true)
                .auto_expand_width(true)
                .auto_shrink_height(false)
                .auto_expand_height(false),
            scroll: ScrollArea::default()
                .always_show_scroll(false)
                .max_height(f32::INFINITY), // As large as we can be
        }
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
        self.floating = self.floating.default_pos(default_pos);
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
}

impl Window {
    pub fn show(self, ctx: &Arc<Context>, add_contents: impl FnOnce(&mut Region)) {
        let Window {
            title,
            floating,
            frame,
            resize,
            scroll,
        } = self;
        // TODO: easier way to compose these
        floating.show(ctx, |region| {
            frame.show(region, |region| {
                resize.show(region, |region| {
                    region.add(Label::new(title).text_style(TextStyle::Heading));
                    region.add(Separator::new().line_width(1.0)); // TODO: nicer way to split window title from contents
                    scroll.show(region, |region| add_contents(region))
                })
            })
        })
    }
}
