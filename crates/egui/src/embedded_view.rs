//! Embedded views: UI handled and rendered by the OS and not egui. For example web views, videos,
//! etc.
//!
//! This is a helper for implementing embedded views. For an example implementation, see
//! [egui_webview](https://github.com/rerun-io/egui_crates/tree/main/crates/egui_webview).
//!
//! Embedded views can either be shown above or below the egui surface. Below usually works better
//! since shadows, backdrops and rounded corners on top of the embedded view can be displayed
//! correctly. Below is not supported on all platforms though, e.g. on Windows it can only be done
//! with the wgpu dx12 backend.
//!
//! egui also reports where it paints over a view ([`EmbeddedViewOutput::egui_paint_areas`]) and
//! where it takes the pointer input from the view ([`EmbeddedViewOutput::egui_input_areas`]). With
//! these, the integration keeps egui's tooltips and popups over a view visible and interactive.

use emath::{Rect, Vec2};

use crate::layers::GraphicLayers;
use crate::{Id, LayerId, Memory, Response, Sense, Ui, Widget, WidgetRects};

/// Whether the [`EmbeddedView`] will be shown above or below the egui surface.
///
/// - Below, it shows through the rect egui clears to transparent, and egui paints over it like
///   over anything else. Input should be handled by the embedded view outside
///   [`EmbeddedViewOutput::egui_input_areas`].
/// - Above, to correctly show egui tooltips and popups, the embedded view has to mask
///   [`EmbeddedViewOutput::egui_paint_areas`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum EmbeddedViewPlacement {
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

/// A widget that reserves a rect of the UI for an embedded view, which the platform draws, such as
/// a web view.
///
/// At the end of the pass, egui reports where the view goes in
/// [`crate::PlatformOutput::embedded_views`]. A view that is not on screen is missing from that
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
/// # let ctx = egui::Context::default();
/// let id = egui::Id::unique("map");
///
/// let output = ctx.run_ui(egui::RawInput::default(), |ui| {
///     // Reserve the rest of the `Ui` for the map. egui draws nothing there.
///     ui.add(egui::EmbeddedView::new(id));
/// });
///
/// // After the pass, move the platform's map view to where egui put the widget.
/// for view in &output.platform_output.embedded_views {
///     if view.id == id {
///         // Show the map in `view.visible`, in front of the layers below `view.z_index`,
///         // and leave the pointer input in `view.egui_input_areas` to egui.
///     }
/// }
/// # let mut output = output;
/// # output.textures_delta.clear(); // Nothing paints this pass.
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedView {
    id: Id,
    size: Option<Vec2>,
    side: EmbeddedViewPlacement,
    clear: Option<bool>,
}

impl EmbeddedView {
    /// Creates the embedded view `id`, [below](EmbeddedViewPlacement::Below) egui's surface.
    ///
    /// `id` identifies the view in [`EmbeddedViewOutput::id`], so it must be unique among the
    /// embedded views.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            size: None,
            side: EmbeddedViewPlacement::Below,
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

    /// Places the view below or above egui's surface. Default: [`EmbeddedViewPlacement::Below`].
    #[inline]
    pub fn placement(mut self, side: EmbeddedViewPlacement) -> Self {
        self.side = side;
        self
    }

    /// Places the view above egui's surface. egui then doesn't clear the view's rect, unless
    /// [`Self::clear`] asks for it.
    #[inline]
    pub fn above(self) -> Self {
        self.placement(EmbeddedViewPlacement::Above)
    }

    /// Places the view below egui's surface, where it shows through the rect that egui clears.
    /// This is the default.
    #[inline]
    pub fn below(self) -> Self {
        self.placement(EmbeddedViewPlacement::Below)
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

impl EmbeddedView {
    /// Shows the view in `rect` of `ui`, a rect that is already allocated, such as the rect of
    /// a [`Response`].
    ///
    /// Unlike [`Ui::add`], this allocates no space and senses no input, so the view is reported
    /// in [`crate::PlatformOutput::embedded_views`] but has no [`Response`] of its own: the widget
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
        if clear.unwrap_or(side == EmbeddedViewPlacement::Below) {
            // The painter's clip rect limits the clear to the visible part of `rect`.
            ui.painter().add(epaint::Shape::Clear(rect));
        }
        let shown = EmbeddedViewShown {
            id,
            side,
            layer: ui.layer_id(),
            rect,
            visible: rect.intersect(ui.clip_rect()),
        };
        ui.ctx()
            .pass_state_mut(|state| state.embedded_views.push(shown));
    }
}

impl Widget for EmbeddedView {
    fn ui(self, ui: &mut Ui) -> Response {
        let (_, rect) = ui.allocate_space(self.size.unwrap_or_else(|| ui.available_size()));
        let response = ui.interact(rect, self.id, Sense::click_and_drag());
        self.show_in(ui, rect);
        response
    }
}

/// Where an [`EmbeddedView`] goes this pass, as reported in
/// [`crate::PlatformOutput::embedded_views`].
///
/// All rects are in ui points, in the coordinates of the viewport: a view in a layer that is
/// transformed, such as the contents of a [`crate::Scene`], is reported where it ends up on
/// screen.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EmbeddedViewOutput {
    /// The id passed to [`EmbeddedView::new`].
    pub id: Id,

    /// Whether the view is below or above egui's surface.
    pub side: EmbeddedViewPlacement,

    /// How the views stack among each other: a view with a higher `z_index` is in front of one
    /// with a lower `z_index`.
    ///
    /// It is the position of the view's egui layer ([`LayerId`]) in egui's paint order, counted
    /// over all of egui's layers, the ones of [`crate::Area`]s included. So it is not a count of
    /// the embedded views, the numbers have gaps, and views in the same layer share one.
    ///
    /// Only compare views with the same [`Self::side`]: every view above egui's surface is in
    /// front of every view below it, whatever their `z_index`.
    pub z_index: usize,

    /// The view's full rect, including any part that is clipped out.
    pub rect: Rect,

    /// The part of [`Self::rect`] inside the clip rect of the view's [`Ui`], rounded to physical
    /// pixels.
    pub visible: Rect,

    /// The parts of [`Self::visible`] that egui paints over: the rects of the visible areas
    /// (windows, popups, tooltips, …) in the egui layers ([`LayerId`]) in front of the view's
    /// layer, and the bounds of what is painted to a layer in front that isn't an area, such as
    /// a drag-and-drop preview or [`crate::Context::debug_painter`].
    ///
    /// A view above egui's surface has to mask these out.
    ///
    /// Only other layers count. What is painted over the view in the view's own layer, such as
    /// a button put over it, is not in the list, nor in [`Self::egui_input_areas`]: show such a
    /// widget in a layer in front, for example in an [`crate::Area`].
    pub egui_paint_areas: Vec<Rect>,

    /// The parts of [`Self::visible`] where egui takes the pointer input from the view: the
    /// interact rects of the interactive widgets in the layers in front of the view's layer,
    /// such as the backdrop of a [`crate::Modal`].
    ///
    /// Unlike [`Self::egui_paint_areas`], this leaves out what egui only paints, such as
    /// tooltips. A rect that lies inside another one of the list is left out. Like that list,
    /// it leaves out the widgets in the view's own layer.
    pub egui_input_areas: Vec<Rect>,
}

impl EmbeddedViewOutput {
    /// The parts of [`Self::visible`] where the view gets the pointer input: everything outside
    /// [`Self::egui_input_areas`].
    ///
    /// Use it to decide who gets the pointer. It applies to both placements:
    ///
    /// * **Below:** egui's surface is in front of the view and would take every click. Let the
    ///   input through to the view in these rects.
    /// * **Above:** the view is in front and gets every click by default. Take the input away
    ///   from the view outside these rects, for example under a window or a modal backdrop.
    ///
    /// If it is empty, egui takes all the input, as it does under a modal.
    ///
    /// The returned rects don't overlap.
    pub fn view_input_areas(&self) -> Vec<Rect> {
        self.visible.subtract(&self.egui_input_areas)
    }

    /// The parts of [`Self::visible`] where the view shows: everything outside
    /// [`Self::egui_paint_areas`].
    ///
    /// Use it as the mask of a view above egui's surface. The view covers egui, so egui's
    /// windows, popups and tooltips only show over it if the view is cut out there. A view
    /// below the surface doesn't need a mask: egui paints over the view by itself.
    ///
    /// Use it when the platform can mask drawing separately from input, and route the input
    /// with [`Self::view_input_areas`]. Under a tooltip the view is then hidden but still gets
    /// the click, which is right because a tooltip takes no input. If one shape has to do both,
    /// use [`Self::view_input_and_paint_areas`] instead.
    ///
    /// The returned rects don't overlap.
    pub fn view_paint_areas(&self) -> Vec<Rect> {
        self.visible.subtract(&self.egui_paint_areas)
    }

    /// The parts of [`Self::visible`] that egui neither paints over nor takes the pointer input
    /// from: everything outside [`Self::egui_paint_areas`] and [`Self::egui_input_areas`].
    ///
    /// Use it for a view above egui's surface when a single shape clips both drawing and
    /// hit-testing. A Win32 window region and a CSS `clip-path` work that way.
    ///
    /// The cost: the view also disappears where egui takes input but paints nothing, such as
    /// under a widget that only senses clicks. If the platform can separate the two, use
    /// [`Self::view_paint_areas`] and [`Self::view_input_areas`] instead.
    ///
    /// Don't use it for a view below the surface. Nothing needs masking there, and it would
    /// block the view's input under tooltips. Use [`Self::view_input_areas`].
    ///
    /// The returned rects don't overlap.
    pub fn view_input_and_paint_areas(&self) -> Vec<Rect> {
        let egui_areas = [&self.egui_paint_areas[..], &self.egui_input_areas[..]].concat();
        self.visible.subtract(&egui_areas)
    }
}

/// An [`EmbeddedView`] that is on screen this pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EmbeddedViewShown {
    pub id: Id,
    pub side: EmbeddedViewPlacement,
    pub layer: LayerId,

    /// In the coordinates of [`Self::layer`], like [`Self::visible`].
    pub rect: Rect,

    /// The part of [`Self::rect`] inside the clip rect of the view's [`Ui`].
    pub visible: Rect,
}

/// Computes the output of the views on screen this pass.
///
/// Call it after [`Memory::end_pass`], which sorts the layers into their paint order for the
/// pass, and before the shapes of `graphics` are drained.
pub(crate) fn outputs(
    shown: &[EmbeddedViewShown],
    memory: &Memory,
    widgets: &WidgetRects,
    graphics: &GraphicLayers,
    pixels_per_point: f32,
) -> Vec<EmbeddedViewOutput> {
    use emath::GuiRounding as _;

    if shown.is_empty() {
        return Vec::new();
    }
    let order = memory.layer_ids().collect::<Vec<_>>();
    let areas = memory.areas();
    // Rects are recorded in the coordinates of their layer.
    let to_global = |layer: LayerId, rect: Rect| match memory.to_global.get(&layer) {
        Some(to_global) => *to_global * rect,
        None => rect,
    };

    // The layers that aren't areas, such as the one of a drag-and-drop preview. egui paints
    // them after the areas of the same `Order`, see `GraphicLayers::drain`.
    let mut other_layers: Vec<LayerId> =
        core::iter::chain(graphics.painted_layer_ids(), widgets.layer_ids())
            .filter(|layer| !order.contains(layer))
            .collect();
    other_layers.sort_by_key(|layer| (layer.order, layer.id.value()));
    other_layers.dedup();

    shown
        .iter()
        .map(|view| {
            let rect = to_global(view.layer, view.rect);
            let visible = to_global(view.layer, view.visible).round_to_pixels(pixels_per_point);

            let in_order = order.iter().rposition(|layer| *layer == view.layer);
            // A view in a layer that isn't an area is in front of the areas of its `Order`.
            let z_index = in_order.map_or_else(
                || {
                    order
                        .iter()
                        .rposition(|layer| layer.order <= view.layer.order)
                        .map_or(0, |i| i + 1)
                },
                |i| i + 1,
            );
            let areas_above = &order[z_index..];
            let others_above: Vec<LayerId> = other_layers
                .iter()
                .copied()
                .filter(|layer| *layer != view.layer)
                .filter(|layer| {
                    view.layer.order < layer.order
                        || (view.layer.order == layer.order && in_order.is_some())
                })
                .collect();

            let area_rects = areas_above
                .iter()
                .filter(|layer| areas.is_visible(layer))
                .filter_map(|layer| Some(to_global(*layer, memory.area_rect(layer.id)?)));
            let painted_bounds = others_above.iter().filter_map(|layer| {
                let bounds = graphics
                    .get(*layer)?
                    .all_entries()
                    .map(|shape| {
                        shape
                            .shape
                            .visual_bounding_rect()
                            .intersect(shape.clip_rect)
                    })
                    .filter(Rect::is_positive)
                    .reduce(Rect::union)?;
                Some(to_global(*layer, bounds))
            });
            let egui_paint_areas = core::iter::chain(area_rects, painted_bounds)
                .map(|rect| rect.intersect(visible))
                .filter(Rect::is_positive)
                .collect();

            let egui_input_areas = core::iter::chain(areas_above, &others_above)
                .flat_map(|layer| widgets.get_layer(*layer))
                .filter(|widget| widget.sense.interactive())
                .map(|widget| to_global(widget.layer_id, widget.interact_rect).intersect(visible))
                .filter(Rect::is_positive)
                .collect();

            EmbeddedViewOutput {
                id: view.id,
                side: view.side,
                rect,
                visible,
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

#[cfg(test)]
mod tests {
    use emath::{Rect, pos2, vec2};

    use crate::{Context, EmbeddedView, EmbeddedViewOutput, Id, LayerId, Order, RawInput};

    fn view_with(mut more: impl FnMut(&mut crate::Ui)) -> Option<EmbeddedViewOutput> {
        let ctx = Context::default();
        let mut output = None;
        for _ in 0..3 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
                ..Default::default()
            };
            let mut full_output = ctx.run_ui(input, |ui| {
                ui.add(EmbeddedView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
                more(ui);
            });
            full_output.textures_delta.clear();
            output = full_output
                .platform_output
                .embedded_views
                .into_iter()
                .next();
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
        let clears = |view: EmbeddedView| {
            let ctx = Context::default();
            let mut output = ctx.run_ui(RawInput::default(), |ui| {
                ui.add(view.size(vec2(100.0, 100.0)));
            });
            output.textures_delta.clear();
            assert_eq!(output.platform_output.embedded_views[0].side, view.side);
            output
                .shapes
                .iter()
                .any(|shape| matches!(shape.shape, epaint::Shape::Clear(_)))
        };
        let view = EmbeddedView::new(Id::unique("view"));
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
            ui.add(EmbeddedView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
        });
        output.textures_delta.clear();
        assert!(output.platform_output.embedded_views.is_empty());
    }

    #[test]
    fn a_clipped_out_view_is_not_shown() {
        let ctx = Context::default();
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            ui.set_clip_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)));
            ui.add_space(100.0);
            ui.add(EmbeddedView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
        });
        output.textures_delta.clear();
        assert!(output.platform_output.embedded_views.is_empty());
    }

    /// Runs three passes of `add` and returns the views of the last one.
    fn views_of(mut add: impl FnMut(&mut crate::Ui)) -> Vec<EmbeddedViewOutput> {
        let ctx = Context::default();
        let mut views = Vec::new();
        for _ in 0..3 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
                ..Default::default()
            };
            let mut full_output = ctx.run_ui(input, |ui| add(ui));
            full_output.textures_delta.clear();
            views = full_output.platform_output.embedded_views;
        }
        views
    }

    #[test]
    fn a_view_in_a_transformed_layer_is_reported_where_it_is_on_screen() {
        let layer = LayerId::new(Order::Middle, Id::unique("moved"));
        let to_global = emath::TSTransform::new(vec2(100.0, 50.0), 2.0);
        let views = views_of(|ui| {
            ui.ctx().set_transform_layer(layer, to_global);
            crate::Area::new(layer.id)
                .order(layer.order)
                .fixed_pos(pos2(10.0, 20.0))
                .show(ui.ctx(), |ui| {
                    ui.add(EmbeddedView::new(Id::unique("view")).size(vec2(100.0, 80.0)));
                });
        });
        let in_layer = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 80.0));
        assert_eq!(views[0].rect, to_global * in_layer, "{views:?}");
        assert_eq!(views[0].visible, to_global * in_layer, "{views:?}");
    }

    #[test]
    fn a_button_put_over_a_view_in_its_own_layer_is_not_reported() {
        let button = Rect::from_min_size(pos2(50.0, 50.0), vec2(80.0, 20.0));
        let view = view_with(|ui| {
            ui.put(button, crate::Button::new("Over the view"));
        })
        .expect("shown");
        // Neither as painted over the view nor as taking its input: the two lists agree.
        assert!(view.egui_paint_areas.is_empty(), "{view:?}");
        assert!(view.egui_input_areas.is_empty(), "{view:?}");
    }

    #[test]
    fn a_view_in_a_layer_that_is_not_an_area_knows_what_is_above_it() {
        let layer = LayerId::new(Order::Background, Id::unique("not an area"));
        let views = views_of(|ui| {
            ui.scope_builder(crate::UiBuilder::new().layer_id(layer), |ui| {
                ui.add(EmbeddedView::new(Id::unique("view")).size(vec2(400.0, 300.0)));
            });
            crate::Window::new("Over the view")
                .fixed_pos(pos2(100.0, 100.0))
                .show(ui.ctx(), |ui| ui.label("Hello"));
        });
        assert_eq!(views[0].egui_paint_areas.len(), 1, "{views:?}");
        assert!(!views[0].egui_input_areas.is_empty(), "{views:?}");
    }

    #[test]
    fn painting_to_a_layer_in_front_that_is_not_an_area_is_a_paint_area() {
        // Like the preview of a drag-and-drop.
        let painted = Rect::from_min_size(pos2(50.0, 50.0), vec2(40.0, 30.0));
        let view = view_with(|ui| {
            let layer = LayerId::new(Order::Tooltip, Id::unique("preview"));
            ui.ctx()
                .layer_painter(layer)
                .rect_filled(painted, 0.0, crate::Color32::RED);
        })
        .expect("shown");
        assert_eq!(view.egui_paint_areas.len(), 1, "{view:?}");
        assert!(view.egui_paint_areas[0].contains_rect(painted), "{view:?}");
        assert!(view.egui_input_areas.is_empty(), "{view:?}");
    }
}
