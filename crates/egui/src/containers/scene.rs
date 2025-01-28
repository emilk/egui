//! A small, self-container pan-and-zoom area for [`egui`].
//!
//! Throughout this module, we use the following conventions or naming the different spaces:
//! * `ui`-space: The _global_ `egui` space.
//! * `view`-space: The space where the pan-and-zoom area is drawn.
//! * `scene`-space: The space where the actual content is drawn.

use core::f32;

use emath::{GuiRounding, Pos2};

use crate::{
    emath::TSTransform, load, LayerId, Rangef, Rect, Response, Sense, Ui, UiBuilder, Vec2,
};

/// Creates a transformation that fits a given scene rectangle into the available screen size.
///
/// The resulting visual scene bounds can be larger, due to letterboxing.
pub fn fit_to_rect_in_scene(rect_in_ui: Rect, rect_in_scene: Rect) -> TSTransform {
    let available_size_in_ui = rect_in_ui.size();

    // Compute the scale factor to fit the bounding rectangle into the available screen size.
    let scale_x = available_size_in_ui.x / rect_in_scene.width();
    let scale_y = available_size_in_ui.y / rect_in_scene.height();

    // Use the smaller of the two scales to ensure the whole rectangle fits on the screen.
    let scale = scale_x.min(scale_y).min(1.0);

    // Compute the translation to center the bounding rect in the screen.
    let center_screen = rect_in_ui.center();
    let center_scene = rect_in_scene.center().to_vec2();

    // Set the transformation to scale and then translate to center.
    TSTransform::from_translation(center_screen.to_vec2() - center_scene * scale)
        * TSTransform::from_scaling(scale)
}

#[derive(Clone, Debug)]
#[must_use = "You should call .show()"]
pub struct Scene {
    zoom_range: Rangef,
    max_inner_size: Vec2,
    fit_rect: Option<Rect>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            zoom_range: Rangef::new(f32::EPSILON, 1.0),
            max_inner_size: Vec2::splat(1000.0),
            fit_rect: None,
        }
    }
}

impl Scene {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the maximum size of the inner [`Ui`] that will be created.
    pub fn max_inner_size(mut self, max_inner_size: impl Into<Vec2>) -> Self {
        self.max_inner_size = max_inner_size.into();
        self
    }

    /// `to_parent` contains the transformation from the scene coordinates to that of the parent ui.
    ///
    /// `to_parent` will be mutated by any panning/zooming done by the user.
    pub fn show(
        &self,
        parent_ui: &mut Ui,
        to_parent: &mut TSTransform,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Response {
        // Create a new egui paint layer, where we can draw our contents:
        let scene_layer_id = LayerId::new(
            parent_ui.layer_id().order,
            parent_ui.id().with("scene_area"),
        );

        // Put the layer directly on-top of the main layer of the ui:
        parent_ui
            .ctx()
            .set_sublayer(parent_ui.layer_id(), scene_layer_id);

        // let size = parent_ui.available_size_before_wrap(); // TODO: let user control via builder
        let size = Vec2::splat(440.0);
        let (global_view_bounds, _outer_response) =
            parent_ui.allocate_exact_size(size, Sense::hover());

        let global_from_parent = TSTransform::from_translation(global_view_bounds.min.to_vec2());
        let mut to_global = global_from_parent * *to_parent;

        // Optionally change the transformation so that a scene rect is
        // contained in the view, potentially with letter boxing.
        if let Some(rect_in_scene) = self.fit_rect {
            // *to_parent = fit_to_rect_in_scene(global_view_bounds, rect_in_scene);
        }

        let mut local_ui = parent_ui.new_child(
            UiBuilder::new()
                .layer_id(scene_layer_id)
                .max_rect(Rect::from_min_size(Pos2::ZERO, self.max_inner_size))
                .sense(Sense::click_and_drag()),
        );

        let mut pan_response = local_ui.response();

        // Update the `to_global` transform based on use interaction:
        self.register_pan_and_zoom(&local_ui, &mut pan_response, &mut to_global);

        if pan_response.changed() {
            // Only update if changed, to avoid numeric drift
            *to_parent = global_from_parent.inverse() * to_global;
        }

        // Set a correct global clip rect:
        local_ui.set_clip_rect(to_global.inverse() * global_view_bounds);

        // Add the actual contents to the area:
        add_contents(&mut local_ui);

        // This ensures we catch clicks/drags/pans anywhere on the background.
        local_ui.force_set_min_rect((to_global.inverse() * global_view_bounds).round_ui());

        // Tell egui to apply the transform on the layer:
        local_ui
            .ctx()
            .set_transform_layer(scene_layer_id, to_global);

        pan_response
    }

    /// Helper function to handle pan and zoom interactions on a response.
    pub fn register_pan_and_zoom(&self, ui: &Ui, resp: &mut Response, to_global: &mut TSTransform) {
        if resp.dragged() {
            to_global.translation += to_global.scaling * resp.drag_delta();
            resp.mark_changed();
        }

        if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
            if resp.contains_pointer() {
                let pointer_in_scene = to_global.inverse() * mouse_pos;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // Most of the time we can return early. This is also important to
                // avoid `ui_from_scene` to change slightly due to floating point errors.
                if zoom_delta == 1.0 && pan_delta == Vec2::ZERO {
                    return;
                }

                // Zoom in on pointer, but only if we are not zoomed in or out too far.
                if zoom_delta > 1.0 && to_global.scaling < self.zoom_range.max
                    || zoom_delta < 1.0 && self.zoom_range.min < to_global.scaling
                {
                    *to_global = *to_global
                        * TSTransform::from_translation(pointer_in_scene.to_vec2())
                        * TSTransform::from_scaling(zoom_delta)
                        * TSTransform::from_translation(-pointer_in_scene.to_vec2());

                    // We clamp the resulting scaling to avoid zooming in/out too far.
                    to_global.scaling = self.zoom_range.clamp(to_global.scaling);
                }

                // Pan:
                *to_global = TSTransform::from_translation(pan_delta) * *to_global;
                resp.mark_changed();
            }
        }
    }
}
