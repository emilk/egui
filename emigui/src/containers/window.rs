use std::sync::Arc;

use crate::{widgets::*, *};

use super::*;

// TODO: separate out resizing into a contained and reusable Resize-region.
#[derive(Clone, Debug)]
pub struct Window {
    title: String,
    floating: Floating,
    frame: Frame,
    resize: Resize,
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
                .auto_shrink_height(true),
        }
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
        } = self;
        floating.show(ctx, |region| {
            frame.show(region, |region| {
                resize.show(region, |region| {
                    region.add(Label::new(title).text_style(TextStyle::Heading));
                    region.add(Separator::new().line_width(1.0)); // TODO: nicer way to split window title from contents
                    add_contents(region);
                });
            });
        });
    }
}
