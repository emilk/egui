//! Panels are fixed `Ui` regions.
//! Together with `Window` and `Area`:s they are
//! the only places where you can put you widgets.

use crate::*;

// ----------------------------------------------------------------------------

/// A panel that covers the entire left side of the screen.
///
/// Panels should be added before adding any `Window`s.
pub struct SidePanel {
    id: Id,
    max_width: f32,
}

impl SidePanel {
    /// `id_source`: Something unique, e.g. `"my_side_panel"`.
    /// The given `max_width` is a soft maximum (as always), and the actual panel may be smaller or larger.
    pub fn left(id_source: impl std::hash::Hash, max_width: f32) -> Self {
        Self {
            id: Id::new(id_source),
            max_width,
        }
    }
}

impl SidePanel {
    pub fn show<R>(self, ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
        let Self { id, max_width } = self;

        let mut panel_rect = ctx.available_rect();
        panel_rect.max.x = panel_rect.max.x.at_most(panel_rect.min.x + max_width);

        let layer_id = LayerId::background();

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, panel_rect, clip_rect);

        let frame = Frame::panel(&ctx.style());
        let r = frame.show(&mut panel_ui, |ui| {
            ui.set_min_height(ui.max_rect_finite().height()); // fill full height
            add_contents(ui)
        });

        let panel_rect = panel_ui.min_rect();
        let response = panel_ui.interact_hover(panel_rect);

        ctx.frame_state().allocate_left_panel(panel_rect);

        (r, response)
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers the entire top side of the screen.
///
/// Panels should be added before adding any `Window`s.
pub struct TopPanel {
    id: Id,
    max_height: Option<f32>,
}

impl TopPanel {
    /// `id_source`: Something unique, e.g. `"my_top_panel"`.
    /// Default height is that of `interact_size.y` (i.e. a button),
    /// but the panel will expand as needed.
    pub fn top(id_source: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(id_source),
            max_height: None,
        }
    }
}

impl TopPanel {
    pub fn show<R>(self, ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
        let Self { id, max_height } = self;
        let max_height = max_height.unwrap_or_else(|| ctx.style().spacing.interact_size.y);

        let mut panel_rect = ctx.available_rect();
        panel_rect.max.y = panel_rect.max.y.at_most(panel_rect.min.y + max_height);

        let layer_id = LayerId::background();

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, panel_rect, clip_rect);

        let frame = Frame::panel(&ctx.style());
        let r = frame.show(&mut panel_ui, |ui| {
            ui.set_min_width(ui.max_rect_finite().width()); // fill full width
            add_contents(ui)
        });

        let panel_rect = panel_ui.min_rect();
        let response = panel_ui.interact_hover(panel_rect);

        ctx.frame_state().allocate_top_panel(panel_rect);

        (r, response)
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers the remainder of the screen,
/// i.e. whatever area is left after adding other panels.
///
/// `CentralPanel` should be added after all other panels.
/// Any `Window`s and `Area`s will cover the `CentralPanel`.
#[derive(Default)]
pub struct CentralPanel {}

impl CentralPanel {
    pub fn show<R>(self, ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
        let Self {} = self;

        let panel_rect = ctx.available_rect();

        let layer_id = LayerId::background();
        let id = Id::new("central_panel");

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, panel_rect, clip_rect);

        let frame = Frame::background(&ctx.style());
        let r = frame.show(&mut panel_ui, |ui| {
            let r = add_contents(ui);
            ui.expand_to_include_rect(ui.max_rect()); // Use it all
            r
        });

        let panel_rect = panel_ui.min_rect();
        let response = panel_ui.interact_hover(panel_rect);

        ctx.frame_state().allocate_central_panel(panel_rect);

        (r, response)
    }
}
