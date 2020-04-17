use std::sync::Arc;

use crate::{
    layout::{make_id, Direction},
    widgets::Label,
    *,
};

#[derive(Clone, Copy, Debug)]
pub struct WindowState {
    /// Last known pos/size
    pub rect: Rect,
}

pub struct Window {
    /// The title of the window and by default the source of its identity.
    title: String,
}

impl Window {
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
        }
    }

    pub fn show<F>(self, ctx: &Arc<Context>, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let id = make_id(&self.title);

        let mut state = ctx.memory.lock().unwrap().get_or_create_window(
            id,
            Rect::from_min_size(
                vec2(400.0, 200.0), // TODO
                vec2(200.0, 200.0), // TODO
            ),
        );

        let layer = Layer::Window(id);
        let where_to_put_background = ctx.graphics.lock().unwrap().layer(layer).len();

        let style = ctx.style();
        let window_padding = style.window_padding;

        let mut contents_region = Region {
            ctx: ctx.clone(),
            layer: Layer::Popup,
            style,
            id: Default::default(),
            dir: Direction::Vertical,
            align: Align::Min,
            cursor: state.rect.min() + window_padding,
            bounding_size: vec2(0.0, 0.0),
            available_space: vec2(ctx.input.screen_size.x.min(350.0), std::f32::INFINITY), // TODO: window.width
        };

        // Show top bar:
        contents_region.add(Label::new(self.title).text_style(TextStyle::Heading));

        add_contents(&mut contents_region);

        // Now insert window background:

        // TODO: handle the last item_spacing in a nicer way
        let inner_size = contents_region.bounding_size - style.item_spacing;
        let outer_size = inner_size + 2.0 * window_padding;

        state.rect = Rect::from_min_size(state.rect.min(), outer_size);

        let mut graphics = ctx.graphics.lock().unwrap();
        let graphics = graphics.layer(layer);
        graphics.insert(
            where_to_put_background,
            PaintCmd::Rect {
                corner_radius: 5.0,
                fill_color: Some(style.background_fill_color()),
                outline: Some(Outline {
                    color: color::WHITE,
                    width: 1.0,
                }),
                rect: state.rect,
            },
        );

        let interact = ctx.interact(layer, state.rect, Some(id));
        if interact.active {
            state.rect = state.rect.translate(ctx.input().mouse_move);
        }

        let mut memory = ctx.memory.lock().unwrap();
        if interact.active || interact.clicked {
            memory.move_window_to_top(id);
        }
        memory.set_window_state(id, state);
    }
}
