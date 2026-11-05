//! Native views: UI handled and rendered by the OS and not egui. For example web views, videos, etc.
//!
//! This is a helper for implementing native views. For an example implementation, see
//! [egui_webview](https://github.com/rerun-io/egui_crates/tree/main/crates/egui_webview).
//!
//! Native views can either be shown above or below the egui surface. Below usually works better
//! since shadows, backdrops and rounded corners on top of the native view can be displayed correctly.
//! Below is not supported on all platforms though, e.g. on Windows it can only be done with the wgpu
//! dx12 backend.
//!
//! egui reports where it paints over a view ([`NativeViewOutput::egui_paint_areas`]) and where
//! it takes the pointer from it ([`NativeViewOutput::egui_input_areas`]), so tooltips and popups
//! over a view show and work.

use emath::{Rect, Vec2};

use crate::{Id, LayerId, Memory, Response, Sense, Ui, Widget, WidgetRects};

/// Whether the [`NativeView`] will be shown above or below the egui surface.
///
/// - Below, it shows through the rect egui clears to transparent, and egui paints over it like
///   over anything else. Input should be handled by the native view outside
///   [`NativeViewOutput::egui_input_areas`].
/// - Above, to correctly show egui tooltips and popups, the native view has to mask
///   [`NativeViewOutput::egui_paint_areas`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NativeViewPlacement {
    /// Under egui's surface, showing through where egui clears it.
    ///
    /// egui paints over the view, shadows and all, but the integration has to let the input
    /// through the surface to it.
    #[default]
    Below,

    /// Over egui's surface, like a platform text field on a phone.
    ///
    /// The view gets its input like any platform view, but has to mask out what egui paints
    /// over it.
    Above,
}

/// A rect egui leaves to a native view: UI that the platform draws, like a web view.
///
/// egui reports where the view goes in [`crate::PlatformOutput::native_views`], at the end of
/// the pass. A view that isn't on screen in a pass isn't in there, and should hide: one that
/// isn't shown, is in an invisible [`Ui`], or is clipped out, like when it's scrolled away.
///
/// The [`Response`] senses clicks, drags and the keyboard focus, so the view takes part in
/// egui's focus like a widget. egui doesn't see the input the platform gives the view.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let id = egui::Id::unique("page");
/// ui.add(egui::NativeView::new(id));
/// ui.add(egui::NativeView::new(id).size(egui::vec2(320.0, 240.0)).above());
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
#[derive(Clone, Copy, Debug)]
pub struct NativeView {
    id: Id,
    size: Option<Vec2>,
    side: NativeViewPlacement,
    clear: Option<bool>,
}

impl NativeView {
    /// The native view `id`, below egui's surface, which egui clears its rect for.
    ///
    /// `id` names the view in [`NativeViewOutput::id`], so it has to be unique among the views.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            size: None,
            side: NativeViewPlacement::Below,
            clear: None,
        }
    }

    /// The size of the view's rect. Default: the space available.
    #[inline]
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    /// Whether the view goes below or above egui's surface. Default: [`NativeViewPlacement::Below`].
    #[inline]
    pub fn side(mut self, side: NativeViewPlacement) -> Self {
        self.side = side;
        self
    }

    /// The view goes above egui's surface, so egui doesn't clear its rect.
    #[inline]
    pub fn above(self) -> Self {
        self.side(NativeViewPlacement::Above)
    }

    /// The view goes below egui's surface, which egui clears its rect for. The default.
    #[inline]
    pub fn below(self) -> Self {
        self.side(NativeViewPlacement::Below)
    }

    /// Whether to clear the visible part of the view's rect to transparent, for a view below
    /// egui's surface to show through.
    ///
    /// Default: `true` for a view below egui's surface, `false` above it. Turn it off below the
    /// surface for a view that something else makes visible, or that should show only where
    /// egui paints nothing.
    #[inline]
    pub fn clear(mut self, clear: bool) -> Self {
        self.clear = Some(clear);
        self
    }
}

impl Widget for NativeView {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            id,
            size,
            side,
            clear,
        } = self;
        let clear = clear.unwrap_or(side == NativeViewPlacement::Below);
        let (_, rect) = ui.allocate_space(size.unwrap_or_else(|| ui.available_size()));
        let response = ui.interact(rect, id, Sense::click_and_drag());
        if ui.is_rect_visible(rect) {
            if clear {
                // The renderer clips it to the clip rect.
                ui.painter().add(epaint::Shape::Clear(rect));
            }
            let shown = NativeViewShown {
                id,
                side,
                layer: ui.layer_id(),
                rect,
                visible: visible_rect(ui, rect),
            };
            ui.ctx()
                .pass_state_mut(|state| state.native_views.push(shown));
        }
        response
    }
}

/// Where a native view goes this pass, in ui points.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NativeViewOutput {
    /// The id given to [`NativeView::new`].
    pub id: Id,

    /// Whether the view goes below or above egui's surface.
    pub side: NativeViewPlacement,

    /// The full rect of the view.
    pub rect: Rect,

    /// The part of [`Self::rect`] inside the clip rect of its [`Ui`], rounded to pixels.
    pub visible: Rect,

    /// Where egui paints over the view: the areas (windows, popups, …) above it, inside
    /// [`Self::visible`]. A view above egui's surface masks these out.
    pub egui_paint_areas: Vec<Rect>,

    /// Where egui takes the pointer from the view: the interactive widgets in the layers above
    /// it, like the backdrop of a [`crate::Modal`], inside [`Self::visible`].
    ///
    /// Unlike [`Self::egui_paint_areas`], this leaves out what only shows, like tooltips.
    pub egui_input_areas: Vec<Rect>,

    /// Where the view's layer is in egui's paint order: one with a higher index is in front.
    pub z_index: usize,
}

impl NativeViewOutput {
    /// Where the view takes the pointer: [`Self::visible`] without
    /// [`Self::egui_input_areas`], as rects that don't overlap.
    pub fn view_input_areas(&self) -> Vec<Rect> {
        subtract(self.visible, &self.egui_input_areas)
    }

    /// Where the view both takes the pointer and shows, with nothing of egui over it:
    /// [`Self::visible`] without [`Self::egui_paint_areas`] and [`Self::egui_input_areas`], as
    /// rects that don't overlap.
    pub fn view_input_and_paint_areas(&self) -> Vec<Rect> {
        let egui_areas = [&self.egui_paint_areas[..], &self.egui_input_areas[..]].concat();
        subtract(self.visible, &egui_areas)
    }
}

/// A [`NativeView`] shown this pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NativeViewShown {
    pub id: Id,
    pub side: NativeViewPlacement,
    pub layer: LayerId,
    pub rect: Rect,
    pub visible: Rect,
}

/// The part of `rect` that `ui` shows, rounded to pixels.
fn visible_rect(ui: &Ui, rect: Rect) -> Rect {
    use emath::GuiRounding as _;

    rect.intersect(ui.clip_rect())
        .round_to_pixels(ui.pixels_per_point())
}

/// Where the views shown this pass go, with the layers in their final order for the pass.
pub(crate) fn outputs(
    shown: &[NativeViewShown],
    memory: &Memory,
    widgets: &WidgetRects,
) -> Vec<NativeViewOutput> {
    if shown.is_empty() {
        return Vec::new();
    }
    let order = memory.layer_ids().collect::<Vec<_>>();
    let areas = memory.areas();
    shown
        .iter()
        .map(|view| {
            let z_index = order
                .iter()
                .rposition(|layer| *layer == view.layer)
                .map_or(order.len(), |i| i + 1);
            let above = &order[z_index..];
            let egui_paint_areas = above
                .iter()
                .filter(|layer| areas.is_visible(layer))
                .filter_map(|layer| memory.area_rect(layer.id))
                .map(|rect| rect.intersect(view.visible))
                .filter(|rect| rect.is_positive())
                .collect();
            let egui_input_areas = above
                .iter()
                .flat_map(|layer| widgets.get_layer(*layer))
                .filter(|widget| widget.sense.interactive())
                .map(|widget| widget.interact_rect.intersect(view.visible))
                .filter(|rect| rect.is_positive())
                .collect();
            NativeViewOutput {
                id: view.id,
                side: view.side,
                rect: view.rect,
                visible: view.visible,
                egui_paint_areas,
                egui_input_areas: without_contained(egui_input_areas),
                z_index,
            }
        })
        .collect()
}

/// `rects` without those inside another, like the widgets inside a window.
fn without_contained(mut rects: Vec<Rect>) -> Vec<Rect> {
    // Largest first, so a rect only has to be checked against those kept before it.
    rects.sort_by(|a, b| b.area().total_cmp(&a.area()));
    let mut kept: Vec<Rect> = Vec::with_capacity(rects.len());
    for rect in rects {
        if !kept.iter().any(|outer| outer.contains_rect(rect)) {
            kept.push(rect);
        }
    }
    kept
}

/// `rect` without `others`, as rects that don't overlap.
fn subtract(rect: Rect, others: &[Rect]) -> Vec<Rect> {
    let mut parts = vec![rect];
    for other in others {
        parts = parts
            .into_iter()
            .flat_map(|part| {
                let cut = part.intersect(*other);
                if !cut.is_positive() {
                    return vec![part];
                }
                // The strips above and below the cut span the full width, the strips left
                // and right of it only its height.
                [
                    Rect::from_x_y_ranges(part.x_range(), part.min.y..=cut.min.y),
                    Rect::from_x_y_ranges(part.x_range(), cut.max.y..=part.max.y),
                    Rect::from_x_y_ranges(part.min.x..=cut.min.x, cut.y_range()),
                    Rect::from_x_y_ranges(cut.max.x..=part.max.x, cut.y_range()),
                ]
                .into_iter()
                .filter(Rect::is_positive)
                .collect()
            })
            .collect();
    }
    parts
}

#[cfg(test)]
mod tests {
    use emath::{Rect, pos2, vec2};

    use super::subtract;
    use crate::{Context, Id, NativeView, NativeViewOutput, RawInput};

    fn area(rects: &[Rect]) -> f32 {
        rects.iter().map(Rect::area).sum()
    }

    #[test]
    fn subtract_one_in_the_middle() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let other = Rect::from_min_size(pos2(25.0, 25.0), vec2(50.0, 50.0));
        let parts = subtract(rect, &[other]);
        assert_eq!(parts.len(), 4, "{parts:?}");
        assert_eq!(area(&parts), 100.0 * 100.0 - 50.0 * 50.0, "{parts:?}");
        assert!(
            parts.iter().all(|part| !part.intersects(other.shrink(0.1))),
            "{parts:?}"
        );
    }

    #[test]
    fn subtract_overlapping_ones() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let others = [
            Rect::from_min_size(pos2(-10.0, -10.0), vec2(60.0, 60.0)),
            Rect::from_min_size(pos2(25.0, 25.0), vec2(50.0, 50.0)),
        ];
        let parts = subtract(rect, &others);
        // 50×50 from the first, plus the 25×25 corners the second adds three of.
        assert_eq!(
            area(&parts),
            100.0 * 100.0 - 50.0 * 50.0 - 3.0 * 25.0 * 25.0
        );
    }

    #[test]
    fn subtract_one_outside() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let other = Rect::from_min_size(pos2(200.0, 0.0), vec2(50.0, 50.0));
        assert_eq!(subtract(rect, &[other]), vec![rect]);
    }

    /// Run passes with a native view of 400×300 at the top left of an 800×600 screen, and
    /// `more` after it, until it has settled. Returns the view's output of the last pass.
    fn view_with(mut more: impl FnMut(&mut crate::Ui)) -> Option<NativeViewOutput> {
        let ctx = Context::default();
        let mut output = None;
        for _ in 0..3 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
                ..Default::default()
            };
            let mut full_output = ctx.run_ui(input, |ui| {
                ui.add(NativeView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
                more(ui);
            });
            full_output.textures_delta.clear();
            output = full_output.platform_output.native_views.into_iter().next();
        }
        output
    }

    #[test]
    fn a_view_alone_has_no_egui_areas() {
        let view = view_with(|_| {}).expect("shown");
        assert!(view.egui_paint_areas.is_empty(), "{view:?}");
        assert!(view.egui_input_areas.is_empty(), "{view:?}");
    }

    #[test]
    fn a_window_over_a_view_is_an_egui_paint_and_input_area() {
        let view = view_with(|ui| {
            crate::Window::new("Over the view")
                .fixed_pos(pos2(100.0, 100.0))
                .show(ui.ctx(), |ui| ui.label("Hello"));
        })
        .expect("shown");
        assert_eq!(view.egui_paint_areas.len(), 1, "{view:?}");
        assert!(
            view.egui_paint_areas[0].contains(pos2(110.0, 110.0)),
            "{view:?}"
        );
        // The window's own widget covers all of its contents. Its resize handles reach a bit
        // past it.
        assert!(
            view.egui_input_areas
                .iter()
                .any(|area| area.contains_rect(view.egui_paint_areas[0])),
            "{view:?}"
        );
    }

    #[test]
    fn a_modal_takes_all_the_input_of_a_view() {
        let view = view_with(|ui| {
            crate::Modal::new(Id::unique("modal")).show(ui.ctx(), |ui| ui.label("Modal"));
        })
        .expect("shown");
        assert!(view.egui_input_areas.contains(&view.visible), "{view:?}");
        assert!(view.view_input_areas().is_empty(), "{view:?}");
    }

    #[test]
    fn only_a_view_below_clears_its_rect() {
        let clears = |view: NativeView| {
            let ctx = Context::default();
            let mut output = ctx.run_ui(RawInput::default(), |ui| {
                ui.add(view.size(vec2(100.0, 100.0)));
            });
            output.textures_delta.clear();
            assert_eq!(output.platform_output.native_views[0].side, view.side);
            output
                .shapes
                .iter()
                .any(|shape| matches!(shape.shape, epaint::Shape::Clear(_)))
        };
        let view = NativeView::new(Id::unique("view"));
        assert!(clears(view));
        assert!(!clears(view.clear(false)));
        assert!(!clears(view.above()));
        assert!(clears(view.above().clear(true)));
    }

    #[test]
    fn a_view_in_an_invisible_ui_is_not_shown() {
        let ctx = Context::default();
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            ui.set_invisible();
            ui.add(NativeView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
        });
        output.textures_delta.clear();
        assert!(output.platform_output.native_views.is_empty());
    }

    #[test]
    fn a_clipped_out_view_is_not_shown() {
        let ctx = Context::default();
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            ui.set_clip_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)));
            ui.add_space(100.0);
            ui.add(NativeView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
        });
        output.textures_delta.clear();
        assert!(output.platform_output.native_views.is_empty());
    }
}
