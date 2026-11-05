use ahash::HashMap;

use crate::{Align, Id, IdMap, LayerId, Rangef, Rect, Vec2, WidgetRects, id::IdSet, style};

#[cfg(debug_assertions)]
use crate::{Align2, Color32, FontId, NumExt as _, Painter, pos2, vec2};

/// Reset at the start of each frame.
#[derive(Clone, Debug, Default)]
pub struct TooltipPassState {
    /// If a tooltip has been shown this frame, where was it?
    /// This is used to prevent multiple tooltips to cover each other.
    pub widget_tooltips: IdMap<PerWidgetTooltipState>,
}

impl TooltipPassState {
    pub fn clear(&mut self) {
        let Self { widget_tooltips } = self;
        widget_tooltips.clear();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PerWidgetTooltipState {
    /// Bounding rectangle for all widget and all previous tooltips.
    pub bounding_rect: Rect,

    /// How many tooltips have been shown for this widget this frame?
    pub tooltip_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct PerLayerState {
    /// Is there any open popup (menus, combo-boxes, etc)?
    ///
    /// Does NOT include tooltips.
    pub open_popups: IdSet,

    /// Which widget is showing a tooltip (if any)?
    ///
    /// Only one widget per layer may show a tooltip.
    /// But if a tooltip contains a tooltip, you can show a tooltip on top of a tooltip.
    pub widget_with_tooltip: Option<Id>,
}

#[derive(Clone, Debug)]
pub struct ScrollTarget {
    // The range that the scroll area should scroll to.
    pub range: Rangef,

    /// How should we align the rect within the visible area?
    /// If `align` is [`Align::TOP`] it means "put the top of the rect at the top of the scroll area", etc.
    /// If `align` is `None`, it'll scroll enough to bring the UI into view.
    pub align: Option<Align>,

    /// How should the scroll be animated?
    pub animation: style::ScrollAnimation,
}

impl ScrollTarget {
    pub fn new(range: Rangef, align: Option<Align>, animation: style::ScrollAnimation) -> Self {
        Self {
            range,
            align,
            animation,
        }
    }
}

#[derive(Clone)]
pub struct AccessKitPassState {
    pub nodes: IdMap<accesskit::Node>,
    pub parent_map: IdMap<Id>,
}

#[cfg(debug_assertions)]
#[derive(Clone)]
pub struct DebugRect {
    pub rect: Rect,
    pub callstack: String,
    pub is_clicking: bool,
}

#[cfg(debug_assertions)]
impl DebugRect {
    pub fn paint(self, painter: &Painter) {
        let Self {
            rect,
            callstack,
            is_clicking,
        } = self;

        let ctx = painter.ctx();
        // The reading direction decides which side of the widget the hover-inspection
        // labels hang from: the reading-start side in both directions.
        let rtl = ctx.global_style().right_to_left;

        // Paint rectangle around widget:
        {
            // Print width and height:
            let text_color = if ctx.global_style().visuals.dark_mode {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            // The height label hangs outside the widget on the reading-start side.
            // Mirrored without an overflow gate on purpose: the LTR label already
            // clips at screen edges for edge-hugging widgets, so a mirrored RTL label
            // clipping at the right edge is parity, not a regression — and this keeps
            // the LTR path byte-identical.
            let (h_pos, h_anchor) = if rtl {
                (rect.right_center() + 2.0 * Vec2::RIGHT, Align2::LEFT_CENTER)
            } else {
                (rect.left_center() + 2.0 * Vec2::LEFT, Align2::RIGHT_CENTER)
            };
            painter.debug_text(
                h_pos,
                h_anchor,
                text_color,
                format!("H: {:.1}", rect.height()),
            );
            painter.debug_text(
                rect.center_top(),
                Align2::CENTER_BOTTOM,
                text_color,
                format!("W: {:.1}", rect.width()),
            );

            // Paint rect:
            let rect_fg_color = if is_clicking {
                Color32::WHITE
            } else {
                Color32::LIGHT_BLUE
            };
            let rect_bg_color = Color32::BLUE.gamma_multiply(0.5);
            painter.rect(
                rect,
                0.0,
                rect_bg_color,
                (1.0, rect_fg_color),
                crate::StrokeKind::Outside,
            );
        }

        if !callstack.is_empty() {
            let font_id = FontId::monospace(12.0);
            let text = format!("{callstack}\n\n(click to copy)");
            let text_color = Color32::WHITE;
            let galley = painter.layout_no_wrap(text, font_id, text_color);

            // Position the text either under or above:
            let content_rect = ctx.content_rect();
            let y = if galley.size().y <= rect.top() {
                // Above
                rect.top() - galley.size().y - 16.0
            } else {
                // Below
                rect.bottom()
            };

            let y = y
                .at_most(content_rect.bottom() - galley.size().y)
                .at_least(0.0);

            // The callstack hangs leftward from the reading-start edge of the widget:
            // from `rect.left()` under LTR (clamped into the content rect), from
            // `rect.right()` under RTL (clamped at the content rect's left edge).
            let x = if rtl {
                (rect.right() - galley.size().x).at_least(content_rect.left())
            } else {
                rect.left()
                    .at_most(content_rect.right() - galley.size().x)
                    .at_least(0.0)
            };
            let text_pos = pos2(x, y) - vec2(galley.rect.left(), 0.0);

            let text_bg_color = Color32::from_black_alpha(180);
            let text_rect_stroke_color = if is_clicking {
                Color32::WHITE
            } else {
                text_bg_color
            };
            let text_rect = Rect::from_min_size(text_pos, galley.size());
            painter.rect(
                text_rect,
                0.0,
                text_bg_color,
                (1.0, text_rect_stroke_color),
                crate::StrokeKind::Middle,
            );
            painter.galley(text_pos, galley, text_color);

            if is_clicking {
                ctx.copy_text(callstack);
            }
        }
    }
}

/// State that is collected during a pass, then saved for the next pass,
/// and then cleared.
///
/// (NOTE: we usually run only one pass per frame).
///
/// One per viewport.
#[derive(Clone)]
pub struct PassState {
    /// All [`Id`]s that were used this pass.
    pub used_ids: IdMap<Rect>,

    /// All widgets produced this pass.
    pub widgets: WidgetRects,

    /// Per-layer state.
    ///
    /// Not all layers registers themselves there though.
    pub layers: HashMap<LayerId, PerLayerState>,

    pub tooltips: TooltipPassState,

    /// What the root UI had available at the end of the previous pass.
    ///
    /// Only set if [`crate::Context::run_ui`] or [`crate::Context::root_ui`] has been called.
    pub root_ui_available_rect: Option<Rect>,

    /// What the root UI had used at the end of the previous pass.
    ///
    /// Only set if [`crate::Context::run_ui`] or [`crate::Context::root_ui`] has been called.
    pub root_ui_min_rect: Option<Rect>,

    /// The current scroll area should scroll to this range (horizontal, vertical).
    pub scroll_target: [Option<ScrollTarget>; 2],

    /// The current scroll area should scroll by this much.
    ///
    /// The delta dictates how the _content_ should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    pub scroll_delta: (Vec2, style::ScrollAnimation),

    pub accesskit_state: Option<AccessKitPassState>,

    /// Highlight these widgets the next pass.
    pub highlight_next_pass: IdSet,

    #[cfg(debug_assertions)]
    pub debug_rect: Option<DebugRect>,

    #[cfg(debug_assertions)]
    pub(crate) widget_id_change_warning_exclusions: Vec<Rect>,
}

impl Default for PassState {
    fn default() -> Self {
        Self {
            used_ids: Default::default(),
            widgets: Default::default(),
            layers: Default::default(),
            tooltips: Default::default(),
            root_ui_available_rect: None,
            root_ui_min_rect: None,
            scroll_target: [None, None],
            scroll_delta: (Vec2::default(), style::ScrollAnimation::none()),
            accesskit_state: None,
            highlight_next_pass: Default::default(),

            #[cfg(debug_assertions)]
            debug_rect: None,

            #[cfg(debug_assertions)]
            widget_id_change_warning_exclusions: Vec::new(),
        }
    }
}

impl PassState {
    pub(crate) fn begin_pass(&mut self) {
        profiling::function_scope!();
        let Self {
            used_ids,
            widgets,
            tooltips,
            layers,
            root_ui_available_rect,
            root_ui_min_rect,
            scroll_target,
            scroll_delta,
            accesskit_state,
            highlight_next_pass,

            #[cfg(debug_assertions)]
            debug_rect,

            #[cfg(debug_assertions)]
            widget_id_change_warning_exclusions,
        } = self;

        used_ids.clear();
        widgets.clear();
        tooltips.clear();
        layers.clear();
        *root_ui_available_rect = None;
        *root_ui_min_rect = None;
        *scroll_target = [None, None];
        *scroll_delta = Default::default();

        #[cfg(debug_assertions)]
        {
            *debug_rect = None;
            widget_id_change_warning_exclusions.clear();
        }

        *accesskit_state = None;

        highlight_next_pass.clear();
    }
}

/// Hover-inspection mirroring ([`DebugRect::paint`]) under right-to-left interfaces.
///
/// `DebugRect` is crate-private, so this is an internal test module. Gated on
/// `debug_assertions` like the code under test (and because fonts and
/// `debug_painter`-painted galleys only exist where the debug paths compile in).
#[cfg(all(test, debug_assertions))]
mod tests {
    use crate::{Context, FullOutput, Pos2, RawInput, Rect, epaint::Shape, pos2, vec2};

    /// (text, absolute ink rect) for every text shape painted this pass.
    fn painted_texts(output: &FullOutput) -> Vec<(String, Rect)> {
        fn collect(shape: &Shape, out: &mut Vec<(String, Rect)>) {
            match shape {
                Shape::Text(text) => {
                    out.push((
                        text.galley.job.text.clone(),
                        text.galley.mesh_bounds.translate(text.pos.to_vec2()),
                    ));
                }
                Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut out);
        }
        out
    }

    /// Paint one [`DebugRect`] inside a real pass (fonts must exist to lay the labels
    /// out), with the interface direction set before the frame.
    fn paint_debug_rect(rtl: bool) -> FullOutput {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let mut output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
                ..Default::default()
            },
            |ui| {
                let debug_rect = super::DebugRect {
                    rect: Rect::from_min_size(pos2(200.0, 100.0), vec2(40.0, 20.0)),
                    callstack: "mod_test".into(),
                    is_clicking: false,
                };
                debug_rect.paint(&ui.ctx().debug_painter());
            },
        );
        output.textures_delta.clear(); // these tests never render
        output
    }

    /// Under RTL the height label hangs on the physical-right side of the widget (the
    /// reading-start side) and the callstack is right-aligned to the widget's right
    /// edge; under LTR today's geometry is kept exactly.
    #[test]
    fn hover_inspection_labels_mirror_to_the_reading_start_side() {
        let widget = Rect::from_min_size(pos2(200.0, 100.0), vec2(40.0, 20.0));

        let output = paint_debug_rect(true);
        let texts = painted_texts(&output);
        let h_label = texts
            .iter()
            .find(|(text, _)| text.starts_with("H: "))
            .map(|(_, rect)| rect)
            .expect("the H label should paint");
        assert!(
            h_label.min.x >= widget.right() + 1.0,
            "RTL: the H label should hang right of the widget, got {h_label:?}"
        );
        let callstack = texts
            .iter()
            .find(|(text, _)| text.contains("mod_test"))
            .map(|(_, rect)| rect)
            .expect("the callstack should paint");
        assert!(
            (callstack.max.x - widget.right()).abs() < 1.5,
            "RTL: the callstack should be right-aligned to the widget's right edge, \
             got {callstack:?}"
        );

        let output = paint_debug_rect(false);
        let texts = painted_texts(&output);
        let h_label = texts
            .iter()
            .find(|(text, _)| text.starts_with("H: "))
            .map(|(_, rect)| rect)
            .expect("the H label should paint");
        assert!(
            h_label.max.x <= widget.left() - 1.0,
            "LTR: the H label should hang left of the widget, got {h_label:?}"
        );
        let callstack = texts
            .iter()
            .find(|(text, _)| text.contains("mod_test"))
            .map(|(_, rect)| rect)
            .expect("the callstack should paint");
        assert!(
            (callstack.min.x - widget.left()).abs() < 1.5,
            "LTR: the callstack should start at the widget's left edge, got {callstack:?}"
        );
    }
}
