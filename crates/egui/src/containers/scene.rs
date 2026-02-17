use core::f32;

use emath::{GuiRounding as _, Pos2};

use crate::{
    InnerResponse, LayerId, PointerButton, Rangef, Rect, Response, Sense, Ui, UiBuilder, Vec2,
    emath::TSTransform,
};

/// Creates a transformation that fits a given scene rectangle into the available screen size.
///
/// The resulting visual scene bounds can be larger, due to letterboxing.
///
/// Returns the transformation from `scene` to `global` coordinates.
fn fit_to_rect_in_scene(
    rect_in_global: Rect,
    rect_in_scene: Rect,
    zoom_range: Rangef,
) -> TSTransform {
    // Compute the scale factor to fit the bounding rectangle into the available screen size:
    let scale = rect_in_global.size() / rect_in_scene.size();

    // Use the smaller of the two scales to ensure the whole rectangle fits on the screen:
    let scale = scale.min_elem();

    // Clamp scale to what is allowed
    let scale = zoom_range.clamp(scale);

    // Compute the translation to center the bounding rect in the screen:
    let center_in_global = rect_in_global.center().to_vec2();
    let center_scene = rect_in_scene.center().to_vec2();

    // Set the transformation to scale and then translate to center.
    TSTransform::from_translation(center_in_global - scale * center_scene)
        * TSTransform::from_scaling(scale)
}

/// A container that allows you to zoom and pan.
///
/// This is similar to [`crate::ScrollArea`] but:
/// * Supports zooming
/// * Has no scroll bars
/// * Has no limits on the scrolling
#[derive(Clone, Debug)]
#[must_use = "You should call .show()"]
pub struct Scene {
    zoom_range: Rangef,
    sense: Sense,
    max_inner_size: Vec2,
    drag_pan_buttons: DragPanButtons,
}

/// Specifies which pointer buttons can be used to pan the scene by dragging.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DragPanButtons(u8);

bitflags::bitflags! {
    impl DragPanButtons: u8 {
        /// [PointerButton::Primary]
        const PRIMARY = 1 << 0;

        /// [PointerButton::Secondary]
        const SECONDARY = 1 << 1;

        /// [PointerButton::Middle]
        const MIDDLE = 1 << 2;

        /// [PointerButton::Extra1]
        const EXTRA_1 = 1 << 3;

        /// [PointerButton::Extra2]
        const EXTRA_2 = 1 << 4;
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            zoom_range: Rangef::new(f32::EPSILON, 1.0),
            sense: Sense::click_and_drag(),
            max_inner_size: Vec2::splat(1000.0),
            drag_pan_buttons: DragPanButtons::all(),
        }
    }
}

impl Scene {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Specify what type of input the scene should respond to.
    ///
    /// The default is `Sense::click_and_drag()`.
    ///
    /// Set this to `Sense::hover()` to disable panning via clicking and dragging.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set the allowed zoom range.
    ///
    /// The default zoom range is `0.0..=1.0`,
    /// which mean you zan make things arbitrarily small, but you cannot zoom in past a `1:1` ratio.
    ///
    /// If you want to allow zooming in, you can set the zoom range to `0.0..=f32::INFINITY`.
    /// Note that text rendering becomes blurry when you zoom in: <https://github.com/emilk/egui/issues/4813>.
    #[inline]
    pub fn zoom_range(mut self, zoom_range: impl Into<Rangef>) -> Self {
        self.zoom_range = zoom_range.into();
        self
    }

    /// Set the maximum size of the inner [`Ui`] that will be created.
    #[inline]
    pub fn max_inner_size(mut self, max_inner_size: impl Into<Vec2>) -> Self {
        self.max_inner_size = max_inner_size.into();
        self
    }

    /// Specify which pointer buttons can be used to pan by clicking and dragging.
    ///
    /// By default, this is `DragPanButtons::all()`.
    #[inline]
    pub fn drag_pan_buttons(mut self, flags: DragPanButtons) -> Self {
        self.drag_pan_buttons = flags;
        self
    }

    /// `scene_rect` contains the view bounds of the inner [`Ui`].
    ///
    /// `scene_rect` will be mutated by any panning/zooming done by the user.
    /// If `scene_rect` is somehow invalid (e.g. `Rect::ZERO`),
    /// then it will be reset to the inner rect of the inner ui.
    ///
    /// You need to store the `scene_rect` in your state between frames.
    pub fn show<R>(
        &self,
        parent_ui: &mut Ui,
        scene_rect: &mut Rect,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let (outer_rect, _outer_response) =
            parent_ui.allocate_exact_size(parent_ui.available_size_before_wrap(), Sense::hover());

        let mut to_global = fit_to_rect_in_scene(outer_rect, *scene_rect, self.zoom_range);

        let scene_rect_was_good =
            to_global.is_valid() && scene_rect.is_finite() && scene_rect.size() != Vec2::ZERO;

        let mut inner_rect = *scene_rect;

        let ret = self.show_global_transform(parent_ui, outer_rect, &mut to_global, |ui| {
            let r = add_contents(ui);
            inner_rect = ui.min_rect();
            r
        });

        if ret.response.changed() {
            // Only update if changed, both to avoid numeric drift,
            // and to avoid expanding the scene rect unnecessarily.
            *scene_rect = to_global.inverse() * outer_rect;
        }

        if !scene_rect_was_good {
            // Auto-reset if the transformation goes bad somehow (or started bad).
            // Recalculates transform based on inner_rect, resulting in a rect that's the full size of outer_rect but centered on inner_rect.
            let to_global = fit_to_rect_in_scene(outer_rect, inner_rect, self.zoom_range);
            *scene_rect = to_global.inverse() * outer_rect;
        }

        ret
    }

    fn show_global_transform<R>(
        &self,
        parent_ui: &mut Ui,
        outer_rect: Rect,
        to_global: &mut TSTransform,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        // Create a new egui paint layer, where we can draw our contents:
        let scene_layer_id = LayerId::new(
            parent_ui.layer_id().order,
            parent_ui.id().with("scene_area"),
        );

        // Put the layer directly on-top of the main layer of the ui:
        parent_ui
            .ctx()
            .set_sublayer(parent_ui.layer_id(), scene_layer_id);

        let mut local_ui = parent_ui.new_child(
            UiBuilder::new()
                .layer_id(scene_layer_id)
                .max_rect(Rect::from_min_size(Pos2::ZERO, self.max_inner_size))
                .sense(self.sense),
        );

        let mut pan_response = local_ui.response();

        // Update the `to_global` transform based on use interaction:
        self.register_pan_and_zoom(&local_ui, &mut pan_response, to_global);

        // Set a correct global clip rect:
        local_ui.set_clip_rect(to_global.inverse() * outer_rect);

        // Tell egui to apply the transform on the layer:
        local_ui
            .ctx()
            .set_transform_layer(scene_layer_id, *to_global);

        // Add the actual contents to the area:
        let ret = add_contents(&mut local_ui);

        // This ensures we catch clicks/drags/pans anywhere on the background.
        local_ui.force_set_min_rect((to_global.inverse() * outer_rect).round_ui());

        InnerResponse {
            response: pan_response,
            inner: ret,
        }
    }

    /// Helper function to handle pan and zoom interactions on a response.
    pub fn register_pan_and_zoom(&self, ui: &Ui, resp: &mut Response, to_global: &mut TSTransform) {
        let dragged = self.drag_pan_buttons.iter().any(|button| match button {
            DragPanButtons::PRIMARY => resp.dragged_by(PointerButton::Primary),
            DragPanButtons::SECONDARY => resp.dragged_by(PointerButton::Secondary),
            DragPanButtons::MIDDLE => resp.dragged_by(PointerButton::Middle),
            DragPanButtons::EXTRA_1 => resp.dragged_by(PointerButton::Extra1),
            DragPanButtons::EXTRA_2 => resp.dragged_by(PointerButton::Extra2),
            _ => false,
        });
        if dragged {
            to_global.translation += to_global.scaling * resp.drag_delta();
            resp.mark_changed();
        }

        if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos())
            && resp.contains_pointer()
        {
            let pointer_in_scene = to_global.inverse() * mouse_pos;
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let pan_delta = ui.input(|i| i.smooth_scroll_delta());

            // Most of the time we can return early. This is also important to
            // avoid `ui_from_scene` to change slightly due to floating point errors.
            if zoom_delta == 1.0 && pan_delta == Vec2::ZERO {
                return;
            }

            if zoom_delta != 1.0 {
                // Zoom in on pointer, but only if we are not zoomed in or out too far.
                let zoom_delta = zoom_delta.clamp(
                    self.zoom_range.min / to_global.scaling,
                    self.zoom_range.max / to_global.scaling,
                );

                *to_global = *to_global
                    * TSTransform::from_translation(pointer_in_scene.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_scene.to_vec2());

                // Clamp to exact zoom range.
                to_global.scaling = self.zoom_range.clamp(to_global.scaling);
            }

            // Pan:
            *to_global = TSTransform::from_translation(pan_delta) * *to_global;
            resp.mark_changed();
        }
    }
}
