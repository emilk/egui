//! The parts of a slider that do not depend on how many handles it has.
//!
//! Shared by [`crate::Slider`] and [`crate::RangeSlider`] so the two cannot drift apart visually.

use core::ops::RangeInclusive;

use crate::{
    DragValue, EventFilter, Id, Key, NumExt as _, Pos2, Rangef, Rect, Response, Ui, Vec2, emath,
    epaint, lerp, pos2, remap, remap_clamp, style, style::HandleShape,
};

use super::slider::{SliderClamping, SliderOrientation};

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
pub type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

pub fn get(get_set_value: &mut GetSetValue<'_>) -> f64 {
    (get_set_value)(None)
}

pub fn set(get_set_value: &mut GetSetValue<'_>, value: f64) {
    (get_set_value)(Some(value));
}

// ----------------------------------------------------------------------------

/// Formats a value for display, given the number of decimals to show.
pub type NumFormatter<'a> = Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;

/// Parses what the user typed, returning `None` if it is not a number.
pub type NumParser<'a> = Box<dyn 'a + Fn(&str) -> Option<f64>>;

// ----------------------------------------------------------------------------

/// How values are spread along a slider's rail.
#[derive(Clone)]
pub struct SliderSpec {
    /// Give the small values as much of the rail as the large ones.
    pub logarithmic: bool,

    /// For logarithmic sliders, the smallest positive value we are interested in.
    /// 1 for integer sliders, maybe 1e-6 for others.
    pub smallest_positive: f64,

    /// For logarithmic sliders, the largest positive value we are interested in
    /// before the slider switches to `INFINITY`, if that is the higher end.
    /// Default: INFINITY.
    pub largest_finite: f64,
}

/// Where a slider's handles may travel, and what a position along that travel means.
pub struct SliderGeometry {
    /// Which way the rail runs.
    pub orientation: SliderOrientation,

    /// Decides how far from the ends a handle may sit.
    pub handle_shape: HandleShape,

    /// What the two ends of the rail stand for. May run high-to-low.
    pub range: RangeInclusive<f64>,

    /// How values are spread between those ends.
    pub spec: SliderSpec,

    /// The space the widget was given, handles included.
    pub rect: Rect,
}

impl SliderGeometry {
    /// Half the width of a handle, before any interaction expansion.
    pub fn handle_radius(&self) -> f32 {
        let limit = match self.orientation {
            SliderOrientation::Horizontal => self.rect.height(),
            SliderOrientation::Vertical => self.rect.width(),
        };
        limit / 2.5
    }

    /// The positions a handle's center may take, inset so a handle at either end stays inside.
    pub fn position_range(&self) -> Rangef {
        let handle_radius = self.handle_radius();
        let handle_radius = match self.handle_shape {
            HandleShape::Circle => handle_radius,
            HandleShape::Rect { aspect_ratio } => handle_radius * aspect_ratio,
        };
        match self.orientation {
            SliderOrientation::Horizontal => self.rect.x_range().shrink(handle_radius),
            // The vertical case has to be flipped because the largest slider value maps to the
            // lowest y value (which is at the top)
            SliderOrientation::Vertical => self.rect.y_range().shrink(handle_radius).flipped(),
        }
    }

    /// The rail, `2 * radius` thick, centered across the widget.
    pub fn rail_rect(&self, radius: f32) -> Rect {
        match self.orientation {
            SliderOrientation::Horizontal => Rect::from_min_max(
                pos2(self.rect.left(), self.rect.center().y - radius),
                pos2(self.rect.right(), self.rect.center().y + radius),
            ),
            SliderOrientation::Vertical => Rect::from_min_max(
                pos2(self.rect.center().x - radius, self.rect.top()),
                pos2(self.rect.center().x + radius, self.rect.bottom()),
            ),
        }
    }

    /// Where to paint a handle that sits at `position_1d` along the rail.
    pub fn marker_center(&self, position_1d: f32, rail_rect: &Rect) -> Pos2 {
        match self.orientation {
            SliderOrientation::Horizontal => pos2(position_1d, rail_rect.center().y),
            SliderOrientation::Vertical => pos2(rail_rect.center().x, position_1d),
        }
    }

    /// The coordinate of `pointer_position_2d` along the rail.
    pub fn pointer_position(&self, pointer_position_2d: Pos2) -> f32 {
        match self.orientation {
            SliderOrientation::Horizontal => pointer_position_2d.x,
            SliderOrientation::Vertical => pointer_position_2d.y,
        }
    }

    /// For instance, `position` is the mouse position along the rail.
    pub fn value_from_position(&self, position: f32) -> f64 {
        let normalized = remap_clamp(position, self.position_range(), 0.0..=1.0) as f64;
        value_from_normalized(normalized, self.range.clone(), &self.spec)
    }

    /// Where along the rail a handle standing for `value` belongs.
    pub fn position_from_value(&self, value: f64) -> f32 {
        let normalized = normalized_from_value(value, self.range.clone(), &self.spec);
        lerp(self.position_range(), normalized as f32)
    }

    /// How much the value changes per point of travel, at `value`.
    ///
    /// The speed of the [`DragValue`] next to the rail, so both gestures move at the same rate.
    pub fn gradient_at(&self, value: f64) -> f64 {
        let position = self.position_from_value(value);
        self.value_from_position(position + 0.5) - self.value_from_position(position - 0.5)
    }
}

// ----------------------------------------------------------------------------
// Painting.

/// Paints the rail and returns it, so the caller can line the fill up with it.
pub fn paint_rail(ui: &Ui, geom: &SliderGeometry) -> Rect {
    let rail_radius = (ui.spacing().slider_rail_height / 2.0).at_least(0.0);
    let rail_rect = geom.rail_rect(rail_radius);
    let widget_visuals = &ui.visuals().widgets;

    ui.painter().rect_filled(
        rail_rect,
        widget_visuals.inactive.corner_radius,
        widget_visuals.inactive.bg_fill,
    );

    rail_rect
}

/// Paints the accent fill over `span` of the rail, in screen coordinates along the rail's axis.
///
/// Overhangs each end by the corner radius so a rounded rail is covered to its tip, clipped to
/// the rail so the overhang cannot escape it.
pub fn paint_fill(ui: &Ui, rail_rect: Rect, span: Rangef, orientation: SliderOrientation) {
    let corner_radius = ui.visuals().widgets.inactive.corner_radius;
    let (span_min, span_max) = (span.min.min(span.max), span.min.max(span.max));

    let mut fill_rect = rail_rect;
    match orientation {
        SliderOrientation::Horizontal => {
            let overhang = corner_radius.nw as f32;
            fill_rect.min.x = (span_min - overhang).at_least(rail_rect.min.x);
            fill_rect.max.x = (span_max + overhang).at_most(rail_rect.max.x);
        }
        SliderOrientation::Vertical => {
            let overhang = corner_radius.se as f32;
            fill_rect.min.y = (span_min - overhang).at_least(rail_rect.min.y);
            fill_rect.max.y = (span_max + overhang).at_most(rail_rect.max.y);
        }
    }

    ui.painter()
        .rect_filled(fill_rect, corner_radius, ui.visuals().selection.bg_fill);
}

/// Paints one handle centered on `center`, in whatever shape the style asks for.
pub fn paint_handle(ui: &Ui, geom: &SliderGeometry, center: Pos2, visuals: &style::WidgetVisuals) {
    let radius = geom.handle_radius();

    match geom.handle_shape {
        HandleShape::Circle => {
            ui.painter().add(epaint::CircleShape {
                center,
                radius: radius + visuals.expansion,
                fill: visuals.bg_fill,
                stroke: visuals.fg_stroke,
            });
        }
        HandleShape::Rect { aspect_ratio } => {
            let v = match geom.orientation {
                SliderOrientation::Horizontal => Vec2::new(radius * aspect_ratio, radius),
                SliderOrientation::Vertical => Vec2::new(radius, radius * aspect_ratio),
            };
            let v = v + Vec2::splat(visuals.expansion);
            let rect = Rect::from_center_size(center, 2.0 * v);
            ui.painter().rect(
                rect,
                visuals.corner_radius,
                visuals.bg_fill,
                visuals.fg_stroke,
                epaint::StrokeKind::Inside,
            );
        }
    }
}

// ----------------------------------------------------------------------------
// Interaction.

/// The value a pointer at `pointer_position_2d` is asking for.
pub fn value_at_pointer(
    ui: &Ui,
    geom: &SliderGeometry,
    pointer_position_2d: Pos2,
    smart_aim: bool,
) -> f64 {
    let position = geom.pointer_position(pointer_position_2d);

    if smart_aim {
        let aim_radius = ui.input(|i| i.aim_radius());
        emath::smart_aim::best_in_range_f64(
            geom.value_from_position(position - aim_radius),
            geom.value_from_position(position + aim_radius),
        )
    } else {
        geom.value_from_position(position)
    }
}

/// How many steps the keyboard and screen reader asked for this frame, negative for down.
///
/// Also locks the arrow keys along the slider's axis, so stepping does not move focus.
pub fn keyboard_steps(ui: &Ui, response: &Response, orientation: SliderOrientation) -> f32 {
    let mut decrement = 0usize;
    let mut increment = 0usize;

    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                response.id,
                EventFilter {
                    // pressing arrows in the orientation of the
                    // slider should not move focus to next widget
                    horizontal_arrows: matches!(orientation, SliderOrientation::Horizontal),
                    vertical_arrows: matches!(orientation, SliderOrientation::Vertical),
                    ..Default::default()
                },
            );
        });

        let (dec_key, inc_key) = match orientation {
            SliderOrientation::Horizontal => (Key::ArrowLeft, Key::ArrowRight),
            // Note that this is for moving the slider position,
            // so up = decrement y coordinate:
            SliderOrientation::Vertical => (Key::ArrowUp, Key::ArrowDown),
        };

        ui.input(|input| {
            decrement += input.num_presses(dec_key);
            increment += input.num_presses(inc_key);
        });
    }

    ui.input(|input| {
        use accesskit::Action;
        decrement += input.num_accesskit_action_requests(response.id, Action::Decrement);
        increment += input.num_accesskit_action_requests(response.id, Action::Increment);
    });

    increment as f32 - decrement as f32
}

/// What [`stepped_value`] needs to know about the widget asking for the step.
pub struct StepOptions {
    /// The smallest change the value may take, if the widget sets one.
    pub step: Option<f64>,

    /// Guide the value towards round numbers.
    pub smart_aim: bool,

    /// Never show more decimals than this, which also decides the smallest visible step.
    pub max_decimals: Option<usize>,
}

/// The value `kb_steps` steps away from `prev_value`.
///
/// One step moves the handle one point along the rail, unless the widget sets its own step.
pub fn stepped_value(
    geom: &SliderGeometry,
    prev_value: f64,
    kb_steps: f32,
    opts: &StepOptions,
) -> f64 {
    let ui_point_per_step = 1.0; // move this many ui points for each kb_step
    let prev_position = geom.position_from_value(prev_value);
    let new_position = prev_position + ui_point_per_step * kb_steps;

    let mut new_value = match opts.step {
        Some(step) => prev_value + (kb_steps as f64 * step),
        None if opts.smart_aim => {
            let aim_radius = 0.49 * ui_point_per_step; // Chosen so we don't include `prev_value` in the search.
            emath::smart_aim::best_in_range_f64(
                geom.value_from_position(new_position - aim_radius),
                geom.value_from_position(new_position + aim_radius),
            )
        }
        _ => geom.value_from_position(new_position),
    };

    if let Some(max_decimals) = opts.max_decimals {
        // `set_value` rounds, so ensure we reach at the least the next breakpoint.
        // Note: we give it a little bit of leeway due to floating point errors. (0.1 isn't representable in binary)
        let min_increment = 1.0 / (10.0_f64.powi(max_decimals as i32));
        new_value = if new_value > prev_value {
            f64::max(new_value, prev_value + min_increment * 1.001)
        } else if new_value < prev_value {
            f64::min(new_value, prev_value - min_increment * 1.001)
        } else {
            new_value
        };
    }

    new_value
}

/// The value a screen reader asked this handle to take.
///
/// The last request of the frame wins, as it did when this was a loop of `set_value` calls.
pub fn accesskit_set_value_request(ui: &Ui, id: Id) -> Option<f64> {
    ui.input(|input| {
        use accesskit::{Action, ActionData};
        input
            .accesskit_action_requests(id, Action::SetValue)
            .filter_map(|request| match request.data {
                Some(ActionData::NumericValue(value)) => Some(value),
                _ => None,
            })
            .last()
    })
}

/// Tells a screen reader what this handle is worth and how far it may go.
///
/// `bounds` is the reported extent; `editable_range` is where a step is actually accepted, which
/// differs when the widget does not clamp.
pub fn declare_accesskit_slider(
    ui: &Ui,
    id: Id,
    value: f64,
    bounds: &RangeInclusive<f64>,
    editable_range: &RangeInclusive<f64>,
    step: Option<f64>,
) {
    ui.ctx().accesskit_node_builder(id, |builder| {
        use accesskit::Action;
        builder.set_min_numeric_value(*bounds.start());
        builder.set_max_numeric_value(*bounds.end());
        if let Some(step) = step {
            builder.set_numeric_value_step(step);
        }
        builder.add_action(Action::SetValue);

        if value < *editable_range.end() {
            builder.add_action(Action::Increment);
        }
        if value > *editable_range.start() {
            builder.add_action(Action::Decrement);
        }
    });
}

// ----------------------------------------------------------------------------
// The number next to the rail.

/// How the numbers beside a rail are written, read back, and applied.
///
/// Everything here is handed straight to the [`DragValue`]s a slider shows.
pub struct ValueFormat<'a> {
    /// Shown before each number, e.g. `"x: "`.
    pub prefix: String,

    /// Shown after each number, e.g. a unit.
    pub suffix: String,

    pub min_decimals: usize,
    pub max_decimals: Option<usize>,
    pub custom_formatter: Option<NumFormatter<'a>>,
    pub custom_parser: Option<NumParser<'a>>,

    /// Apply each keystroke while typing, rather than on enter.
    pub update_while_editing: bool,
}

impl Default for ValueFormat<'_> {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            min_decimals: 0,
            max_decimals: None,
            custom_formatter: None,
            custom_parser: None,
            update_while_editing: true,
        }
    }
}

impl ValueFormat<'_> {
    /// Show exactly this many decimals.
    pub fn set_fixed_decimals(&mut self, num_decimals: usize) {
        self.min_decimals = num_decimals;
        self.max_decimals = Some(num_decimals);
    }

    /// Rounds `value` to `max_decimals`, if set.
    pub fn round(&self, value: f64) -> f64 {
        match self.max_decimals {
            Some(max_decimals) => emath::round_to_decimals(value, max_decimals),
            None => value,
        }
    }

    fn apply<'b>(&'b self, mut dv: DragValue<'b>) -> DragValue<'b> {
        let Self {
            prefix,
            suffix,
            min_decimals,
            max_decimals,
            custom_formatter,
            custom_parser,
            update_while_editing,
        } = self;

        dv = dv
            .prefix(prefix)
            .suffix(suffix)
            .min_decimals(*min_decimals)
            .max_decimals_opt(*max_decimals)
            .update_while_editing(*update_while_editing);

        if let Some(fmt) = custom_formatter {
            dv = dv.custom_formatter(fmt);
        }
        if let Some(parser) = custom_parser {
            dv = dv.custom_parser(parser);
        }
        dv
    }
}

/// How the [`DragValue`] beside a rail is built.
pub struct ValueOptions<'a> {
    /// Value change per point of drag.
    pub speed: f64,

    /// The values this number may take. A range slider bounds each handle by the other.
    pub range: RangeInclusive<f64>,

    /// When values outside `range` are pulled in.
    pub clamping: SliderClamping,

    /// How the number is written and read back.
    pub format: &'a ValueFormat<'a>,
}

/// The editable number that sits next to a rail.
pub fn slider_drag_value(ui: &mut Ui, value: &mut f64, opts: &ValueOptions<'_>) -> Response {
    ui.add({
        let mut dv = opts.format.apply(DragValue::new(value).speed(opts.speed));

        match opts.clamping {
            SliderClamping::Never => {}
            SliderClamping::Edits => {
                dv = dv.range(opts.range.clone()).clamp_existing_to_range(false);
            }
            SliderClamping::Always => {
                dv = dv.range(opts.range.clone()).clamp_existing_to_range(true);
            }
        }

        dv
    })
}

// ----------------------------------------------------------------------------
// Helpers for converting slider range to/from normalized [0-1] range.
// Always clamps.
// Logarithmic sliders are allowed to include zero and infinity,
// even though mathematically it doesn't make sense.

const INFINITY: f64 = f64::INFINITY;

/// When the user asks for an infinitely large range (e.g. logarithmic from zero),
/// give a scale that this many orders of magnitude in size.
const INF_RANGE_MAGNITUDE: f64 = 10.0;

fn value_from_normalized(normalized: f64, range: RangeInclusive<f64>, spec: &SliderSpec) -> f64 {
    let (min, max) = (*range.start(), *range.end());

    if min.is_nan() || max.is_nan() {
        f64::NAN
    } else if min == max {
        min
    } else if min > max {
        value_from_normalized(1.0 - normalized, max..=min, spec)
    } else if normalized <= 0.0 {
        min
    } else if normalized >= 1.0 {
        max
    } else if spec.logarithmic {
        if max <= 0.0 {
            // non-positive range
            -value_from_normalized(normalized, -min..=-max, spec)
        } else if 0.0 <= min {
            let (min_log, max_log) = range_log10(min, max, spec);
            let log = lerp(min_log..=max_log, normalized);
            10.0_f64.powf(log)
        } else {
            assert!(
                min < 0.0 && 0.0 < max,
                "min should be negative and max positive, but got min={min} and max={max}"
            );
            let zero_cutoff = logarithmic_zero_cutoff(min, max);
            if normalized < zero_cutoff {
                // negative
                value_from_normalized(
                    remap(normalized, 0.0..=zero_cutoff, 0.0..=1.0),
                    min..=0.0,
                    spec,
                )
            } else {
                // positive
                value_from_normalized(
                    remap(normalized, zero_cutoff..=1.0, 0.0..=1.0),
                    0.0..=max,
                    spec,
                )
            }
        }
    } else {
        debug_assert!(
            min.is_finite() && max.is_finite(),
            "You should use a logarithmic range"
        );
        lerp(range, normalized.clamp(0.0, 1.0))
    }
}

fn normalized_from_value(value: f64, range: RangeInclusive<f64>, spec: &SliderSpec) -> f64 {
    let (min, max) = (*range.start(), *range.end());

    if min.is_nan() || max.is_nan() {
        f64::NAN
    } else if min == max {
        0.5 // empty range, show center of slider
    } else if min > max {
        1.0 - normalized_from_value(value, max..=min, spec)
    } else if value <= min {
        0.0
    } else if value >= max {
        1.0
    } else if spec.logarithmic {
        if max <= 0.0 {
            // non-positive range
            normalized_from_value(-value, -min..=-max, spec)
        } else if 0.0 <= min {
            let (min_log, max_log) = range_log10(min, max, spec);
            let value_log = value.log10();
            remap_clamp(value_log, min_log..=max_log, 0.0..=1.0)
        } else {
            assert!(
                min < 0.0 && 0.0 < max,
                "min should be negative and max positive, but got min={min} and max={max}"
            );
            let zero_cutoff = logarithmic_zero_cutoff(min, max);
            if value < 0.0 {
                // negative
                remap(
                    normalized_from_value(value, min..=0.0, spec),
                    0.0..=1.0,
                    0.0..=zero_cutoff,
                )
            } else {
                // positive side
                remap(
                    normalized_from_value(value, 0.0..=max, spec),
                    0.0..=1.0,
                    zero_cutoff..=1.0,
                )
            }
        }
    } else {
        debug_assert!(
            min.is_finite() && max.is_finite(),
            "You should use a logarithmic range"
        );
        remap_clamp(value, range, 0.0..=1.0)
    }
}

fn range_log10(min: f64, max: f64, spec: &SliderSpec) -> (f64, f64) {
    assert!(spec.logarithmic, "spec must be logarithmic");
    assert!(
        min <= max,
        "min must be less than or equal to max, but was min={min} and max={max}"
    );

    if min == 0.0 && max == INFINITY {
        (spec.smallest_positive.log10(), INF_RANGE_MAGNITUDE)
    } else if min == 0.0 {
        if spec.smallest_positive < max {
            (spec.smallest_positive.log10(), max.log10())
        } else {
            (max.log10() - INF_RANGE_MAGNITUDE, max.log10())
        }
    } else if max == INFINITY {
        if min < spec.largest_finite {
            (min.log10(), spec.largest_finite.log10())
        } else {
            (min.log10(), min.log10() + INF_RANGE_MAGNITUDE)
        }
    } else {
        (min.log10(), max.log10())
    }
}

/// where to put the zero cutoff for logarithmic sliders
/// that crosses zero ?
fn logarithmic_zero_cutoff(min: f64, max: f64) -> f64 {
    assert!(
        min < 0.0 && 0.0 < max,
        "min must be negative and max positive, but got min={min} and max={max}"
    );

    let min_magnitude = if min == -INFINITY {
        INF_RANGE_MAGNITUDE
    } else {
        min.abs().log10().abs()
    };
    let max_magnitude = if max == INFINITY {
        INF_RANGE_MAGNITUDE
    } else {
        max.log10().abs()
    };

    let cutoff = min_magnitude / (min_magnitude + max_magnitude);
    debug_assert!(
        0.0 <= cutoff && cutoff <= 1.0,
        "Bad cutoff {cutoff:?} for min {min:?} and max {max:?}"
    );
    cutoff
}

#[cfg(test)]
mod tests {
    use super::{SliderGeometry, SliderSpec};
    use crate::widgets::slider::SliderOrientation;
    use crate::{Rect, pos2};

    fn geometry(orientation: SliderOrientation) -> SliderGeometry {
        SliderGeometry {
            orientation,
            handle_shape: crate::style::HandleShape::Circle,
            range: 0.0..=100.0,
            spec: SliderSpec {
                logarithmic: false,
                smallest_positive: 1e-6,
                largest_finite: f64::INFINITY,
            },
            rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 20.0)),
        }
    }

    #[test]
    fn value_position_round_trip() {
        // Vertical sliders run the other way, which is easy to get wrong in only one direction.
        for orientation in [SliderOrientation::Horizontal, SliderOrientation::Vertical] {
            let geom = geometry(orientation);
            for value in [0.0, 1.0, 50.0, 99.0, 100.0] {
                let round_tripped = geom.value_from_position(geom.position_from_value(value));
                assert!(
                    (round_tripped - value).abs() < 0.1,
                    "{orientation:?}: {value} came back as {round_tripped}"
                );
            }
        }
    }

    #[test]
    fn rail_ends_map_to_range_ends() {
        let geom = geometry(SliderOrientation::Horizontal);
        let positions = geom.position_range();
        assert_eq!(geom.value_from_position(positions.min), 0.0);
        assert_eq!(geom.value_from_position(positions.max), 100.0);
    }
}
