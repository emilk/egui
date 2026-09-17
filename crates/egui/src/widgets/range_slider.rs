use core::ops::RangeInclusive;

use crate::{
    IntoAtoms, Key, Label, NumExt as _, Pos2, Rangef, Rect, Response, Sense, TextStyle,
    TextWrapMode, Ui, Widget, WidgetInfo, WidgetText, WidgetType, emath, style::HandleShape, vec2,
};

use super::drag_value::clamp_value_to_range;
use super::slider::{SliderClamping, SliderOrientation};
use super::slider_core::{
    self, GetSetValue, SliderGeometry, SliderSpec, StepOptions, ValueOptions, get, set,
};
use super::value_format::ValueFormat;

/// Select a range of numbers with a two-handled slider.
///
/// The handles may meet but never cross. Looks and behaves like [`crate::Slider`], whose
/// geometry, painting and pointer handling it shares.
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
    range: RangeInclusive<f64>,
    spec: SliderSpec,
    clamping: SliderClamping,
    smart_aim: bool,
    show_value: bool,
    orientation: SliderOrientation,
    text: WidgetText,

    /// The closest the two handles may come to each other, in values.
    min_separation: f64,

    /// Sets the minimal step of the widget value
    step: Option<f64>,

    drag_value_speed: Option<f64>,
    format: ValueFormat<'a>,
    handle_shape: Option<HandleShape>,
}

impl<'a> RangeSlider<'a> {
    /// Creates a new horizontal range slider.
    ///
    /// The values are clamped to `range`, and `low` is clamped to `high`.
    pub fn new<Num: emath::Numeric>(
        low: &'a mut Num,
        high: &'a mut Num,
        range: RangeInclusive<Num>,
    ) -> Self {
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
            range,
            spec: SliderSpec {
                logarithmic: false,
                smallest_positive: 1e-6,
                largest_finite: f64::INFINITY,
            },
            clamping: SliderClamping::default(),
            smart_aim: true,
            show_value: true,
            orientation: SliderOrientation::Horizontal,
            text: Default::default(),
            min_separation: 0.0,
            step: None,
            drag_value_speed: None,
            format: ValueFormat::default(),
            handle_shape: None,
        }
    }

    /// Show an editable value beside each end of the rail: low before it, high after it.
    ///
    /// Default: `true`.
    #[inline]
    pub fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    /// Show a prefix before both numbers. Default: no prefix.
    #[inline]
    pub fn prefix(mut self, prefix: impl IntoAtoms<'a>) -> Self {
        self.format = self.format.prefix(prefix);
        self
    }

    /// Add a suffix to both numbers, e.g. a unit. Default: no suffix.
    #[inline]
    pub fn suffix(mut self, suffix: impl IntoAtoms<'a>) -> Self {
        self.format = self.format.suffix(suffix);
        self
    }

    /// Show a text label next to the widget.
    #[inline]
    pub fn text(mut self, text: impl Into<WidgetText>) -> Self {
        self.text = text.into();
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
        self.orientation = orientation;
        self
    }

    /// Make this a vertical range slider.
    #[inline]
    pub fn vertical(mut self) -> Self {
        self.orientation = SliderOrientation::Vertical;
        self
    }

    /// Give the small values as much of the rail as the large ones. Default: `false`.
    #[inline]
    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.spec.logarithmic = logarithmic;
        self
    }

    /// For logarithmic sliders that includes zero:
    /// what is the smallest positive value you want to be able to select?
    /// Default: `1e-6`.
    #[inline]
    pub fn smallest_positive(mut self, smallest_positive: f64) -> Self {
        self.spec.smallest_positive = smallest_positive;
        self
    }

    /// For logarithmic sliders, the largest positive value we are interested in
    /// before the slider switches to `INFINITY`, if that is the higher end.
    /// Default: `INFINITY`.
    #[inline]
    pub fn largest_finite(mut self, largest_finite: f64) -> Self {
        self.spec.largest_finite = largest_finite;
        self
    }

    /// Controls when the values are clamped to the range. Default: [`SliderClamping::Always`].
    #[inline]
    pub fn clamping(mut self, clamping: SliderClamping) -> Self {
        self.clamping = clamping;
        self
    }

    /// Guide the values towards round numbers while dragging. Default: `true`.
    #[inline]
    pub fn smart_aim(mut self, smart_aim: bool) -> Self {
        self.smart_aim = smart_aim;
        self
    }

    /// Set the minimal change of the values.
    ///
    /// Value `0.0` effectively disables the feature. Default: `0.0`.
    #[inline]
    pub fn step_by(mut self, step: f64) -> Self {
        self.step = if step == 0.0 { None } else { Some(step) };
        self
    }

    /// How much the numbers beside the rail change per point of drag.
    ///
    /// Default: the rate the handles move at.
    #[inline]
    pub fn drag_value_speed(mut self, drag_value_speed: f64) -> Self {
        self.drag_value_speed = Some(drag_value_speed);
        self
    }

    /// Set the minimum number of decimals to display. Default: `0`.
    #[inline]
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.format = self.format.min_decimals(min_decimals);
        self
    }

    /// Set the maximum number of decimals to display.
    #[inline]
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.format = self.format.max_decimals(max_decimals);
        self
    }

    /// Show exactly this many decimals.
    #[inline]
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.format = self.format.fixed_decimals(num_decimals);
        self
    }

    /// Set a formatter for both numbers.
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.format = self.format.custom_formatter(formatter);
        self
    }

    /// Set a parser for both numbers, accepting what [`Self::custom_formatter`] writes.
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.format = self.format.custom_parser(parser);
        self
    }

    /// Change the shape of both handles. Default: [`crate::style::Visuals::handle_shape`].
    #[inline]
    pub fn handle_shape(mut self, handle_shape: HandleShape) -> Self {
        self.handle_shape = Some(handle_shape);
        self
    }

    /// Update the values on each key press while a number is being typed. Default: `true`.
    #[inline]
    pub fn update_while_editing(mut self, update: bool) -> Self {
        self.format = self.format.update_while_editing(update);
        self
    }

    /// Helper: equivalent to `self.fixed_decimals(0).smallest_positive(1.0).step_by(1.0)`.
    pub fn integer(self) -> Self {
        self.fixed_decimals(0).smallest_positive(1.0).step_by(1.0)
    }
}

impl RangeSlider<'_> {
    fn get_low(&mut self) -> f64 {
        let value = get(&mut self.get_set_low);
        if self.clamping == SliderClamping::Always {
            clamp_value_to_range(value, self.range.clone())
        } else {
            value
        }
    }

    fn get_high(&mut self) -> f64 {
        let value = get(&mut self.get_set_high);
        if self.clamping == SliderClamping::Always {
            clamp_value_to_range(value, self.range.clone())
        } else {
            value
        }
    }

    /// Rounds `value` to the widget's range, step and decimals.
    fn rounded(&self, mut value: f64) -> f64 {
        if self.clamping != SliderClamping::Never {
            value = clamp_value_to_range(value, self.range.clone());
        }

        if let Some(step) = self.step {
            let start = *self.range.start();
            value = start + ((value - start) / step).round() * step;
        }
        value = self.format.round(value);
        value
    }

    fn set_low(&mut self, value: f64) {
        let value = self.rounded(value);
        set(&mut self.get_set_low, value);
    }

    fn set_high(&mut self, value: f64) {
        let value = self.rounded(value);
        set(&mut self.get_set_high, value);
    }

    fn geometry(&self, rect: Rect, ui: &Ui) -> SliderGeometry {
        SliderGeometry {
            orientation: self.orientation,
            handle_shape: self
                .handle_shape
                .unwrap_or_else(|| ui.style().visuals.handle_shape),
            range: self.range.clone(),
            spec: self.spec.clone(),
            rect,
        }
    }

    fn desired_size(&self, ui: &Ui, thickness: f32) -> emath::Vec2 {
        match self.orientation {
            SliderOrientation::Horizontal => vec2(ui.spacing().slider_width, thickness),
            SliderOrientation::Vertical => vec2(thickness, ui.spacing().slider_width),
        }
    }

    /// Just the rail and its handles, no numbers.
    fn range_slider_ui(&mut self, ui: &Ui, response: &Response) {
        let geom = self.geometry(response.rect, ui);
        let (mut low, mut high) = (self.get_low(), self.get_high());

        if let Some(pointer_position_2d) = response.interact_pointer_pos() {
            // Remembered for the whole gesture, so dragging one handle into the other does not
            // hand the pointer over to its neighbor half way.
            let grabbed_low = ui
                .data(|data| data.get_temp::<bool>(response.id))
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

                    if to_low == to_high {
                        // The handles coincide, so the side the pointer is on decides. Otherwise
                        // a collapsed range could only ever be opened in one direction.
                        pointer < low_position
                    } else {
                        to_low < to_high
                    }
                });
            ui.data_mut(|data| data.insert_temp(response.id, grabbed_low));

            let value =
                slider_core::value_at_pointer(ui, &geom, pointer_position_2d, self.smart_aim);

            (low, high) = moved_handle(grabbed_low, value, low, high, self.min_separation);

            self.set_low(low);
            self.set_high(high);
        } else if ui.data(|data| data.get_temp::<bool>(response.id)).is_some() {
            ui.data_mut(|data| data.remove_temp::<bool>(response.id));
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
            (low, high) = moved_handle(true, value, low, high, self.min_separation);
            self.set_low(low);
        }
        if let Some(value) = self.stepped_by_keyboard(ui, &geom, &high_response, high) {
            (_, high) = moved_handle(false, value, low, high, self.min_separation);
            self.set_high(high);
        }

        // Rounding is applied on the way in, so read back what was actually stored.
        let (low, high) = (self.get_low(), self.get_high());

        // A handle may travel as far as its neighbor, not as far as the end of the rail.
        let low_bounds = *self.range.start()..=(high - self.min_separation);
        let high_bounds = (low + self.min_separation)..=*self.range.end();

        for (handle, value, bounds) in [
            (&low_response, low, low_bounds),
            (&high_response, high, high_bounds),
        ] {
            slider_core::declare_accesskit_slider(
                ui, handle.id, value, &bounds, &bounds, self.step,
            );
            handle.widget_info(|| WidgetInfo::slider(ui.is_enabled(), value, self.text.text()));
        }

        // Paint it:
        if ui.is_rect_visible(response.rect) {
            let rail_rect = slider_core::paint_rail(ui, &geom);

            let low_center = geom.marker_center(geom.position_from_value(low), &rail_rect);
            let high_center = geom.marker_center(geom.position_from_value(high), &rail_rect);

            // The fill a `Slider` paints behind its handle, bounded at both ends instead of one.
            let span = match self.orientation {
                SliderOrientation::Horizontal => Rangef::new(low_center.x, high_center.x),
                SliderOrientation::Vertical => Rangef::new(high_center.y, low_center.y),
            };
            slider_core::paint_fill(ui, rail_rect, span, self.orientation);

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
        let steps = slider_core::keyboard_steps(ui, handle, self.orientation);
        let stepped = (steps != 0.0).then(|| {
            slider_core::stepped_value(
                geom,
                value,
                steps,
                &StepOptions {
                    step: self.step,
                    smart_aim: self.smart_aim,
                    max_decimals: self.format.max_decimals,
                },
            )
        });

        // A screen reader naming a value outranks a step, as it does for `Slider`.
        slider_core::accesskit_set_value_request(ui, handle.id).or(stepped)
    }

    /// One of the two numbers beside the rail.
    ///
    /// `bounds` stops either number being typed past the other.
    fn value_ui(
        &self,
        ui: &mut Ui,
        value: &mut f64,
        bounds: RangeInclusive<f64>,
        speed: f64,
    ) -> Response {
        slider_core::slider_drag_value(
            ui,
            value,
            ValueOptions {
                speed,
                range: bounds,
                clamping: self.clamping,
                format: self.format.clone(),
            },
        )
    }

    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        let thickness = ui
            .text_style_height(&TextStyle::Body)
            .at_least(ui.spacing().interact_size.y);
        let desired_size = self.desired_size(ui, thickness);

        // If a [`DragValue`] is controlled from the keyboard and `step` is defined, set speed to `step`
        let change = ui.input(|input| {
            input.num_presses(Key::ArrowUp) as i32 + input.num_presses(Key::ArrowRight) as i32
                - input.num_presses(Key::ArrowDown) as i32
                - input.num_presses(Key::ArrowLeft) as i32
        });

        let speed = match (self.step, change != 0) {
            (Some(step), true) => step,
            _ => self.drag_value_speed.unwrap_or_else(|| {
                // The numbers are laid out before the rail exists, so the speed is read off a
                // geometry of the right size at an arbitrary position: a gradient only depends
                // on the length.
                let probe = self.geometry(Rect::from_min_size(Pos2::ZERO, desired_size), ui);
                let low = self.get_low();
                probe.gradient_at(low)
            }),
        };

        let (mut low, mut high) = (self.get_low(), self.get_high());
        let mut value_responses = Vec::new();

        if self.show_value {
            let mut edited = low;
            let response = self.value_ui(ui, &mut edited, *self.range.start()..=high, speed);
            if edited != low {
                low = edited.at_most(high - self.min_separation);
                self.set_low(low);
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
            WidgetInfo::labeled(WidgetType::Other, ui.is_enabled(), self.text.text())
        });

        let slider_response = response.clone();

        if self.show_value {
            let mut edited = high;
            let value_response = self.value_ui(ui, &mut edited, low..=*self.range.end(), speed);
            if edited != high {
                high = edited.at_least(low + self.min_separation);
                self.set_high(high);
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

        if !self.text.is_empty() {
            let label_response =
                ui.add(Label::new(self.text.clone()).wrap_mode(TextWrapMode::Extend));
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
    grabbed_low: bool,
    value: f64,
    low: f64,
    high: f64,
    min_separation: f64,
) -> (f64, f64) {
    if grabbed_low {
        (value.at_most(high - min_separation), high)
    } else {
        (low, value.at_least(low + min_separation))
    }
}

impl Widget for RangeSlider<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let inner_response = match self.orientation {
            SliderOrientation::Horizontal => ui.horizontal(|ui| self.add_contents(ui)),
            SliderOrientation::Vertical => ui.vertical(|ui| self.add_contents(ui)),
        };

        inner_response.inner | inner_response.response
    }
}

#[cfg(test)]
mod tests {
    use super::moved_handle;

    #[test]
    fn handles_meet_but_never_cross() {
        assert_eq!(moved_handle(true, 30.0, 10.0, 80.0, 0.0), (30.0, 80.0));
        assert_eq!(moved_handle(false, 30.0, 10.0, 80.0, 0.0), (10.0, 30.0));

        assert_eq!(
            moved_handle(true, 95.0, 10.0, 80.0, 0.0),
            (80.0, 80.0),
            "the low handle stops on the high one"
        );
        assert_eq!(
            moved_handle(false, 5.0, 10.0, 80.0, 0.0),
            (10.0, 10.0),
            "and the high handle stops on the low one"
        );
    }

    #[test]
    fn handles_keep_their_separation() {
        assert_eq!(
            moved_handle(true, 95.0, 10.0, 80.0, 5.0),
            (75.0, 80.0),
            "a handle dragged past its neighbor keeps the separation"
        );
        assert_eq!(moved_handle(false, 5.0, 10.0, 80.0, 5.0), (10.0, 15.0));
    }
}
