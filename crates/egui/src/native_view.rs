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
//! egui also reports where it paints over a view ([`NativeViewOutput::egui_paint_areas`]) and
//! where it takes the pointer input from the view ([`NativeViewOutput::egui_input_areas`]). With
//! these, the integration keeps egui's tooltips and popups over a view visible and interactive.

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
    /// The view is under egui's surface and shows through where egui clears the surface to
    /// transparent.
    ///
    /// egui paints over the view, shadows included. The integration has to pass the pointer
    /// input through the surface to the view.
    #[default]
    Below,

    /// The view is over egui's surface, like a platform text field on a phone.
    ///
    /// The view gets its input directly from the platform, but has to mask out what egui paints
    /// over it.
    Above,
}

/// A widget that reserves a rect of the UI for a native view, which the platform draws, such as
/// a web view.
///
/// At the end of the pass, egui reports where the view goes in
/// [`crate::PlatformOutput::native_views`]. A view that is not on screen is missing from that
/// list, and the integration should hide it. That is the case when the widget wasn't added this
/// pass, is in an invisible [`Ui`], or is clipped out, for example by scrolling.
///
/// Added with [`Ui::add`], the widget allocates its rect, and its [`Response`] senses clicks and
/// drags and can take the keyboard focus, so the view takes part in egui's focus handling like
/// any other widget. egui does not see the input that the platform delivers to the view itself.
/// [`Self::show_in`] instead puts the view in a rect that is already allocated, such as the rect
/// of another widget's [`Response`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let id = egui::Id::unique("page");
/// ui.add(egui::NativeView::new(id));
/// ui.add(egui::NativeView::new(id).size(egui::vec2(320.0, 240.0)).above());
///
/// let response = ui.button("A page over a button");
/// egui::NativeView::new(id).show_in(ui, response.rect);
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
    /// Creates the native view `id`, [below](NativeViewPlacement::Below) egui's surface.
    ///
    /// `id` identifies the view in [`NativeViewOutput::id`], so it must be unique among the
    /// native views.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            size: None,
            side: NativeViewPlacement::Below,
            clear: None,
        }
    }

    /// Sets the size of the rect that [`Ui::add`] allocates for the view. Default: all of the
    /// available space.
    ///
    /// [`Self::show_in`] takes its rect from the caller and ignores this.
    #[inline]
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    /// Places the view below or above egui's surface. Default: [`NativeViewPlacement::Below`].
    #[inline]
    pub fn placement(mut self, side: NativeViewPlacement) -> Self {
        self.side = side;
        self
    }

    /// Places the view above egui's surface. egui then doesn't clear the view's rect, unless
    /// [`Self::clear`] asks for it.
    #[inline]
    pub fn above(self) -> Self {
        self.placement(NativeViewPlacement::Above)
    }

    /// Places the view below egui's surface, where it shows through the rect that egui clears.
    /// This is the default.
    #[inline]
    pub fn below(self) -> Self {
        self.placement(NativeViewPlacement::Below)
    }

    /// Whether egui clears the visible part of the view's rect to transparent with
    /// [`epaint::Shape::Clear`], so that a view below egui's surface shows through.
    ///
    /// Default: `true` for a view below egui's surface and `false` for one above it. Turn it off
    /// for a view below the surface that something else makes visible, or that should only show
    /// where egui paints nothing.
    #[inline]
    pub fn clear(mut self, clear: bool) -> Self {
        self.clear = Some(clear);
        self
    }
}

impl NativeView {
    /// Shows the view in `rect` of `ui`, a rect that is already allocated, such as the rect of
    /// a [`Response`].
    ///
    /// Unlike [`Ui::add`], this allocates no space and senses no input, so the view is reported
    /// in [`crate::PlatformOutput::native_views`] but has no [`Response`] of its own: the widget
    /// that owns `rect` keeps egui's clicks, drags and keyboard focus. [`Self::size`] is ignored.
    ///
    /// The view is clipped like the contents of `ui`, and is not shown when `ui` is invisible or
    /// `rect` lies outside its clip rect.
    pub fn show_in(self, ui: &Ui, rect: Rect) {
        let Self {
            id,
            size: _,
            side,
            clear,
        } = self;
        if !ui.is_rect_visible(rect) {
            return;
        }
        if clear.unwrap_or(side == NativeViewPlacement::Below) {
            // The painter's clip rect limits the clear to the visible part of `rect`.
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
}

impl Widget for NativeView {
    fn ui(self, ui: &mut Ui) -> Response {
        let (_, rect) = ui.allocate_space(self.size.unwrap_or_else(|| ui.available_size()));
        let response = ui.interact(rect, self.id, Sense::click_and_drag());
        self.show_in(ui, rect);
        response
    }
}

/// Where a [`NativeView`] goes this pass, as reported in [`crate::PlatformOutput::native_views`].
///
/// All rects are in ui points.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NativeViewOutput {
    /// The id passed to [`NativeView::new`].
    pub id: Id,

    /// Whether the view is below or above egui's surface.
    pub side: NativeViewPlacement,

    /// The view's full rect, including any part that is clipped out.
    pub rect: Rect,

    /// The part of [`Self::rect`] inside the clip rect of the view's [`Ui`], rounded to physical
    /// pixels.
    pub visible: Rect,

    /// The parts of [`Self::visible`] that egui paints over: the rects of the visible areas
    /// (windows, popups, tooltips, …) in the layers in front of the view's layer.
    ///
    /// A view above egui's surface has to mask these out.
    pub egui_paint_areas: Vec<Rect>,

    /// The parts of [`Self::visible`] where egui takes the pointer input from the view: the
    /// interact rects of the interactive widgets in the layers in front of the view's layer,
    /// such as the backdrop of a [`crate::Modal`].
    ///
    /// Unlike [`Self::egui_paint_areas`], this leaves out what egui only paints, such as
    /// tooltips. A rect that lies inside another one of the list is left out.
    pub egui_input_areas: Vec<Rect>,

    /// The position of the view's layer in egui's paint order. A view with a higher `z_index`
    /// is in front of one with a lower `z_index`, and views in the same layer have the same one.
    pub z_index: usize,
}

impl NativeViewOutput {
    /// The parts of [`Self::visible`] where the view takes the pointer input, that is,
    /// outside [`Self::egui_input_areas`].
    ///
    /// The returned rects don't overlap.
    pub fn view_input_areas(&self) -> Vec<Rect> {
        subtract(self.visible, &self.egui_input_areas)
    }

    /// The parts of [`Self::visible`] that egui neither paints over nor takes the pointer input
    /// from: [`Self::visible`] without [`Self::egui_paint_areas`] and
    /// [`Self::egui_input_areas`].
    ///
    /// The returned rects don't overlap.
    pub fn view_input_and_paint_areas(&self) -> Vec<Rect> {
        let egui_areas = [&self.egui_paint_areas[..], &self.egui_input_areas[..]].concat();
        subtract(self.visible, &egui_areas)
    }
}

/// A [`NativeView`] that is on screen this pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NativeViewShown {
    pub id: Id,
    pub side: NativeViewPlacement,
    pub layer: LayerId,
    pub rect: Rect,
    pub visible: Rect,
}

/// The part of `rect` inside the clip rect of `ui`, rounded to physical pixels.
fn visible_rect(ui: &Ui, rect: Rect) -> Rect {
    use emath::GuiRounding as _;

    rect.intersect(ui.clip_rect())
        .round_to_pixels(ui.pixels_per_point())
}

/// Computes the output of the views on screen this pass.
///
/// Call it after [`Memory::end_pass`], which sorts the layers into their paint order for the
/// pass.
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

/// Removes the rects that lie inside another one of `rects`, such as the widgets inside a window.
fn without_contained(mut rects: Vec<Rect>) -> Vec<Rect> {
    // Sorting by area, largest first, means a rect can only lie inside one that was kept before
    // it.
    rects.sort_by(|a, b| b.area().total_cmp(&a.area()));
    let mut kept: Vec<Rect> = Vec::with_capacity(rects.len());
    for rect in rects {
        if !kept.iter().any(|outer| outer.contains_rect(rect)) {
            kept.push(rect);
        }
    }
    kept
}

/// Returns the parts of `rect` outside all of `others`, as rects that don't overlap.
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
                // The strips above and below the cut span the full width of `part`. The strips
                // to the left and right of the cut span only its height, so none overlap.
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
        // The first cuts out 50×50. The second cuts out three more 25×25 squares, because its
        // fourth one overlaps the first.
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

    /// Runs three passes, so the layout settles, with a 400×300 native view at the top left of
    /// an 800×600 screen followed by `more`. Returns the view's output of the last pass.
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
        // The window's own interact rect covers all of its contents, and its resize handles
        // reach a little past them, so one input area contains the whole paint area.
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
