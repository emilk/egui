use core::ops::RangeInclusive;

use crate::{
    IntoAtoms, Label, NumExt as _, Pos2, Rangef, Rect, Response, Sense, TextWrapMode, Ui, Widget,
    WidgetInfo, WidgetText, WidgetType, emath, style::HandleShape, vec2,
};

use super::drag_value::{GetSetValue, get, set};
use super::slider::{SliderClamping, SliderOrientation};
use super::slider_core::{self, SliderCore, SliderGeometry};

/// One of the two handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle {
    Low,
    High,
}

/// Select a range of numbers with a two-handled slider.
///
/// The handles may meet but never cross.
/// Looks and behaves like [`crate::Slider`], but with two handles.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let (mut low, mut high) = (20.0_f32, 80.0_f32);
/// ui.add(egui::RangeSlider::new(&mut low, &mut high, 0.0..=100.0).text("Range"));
/// # });
/// ```
///
/// The default size is set by [`crate::style::Spacing::slider_width`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct RangeSlider<'a> {
    get_set_low: GetSetValue<'a>,
    get_set_high: GetSetValue<'a>,
    core: SliderCore<'a>,

    /// The closest the two handles may come to each other, in values.
    min_separation: f64,
}

impl<'a> RangeSlider<'a> {
    /// Creates a new horizontal range slider.
    ///
    /// The values are clamped to `range`, and `low` is clamped to `high`.
    pub fn new<Num: emath::Numeric>(
        low: &'a mut Num,
        high: &'a mut Num,
        range: impl Into<RangeInclusive<Num>>,
    ) -> Self {
        let range = range.into();
        let range_f64 = range.start().to_f64()..=range.end().to_f64();
        let slf = Self::from_get_set(
            range_f64,
            move |v: Option<f64>| {
                if let Some(v) = v {
                    *low = Num::from_f64(v);
                }
                low.to_f64()
            },
            move |v: Option<f64>| {
                if let Some(v) = v {
                    *high = Num::from_f64(v);
                }
                high.to_f64()
            },
        );

        if Num::INTEGRAL { slf.integer() } else { slf }
    }

    /// Creates a range slider over values that cannot be borrowed directly.
    pub fn from_get_set(
        range: RangeInclusive<f64>,
        get_set_low: impl 'a + FnMut(Option<f64>) -> f64,
        get_set_high: impl 'a + FnMut(Option<f64>) -> f64,
    ) -> Self {
        Self {
            get_set_low: Box::new(get_set_low),
            get_set_high: Box::new(get_set_high),
            core: SliderCore::new(range),
            min_separation: 0.0,
        }
    }

    /// Show an editable value beside each end of the rail: low before it, high after it.
    ///
    /// Default: `true`.
    #[inline]
    pub fn show_value(mut self, show_value: bool) -> Self {
        self.core.drag_value.show = show_value;
        self
    }

    /// Show a prefix before both numbers. Default: no prefix.
    #[inline]
    pub fn prefix(mut self, prefix: impl IntoAtoms<'a>) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.prefix(prefix);
        self
    }

    /// Add a suffix to both numbers, e.g. a unit. Default: no suffix.
    #[inline]
    pub fn suffix(mut self, suffix: impl IntoAtoms<'a>) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.suffix(suffix);
        self
    }

    /// Show a text label next to the widget.
    #[inline]
    pub fn text(mut self, text: impl Into<WidgetText>) -> Self {
        self.core.text = text.into();
        self
    }

    /// The closest the handles may come, in values rather than points.
    ///
    /// Default: `0.0`, which lets the range collapse to nothing.
    #[inline]
    pub fn min_separation<Num: emath::Numeric>(mut self, min_separation: Num) -> Self {
        self.min_separation = min_separation.to_f64().at_least(0.0);
        self
    }

    /// Which way the rail runs. Default: [`SliderOrientation::Horizontal`].
    #[inline]
    pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
        self.core.orientation = orientation;
        self
    }

    /// Make this a vertical range slider.
    #[inline]
    pub fn vertical(mut self) -> Self {
        self.core.orientation = SliderOrientation::Vertical;
        self
    }

    /// Give the small values as much of the rail as the large ones. Default: `false`.
    #[inline]
    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.core.spec.logarithmic = logarithmic;
        self
    }

    /// For logarithmic sliders that includes zero:
    /// what is the smallest positive value you want to be able to select?
    /// Default: `1e-6`.
    #[inline]
    pub fn smallest_positive(mut self, smallest_positive: f64) -> Self {
        self.core.spec.smallest_positive = smallest_positive;
        self
    }

    /// For logarithmic sliders, the largest positive value we are interested in
    /// before the slider switches to `INFINITY`, if that is the higher end.
    /// Default: `INFINITY`.
    #[inline]
    pub fn largest_finite(mut self, largest_finite: f64) -> Self {
        self.core.spec.largest_finite = largest_finite;
        self
    }

    /// Controls when the values are clamped to the range. Default: [`SliderClamping::Always`].
    #[inline]
    pub fn clamping(mut self, clamping: SliderClamping) -> Self {
        self.core.clamping = clamping;
        self
    }

    /// Guide the values towards round numbers while dragging. Default: `true`.
    #[inline]
    pub fn smart_aim(mut self, smart_aim: bool) -> Self {
        self.core.smart_aim = smart_aim;
        self
    }

    /// Set the minimal change of the values.
    ///
    /// Value `0.0` effectively disables the feature. Default: `0.0`.
    #[inline]
    pub fn step_by(mut self, step: f64) -> Self {
        self.core.step = if step == 0.0 { None } else { Some(step) };
        self
    }

    /// How much the numbers beside the rail change per point of drag.
    ///
    /// Default: the rate the handles move at.
    #[inline]
    pub fn drag_value_speed(mut self, drag_value_speed: f64) -> Self {
        self.core.drag_value.speed = Some(drag_value_speed);
        self
    }

    /// Set the minimum number of decimals to display. Default: `0`.
    #[inline]
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.min_decimals(min_decimals);
        self
    }

    /// Set the maximum number of decimals to display.
    #[inline]
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.max_decimals(max_decimals);
        self
    }

    /// Show exactly this many decimals.
    #[inline]
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.fixed_decimals(num_decimals);
        self
    }

    /// Set a formatter for both numbers.
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.custom_formatter(formatter);
        self
    }

    /// Set a parser for both numbers, accepting what [`Self::custom_formatter`] writes.
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.custom_parser(parser);
        self
    }

    /// Change the shape of both handles. Default: [`crate::style::Visuals::handle_shape`].
    #[inline]
    pub fn handle_shape(mut self, handle_shape: HandleShape) -> Self {
        self.core.handle_shape = Some(handle_shape);
        self
    }

    /// Update the values on each key press while a number is being typed. Default: `true`.
    #[inline]
    pub fn update_while_editing(mut self, update: bool) -> Self {
        self.core.drag_value.format = self.core.drag_value.format.update_while_editing(update);
        self
    }

    /// Helper: equivalent to `self.fixed_decimals(0).smallest_positive(1.0).step_by(1.0)`.
    pub fn integer(self) -> Self {
        self.fixed_decimals(0).smallest_positive(1.0).step_by(1.0)
    }
}

impl RangeSlider<'_> {
    fn get_low(&mut self) -> f64 {
        self.core.existing(get(&mut self.get_set_low))
    }

    fn get_high(&mut self) -> f64 {
        self.core.existing(get(&mut self.get_set_high))
    }

    /// Rounding can land past the other handle when that one is off the step grid, so the
    /// order of the handles is enforced after rounding, not before.
    fn set_low(&mut self, value: f64) {
        let high = self.get_high();
        let value = self.core.rounded(value).at_most(high - self.min_separation);
        set(&mut self.get_set_low, value);
    }

    fn set_high(&mut self, value: f64) {
        let low = self.get_low();
        let value = self.core.rounded(value).at_least(low + self.min_separation);
        set(&mut self.get_set_high, value);
    }

    /// Just the rail and its handles, no numbers.
    fn range_slider_ui(&mut self, ui: &Ui, response: &Response) {
        let geom = self.core.geometry(response.rect, ui);
        let (mut low, mut high) = (self.get_low(), self.get_high());

        if let Some(pointer_position_2d) = response.interact_pointer_pos() {
            // Remembered for the whole gesture, so dragging one handle into the other does not
            // hand the pointer over to its neighbor half way.
            let grabbed = ui
                .data(|data| data.get_temp::<Handle>(response.id))
                .unwrap_or_else(|| {
                    let pointer = geom.pointer_position(pointer_position_2d);
                    let (low_position, high_position) = (
                        geom.position_from_value(low),
                        geom.position_from_value(high),
                    );
                    let (to_low, to_high) = (
                        (pointer - low_position).abs(),
                        (pointer - high_position).abs(),
                    );

                    let nearer_low = if to_low == to_high {
                        // The handles coincide, so the side the pointer is on decides. Otherwise
                        // a collapsed range could only ever be opened in one direction.
                        pointer < low_position
                    } else {
                        to_low < to_high
                    };
                    if nearer_low {
                        Handle::Low
                    } else {
                        Handle::High
                    }
                });
            ui.data_mut(|data| data.insert_temp(response.id, grabbed));

            let value =
                slider_core::value_at_pointer(ui, &geom, pointer_position_2d, self.core.smart_aim);

            (low, high) = moved_handle(grabbed, value, low, high, self.min_separation);

            self.set_low(low);
            self.set_high(high);
        } else if ui
            .data(|data| data.get_temp::<Handle>(response.id))
            .is_some()
        {
            ui.data_mut(|data| data.remove::<Handle>(response.id));
        }

        // Each handle is its own focus stop, so the keyboard and a screen reader can reach
        // either end of the range.
        let handle_rect = |value: f64| {
            let center = geom.marker_center(geom.position_from_value(value), &response.rect);
            Rect::from_center_size(center, vec2(2.0, 2.0) * geom.handle_radius())
        };
        let low_response = ui.interact(
            handle_rect(low),
            response.id.with("low"),
            Sense::focusable_noninteractive(),
        );
        let high_response = ui.interact(
            handle_rect(high),
            response.id.with("high"),
            Sense::focusable_noninteractive(),
        );

        if let Some(value) = self.stepped_by_keyboard(ui, &geom, &low_response, low) {
            (low, high) = moved_handle(Handle::Low, value, low, high, self.min_separation);
            self.set_low(low);
        }
        if let Some(value) = self.stepped_by_keyboard(ui, &geom, &high_response, high) {
            (_, high) = moved_handle(Handle::High, value, low, high, self.min_separation);
            self.set_high(high);
        }

        // Rounding is applied on the way in, so read back what was actually stored.
        let (low, high) = (self.get_low(), self.get_high());

        // A handle may travel as far as its neighbor, not as far as the end of the rail.
        let low_bounds = *self.core.range.start()..=(high - self.min_separation);
        let high_bounds = (low + self.min_separation)..=*self.core.range.end();

        for (handle, value, bounds) in [
            (&low_response, low, low_bounds),
            (&high_response, high, high_bounds),
        ] {
            slider_core::declare_accesskit_slider(
                ui,
                handle.id,
                value,
                &bounds,
                &bounds,
                self.core.step,
            );
            handle
                .widget_info(|| WidgetInfo::slider(ui.is_enabled(), value, self.core.text.text()));
        }

        // Paint it:
        if ui.is_rect_visible(response.rect) {
            let rail_rect = slider_core::paint_rail(ui, &geom);

            let low_center = geom.marker_center(geom.position_from_value(low), &rail_rect);
            let high_center = geom.marker_center(geom.position_from_value(high), &rail_rect);

            // The fill a `Slider` paints behind its handle, bounded at both ends instead of one.
            let span = match self.core.orientation {
                SliderOrientation::Horizontal => Rangef::new(low_center.x, high_center.x),
                SliderOrientation::Vertical => Rangef::new(high_center.y, low_center.y),
            };
            slider_core::paint_fill(ui, rail_rect, span, self.core.orientation);

            // A focused handle looks active, so the keyboard shows where it is.
            let focused = &ui.visuals().widgets.active;
            let dragged = ui.style().interact(response);
            let visuals = |handle: &Response| if handle.has_focus() { focused } else { dragged };

            slider_core::paint_handle(ui, &geom, low_center, visuals(&low_response));
            slider_core::paint_handle(ui, &geom, high_center, visuals(&high_response));
        }
    }

    /// The value the keyboard or a screen reader asked this handle to take, if either did.
    fn stepped_by_keyboard(
        &self,
        ui: &Ui,
        geom: &SliderGeometry,
        handle: &Response,
        value: f64,
    ) -> Option<f64> {
        let steps = slider_core::keyboard_steps(ui, handle, self.core.orientation);
        let stepped = (steps != 0.0)
            .then(|| slider_core::stepped_value(geom, value, steps, &self.core.step_options()));

        // A screen reader naming a value outranks a step, as it does for `Slider`.
        slider_core::accesskit_set_value_request(ui, handle.id).or(stepped)
    }

    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        let desired_size = self.core.desired_size(ui);

        // The numbers are laid out before the rail exists, so their speed is read off a
        // geometry of the right size at an arbitrary position: a gradient only depends on the
        // length.
        let probe = self
            .core
            .geometry(Rect::from_min_size(Pos2::ZERO, desired_size), ui);

        let (mut low, mut high) = (self.get_low(), self.get_high());
        let mut value_responses = Vec::new();

        if self.core.drag_value.show {
            let mut edited = low;
            let bounds = *self.core.range.start()..=(high - self.min_separation);
            let speed = self.core.drag_value_speed_at(ui, &probe, low);
            let response = self.core.drag_value_ui(ui, &mut edited, bounds, speed);
            if edited != low {
                self.set_low(edited);
                low = self.get_low();
            }
            value_responses.push(response);
        }

        let mut response = ui.allocate_response(desired_size, Sense::DRAG);
        self.range_slider_ui(ui, &response);

        let (new_low, new_high) = (self.get_low(), self.get_high());
        if (new_low, new_high) != (low, high) {
            response.mark_changed();
            (low, high) = (new_low, new_high);
        }

        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Other, ui.is_enabled(), self.core.text.text())
        });

        let slider_response = response.clone();

        if self.core.drag_value.show {
            let mut edited = high;
            let bounds = (low + self.min_separation)..=*self.core.range.end();
            let speed = self.core.drag_value_speed_at(ui, &probe, high);
            let value_response = self.core.drag_value_ui(ui, &mut edited, bounds, speed);
            if edited != high {
                self.set_high(edited);
                response.mark_changed();
            }
            value_responses.push(value_response);
        }

        for value_response in &value_responses {
            if value_response.gained_focus()
                || value_response.has_focus()
                || value_response.lost_focus()
            {
                // Use the focused [`DragValue`] id as the id of the whole widget,
                // so that the focus events work as expected.
                response = value_response.clone().union(response);
            } else {
                response = response.union(value_response.clone());
            }
        }

        if !self.core.text.is_empty() {
            let label_response =
                ui.add(Label::new(self.core.text.clone()).wrap_mode(TextWrapMode::Extend));
            slider_response.labelled_by(label_response.id);
            for value_response in &value_responses {
                value_response.clone().labelled_by(label_response.id);
            }
        }

        response
    }
}

/// Where the handles end up when the grabbed one is dragged to `value`.
///
/// Neither pushes past its neighbor, less any separation the widget insists on.
fn moved_handle(
    grabbed: Handle,
    value: f64,
    low: f64,
    high: f64,
    min_separation: f64,
) -> (f64, f64) {
    match grabbed {
        Handle::Low => (value.at_most(high - min_separation), high),
        Handle::High => (low, value.at_least(low + min_separation)),
    }
}

impl Widget for RangeSlider<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let inner_response = match self.core.orientation {
            SliderOrientation::Horizontal => ui.horizontal(|ui| self.add_contents(ui)),
            SliderOrientation::Vertical => ui.vertical(|ui| self.add_contents(ui)),
        };

        inner_response.inner | inner_response.response
    }
}

#[cfg(test)]
mod tests {
    use super::{Handle, moved_handle};

    #[test]
    fn handles_meet_but_never_cross() {
        assert_eq!(
            moved_handle(Handle::Low, 30.0, 10.0, 80.0, 0.0),
            (30.0, 80.0)
        );
        assert_eq!(
            moved_handle(Handle::High, 30.0, 10.0, 80.0, 0.0),
            (10.0, 30.0)
        );

        assert_eq!(
            moved_handle(Handle::Low, 95.0, 10.0, 80.0, 0.0),
            (80.0, 80.0),
            "the low handle stops on the high one"
        );
        assert_eq!(
            moved_handle(Handle::High, 5.0, 10.0, 80.0, 0.0),
            (10.0, 10.0),
            "and the high handle stops on the low one"
        );
    }

    #[test]
    fn handles_keep_their_separation() {
        assert_eq!(
            moved_handle(Handle::Low, 95.0, 10.0, 80.0, 5.0),
            (75.0, 80.0),
            "a handle dragged past its neighbor keeps the separation"
        );
        assert_eq!(
            moved_handle(Handle::High, 5.0, 10.0, 80.0, 5.0),
            (10.0, 15.0)
        );
    }
}
