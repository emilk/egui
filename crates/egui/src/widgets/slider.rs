use core::ops::RangeInclusive;

use crate::{
    Color32, IntoAtoms, Key, Label, NumExt as _, Rangef, Rect, Response, Sense, TextStyle,
    TextWrapMode, Ui, Widget, WidgetInfo, WidgetText, emath, style::HandleShape, vec2,
};

use super::drag_value::clamp_value_to_range;
use super::slider_core::{
    self, GetSetValue, SliderGeometry, SliderSpec, StepOptions, ValueOptions, get, set,
};
use super::value_format::ValueFormat;
// ----------------------------------------------------------------------------

/// Specifies the orientation of a [`Slider`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

/// Specifies how values in a [`Slider`] are clamped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SliderClamping {
    /// Values are not clamped.
    ///
    /// This means editing the value with the keyboard,
    /// or dragging the number next to the slider will always work.
    ///
    /// The actual slider part is always clamped though.
    Never,

    /// Users cannot enter new values that are outside the range.
    ///
    /// Existing values remain intact though.
    Edits,

    /// Always clamp values, even existing ones.
    #[default]
    Always,
}

/// Control a number with a slider.
///
/// The slider range defines the values you get when pulling the slider to the far edges.
/// By default all values are clamped to this range, even when not interacted with.
/// You can change this behavior by passing `false` to [`Slider::clamping`].
///
/// The range can include any numbers, and go from low-to-high or from high-to-low.
///
/// The slider consists of three parts: a slider, a value display, and an optional text.
/// The user can click the value display to edit its value. It can be turned off with `.show_value(false)`.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_f32: f32 = 0.0;
/// ui.add(egui::Slider::new(&mut my_f32, 0.0..=100.0).text("My value"));
/// # });
/// ```
///
/// The default [`Slider`] size is set by [`crate::style::Spacing::slider_width`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Slider<'a> {
    get_set_value: GetSetValue<'a>,
    range: RangeInclusive<f64>,
    spec: SliderSpec,
    clamping: SliderClamping,
    smart_aim: bool,
    show_value: bool,
    orientation: SliderOrientation,
    text: WidgetText,

    /// Sets the minimal step of the widget value
    step: Option<f64>,

    drag_value_speed: Option<f64>,
    format: ValueFormat<'a>,
    trailing_fill: Option<bool>,
    handle_shape: Option<HandleShape>,
}

impl<'a> Slider<'a> {
    /// Creates a new horizontal slider.
    ///
    /// The `value` given will be clamped to the `range`,
    /// unless you change this behavior with [`Self::clamping`].
    pub fn new<Num: emath::Numeric>(
        value: &'a mut Num,
        range: impl Into<RangeInclusive<Num>>,
    ) -> Self {
        let range = range.into();
        let range_f64 = range.start().to_f64()..=range.end().to_f64();
        let slf = Self::from_get_set(range_f64, move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });

        if Num::INTEGRAL { slf.integer() } else { slf }
    }

    pub fn from_get_set(
        range: RangeInclusive<f64>,
        get_set_value: impl 'a + FnMut(Option<f64>) -> f64,
    ) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
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
            step: None,
            drag_value_speed: None,
            format: ValueFormat::default(),
            trailing_fill: None,
            handle_shape: None,
        }
    }

    /// Control whether or not the slider shows the current value.
    /// Default: `true`.
    #[inline]
    pub fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    /// Show a prefix before the number, e.g. "x: "
    #[inline]
    pub fn prefix(mut self, prefix: impl IntoAtoms<'a>) -> Self {
        self.format = self.format.prefix(prefix);
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m")
    #[inline]
    pub fn suffix(mut self, suffix: impl IntoAtoms<'a>) -> Self {
        self.format = self.format.suffix(suffix);
        self
    }

    /// Show a text next to the slider (e.g. explaining what the slider controls).
    #[inline]
    pub fn text(mut self, text: impl Into<WidgetText>) -> Self {
        self.text = text.into();
        self
    }

    #[inline]
    pub fn text_color(mut self, text_color: Color32) -> Self {
        self.text = self.text.color(text_color);
        self
    }

    /// Vertical or horizontal slider? The default is horizontal.
    #[inline]
    pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Make this a vertical slider.
    #[inline]
    pub fn vertical(mut self) -> Self {
        self.orientation = SliderOrientation::Vertical;
        self
    }

    /// Make this a logarithmic slider.
    /// This is great for when the slider spans a huge range,
    /// e.g. from one to a million.
    /// The default is OFF.
    #[inline]
    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.spec.logarithmic = logarithmic;
        self
    }

    /// For logarithmic sliders that includes zero:
    /// what is the smallest positive value you want to be able to select?
    /// The default is `1` for integer sliders and `1e-6` for real sliders.
    #[inline]
    pub fn smallest_positive(mut self, smallest_positive: f64) -> Self {
        self.spec.smallest_positive = smallest_positive;
        self
    }

    /// For logarithmic sliders, the largest positive value we are interested in
    /// before the slider switches to `INFINITY`, if that is the higher end.
    /// Default: INFINITY.
    #[inline]
    pub fn largest_finite(mut self, largest_finite: f64) -> Self {
        self.spec.largest_finite = largest_finite;
        self
    }

    /// Controls when the values will be clamped to the range.
    ///
    /// ### With `.clamping(SliderClamping::Always)` (default)
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut my_value: f32 = 1337.0;
    /// ui.add(egui::Slider::new(&mut my_value, 0.0..=1.0));
    /// assert!(0.0 <= my_value && my_value <= 1.0, "Existing value should be clamped");
    /// # });
    /// ```
    ///
    /// ### With `.clamping(SliderClamping::Edits)`
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut my_value: f32 = 1337.0;
    /// let response = ui.add(
    ///     egui::Slider::new(&mut my_value, 0.0..=1.0)
    ///         .clamping(egui::SliderClamping::Edits)
    /// );
    /// if response.dragged() {
    ///     // The user edited the value, so it should now be clamped to the range
    ///     assert!(0.0 <= my_value && my_value <= 1.0);
    /// }
    /// # });
    /// ```
    ///
    /// ### With `.clamping(SliderClamping::Never)`
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut my_value: f32 = 1337.0;
    /// let response = ui.add(
    ///     egui::Slider::new(&mut my_value, 0.0..=1.0)
    ///         .clamping(egui::SliderClamping::Never)
    /// );
    /// // The user could have set the value to anything
    /// # });
    /// ```
    #[inline]
    pub fn clamping(mut self, clamping: SliderClamping) -> Self {
        self.clamping = clamping;
        self
    }

    /// Turn smart aim on/off. Default is ON.
    /// There is almost no point in turning this off.
    #[inline]
    pub fn smart_aim(mut self, smart_aim: bool) -> Self {
        self.smart_aim = smart_aim;
        self
    }

    /// Sets the minimal change of the value.
    ///
    /// Value `0.0` effectively disables the feature. If the new value is out of range
    /// and `clamp_to_range` is enabled, you would not have the ability to change the value.
    ///
    /// Default: `0.0` (disabled).
    #[inline]
    pub fn step_by(mut self, step: f64) -> Self {
        self.step = if step == 0.0 { None } else { Some(step) };
        self
    }

    /// When dragging the value, how fast does it move?
    ///
    /// Unit: values per point (logical pixel).
    /// See also [`DragValue::speed`].
    ///
    /// By default this is the same speed as when dragging the slider,
    /// but you can change it here to for instance have a much finer control
    /// by dragging the slider value rather than the slider itself.
    #[inline]
    pub fn drag_value_speed(mut self, drag_value_speed: f64) -> Self {
        self.drag_value_speed = Some(drag_value_speed);
        self
    }

    // TODO(emilk): we should also have a "min precision".
    /// Set a minimum number of decimals to display.
    ///
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    #[inline]
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.format = self.format.min_decimals(min_decimals);
        self
    }

    // TODO(emilk): we should also have a "max precision".
    /// Set a maximum number of decimals to display.
    ///
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    #[inline]
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.format = self.format.max_decimals(max_decimals);
        self
    }

    #[inline]
    pub fn max_decimals_opt(mut self, max_decimals: Option<usize>) -> Self {
        self.format = self.format.max_decimals_opt(max_decimals);
        self
    }

    /// Set an exact number of decimals to display.
    ///
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    #[inline]
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.format = self.format.fixed_decimals(num_decimals);
        self
    }

    /// Display trailing color behind the slider's circle. Default is OFF.
    ///
    /// This setting can be enabled globally for all sliders with [`crate::Visuals::slider_trailing_fill`].
    /// Toggling it here will override the above setting ONLY for this individual slider.
    ///
    /// The fill color will be taken from `selection.bg_fill` in your [`crate::Visuals`], the same as a [`crate::ProgressBar`].
    #[inline]
    pub fn trailing_fill(mut self, trailing_fill: bool) -> Self {
        self.trailing_fill = Some(trailing_fill);
        self
    }

    /// Change the shape of the slider handle
    ///
    /// This setting can be enabled globally for all sliders with [`crate::Visuals::handle_shape`].
    /// Changing it here will override the above setting ONLY for this individual slider.
    #[inline]
    pub fn handle_shape(mut self, handle_shape: HandleShape) -> Self {
        self.handle_shape = Some(handle_shape);
        self
    }

    /// Set custom formatter defining how numbers are converted into text.
    ///
    /// A custom formatter takes a `f64` for the numeric value and a `RangeInclusive<usize>` representing
    /// the decimal range i.e. minimum and maximum number of decimal places shown.
    ///
    /// The default formatter is [`crate::Style::number_formatter`].
    ///
    /// See also: [`Slider::custom_parser`]
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::Slider::new(&mut my_i32, 0..=((60 * 60 * 24) - 1))
    ///     .custom_formatter(|n, _| {
    ///         let n = n as i32;
    ///         let hours = n / (60 * 60);
    ///         let mins = (n / 60) % 60;
    ///         let secs = n % 60;
    ///         format!("{hours:02}:{mins:02}:{secs:02}")
    ///     })
    ///     .custom_parser(|s| {
    ///         let parts: Vec<&str> = s.split(':').collect();
    ///         if parts.len() == 3 {
    ///             parts[0].parse::<i32>().and_then(|h| {
    ///                 parts[1].parse::<i32>().and_then(|m| {
    ///                     parts[2].parse::<i32>().map(|s| {
    ///                         ((h * 60 * 60) + (m * 60) + s) as f64
    ///                     })
    ///                 })
    ///             })
    ///             .ok()
    ///         } else {
    ///             None
    ///         }
    ///     }));
    /// # });
    /// ```
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.format = self.format.custom_formatter(formatter);
        self
    }

    /// Set custom parser defining how the text input is parsed into a number.
    ///
    /// A custom parser takes an `&str` to parse into a number and returns `Some` if it was successfully parsed
    /// or `None` otherwise.
    ///
    /// See also: [`Slider::custom_formatter`]
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::Slider::new(&mut my_i32, 0..=((60 * 60 * 24) - 1))
    ///     .custom_formatter(|n, _| {
    ///         let n = n as i32;
    ///         let hours = n / (60 * 60);
    ///         let mins = (n / 60) % 60;
    ///         let secs = n % 60;
    ///         format!("{hours:02}:{mins:02}:{secs:02}")
    ///     })
    ///     .custom_parser(|s| {
    ///         let parts: Vec<&str> = s.split(':').collect();
    ///         if parts.len() == 3 {
    ///             parts[0].parse::<i32>().and_then(|h| {
    ///                 parts[1].parse::<i32>().and_then(|m| {
    ///                     parts[2].parse::<i32>().map(|s| {
    ///                         ((h * 60 * 60) + (m * 60) + s) as f64
    ///                     })
    ///                 })
    ///             })
    ///             .ok()
    ///         } else {
    ///             None
    ///         }
    ///     }));
    /// # });
    /// ```
    #[inline]
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.format = self.format.custom_parser(parser);
        self
    }

    /// Display and parse the number as a binary integer. See [`ValueFormat::binary`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::Slider::new(&mut my_i32, -100..=100).binary(64, false));
    /// # });
    /// ```
    pub fn binary(mut self, min_width: usize, twos_complement: bool) -> Self {
        self.format = self.format.binary(min_width, twos_complement);
        self
    }

    /// Display and parse the number as an octal integer. See [`ValueFormat::octal`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::Slider::new(&mut my_i32, -100..=100).octal(22, false));
    /// # });
    /// ```
    pub fn octal(mut self, min_width: usize, twos_complement: bool) -> Self {
        self.format = self.format.octal(min_width, twos_complement);
        self
    }

    /// Display and parse the number as a hexadecimal integer. See [`ValueFormat::hexadecimal`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::Slider::new(&mut my_i32, -100..=100).hexadecimal(16, false, true));
    /// # });
    /// ```
    pub fn hexadecimal(mut self, min_width: usize, twos_complement: bool, upper: bool) -> Self {
        self.format = self.format.hexadecimal(min_width, twos_complement, upper);
        self
    }

    /// Helper: equivalent to `self.precision(0).smallest_positive(1.0)`.
    /// If you use one of the integer constructors (e.g. `Slider::i32`) this is called for you,
    /// but if you want to have a slider for picking integer values in an `Slider::f64`, use this.
    pub fn integer(self) -> Self {
        self.fixed_decimals(0).smallest_positive(1.0).step_by(1.0)
    }

    fn get_value(&mut self) -> f64 {
        let value = get(&mut self.get_set_value);
        if self.clamping == SliderClamping::Always {
            clamp_value_to_range(value, self.range.clone())
        } else {
            value
        }
    }

    fn set_value(&mut self, mut value: f64) {
        if self.clamping != SliderClamping::Never {
            value = clamp_value_to_range(value, self.range.clone());
        }

        if let Some(step) = self.step {
            let start = *self.range.start();
            value = start + ((value - start) / step).round() * step;
        }
        value = self.format.round(value);
        set(&mut self.get_set_value, value);
    }

    fn range(&self) -> RangeInclusive<f64> {
        self.range.clone()
    }

    /// Where this slider's handle may travel this frame, and what a position there means.
    fn geometry(&self, rect: Rect, ui: &Ui) -> SliderGeometry {
        SliderGeometry {
            orientation: self.orientation,
            handle_shape: self.resolved_handle_shape(ui),
            range: self.range(),
            spec: self.spec.clone(),
            rect,
        }
    }

    fn resolved_handle_shape(&self, ui: &Ui) -> HandleShape {
        self.handle_shape
            .unwrap_or_else(|| ui.style().visuals.handle_shape)
    }

    /// Update the value on each key press when text-editing the value.
    ///
    /// Default: `true`.
    /// If `false`, the value will only be updated when user presses enter or deselects the value.
    #[inline]
    pub fn update_while_editing(mut self, update: bool) -> Self {
        self.format = self.format.update_while_editing(update);
        self
    }
}

impl Slider<'_> {
    /// Just the slider, no text
    fn allocate_slider_space(&self, ui: &mut Ui, thickness: f32) -> Response {
        let desired_size = match self.orientation {
            SliderOrientation::Horizontal => vec2(ui.spacing().slider_width, thickness),
            SliderOrientation::Vertical => vec2(thickness, ui.spacing().slider_width),
        };
        ui.allocate_response(desired_size, Sense::drag())
    }

    /// Just the slider, no text
    fn slider_ui(&mut self, ui: &Ui, response: &Response) {
        let geom = self.geometry(response.rect, ui);

        if let Some(pointer_position_2d) = response.interact_pointer_pos() {
            let new_value =
                slider_core::value_at_pointer(ui, &geom, pointer_position_2d, self.smart_aim);
            self.set_value(new_value);
        }

        let kb_steps = slider_core::keyboard_steps(ui, response, self.orientation);

        if kb_steps != 0.0 {
            let new_value = slider_core::stepped_value(
                &geom,
                self.get_value(),
                kb_steps,
                &StepOptions {
                    step: self.step,
                    smart_aim: self.smart_aim,
                    max_decimals: self.format.max_decimals,
                },
            );
            self.set_value(new_value);
        }

        if let Some(new_value) = slider_core::accesskit_set_value_request(ui, response.id) {
            self.set_value(new_value);
        }

        // Paint it:
        if ui.is_rect_visible(response.rect) {
            let value = self.get_value();

            let rail_rect = slider_core::paint_rail(ui, &geom);
            let center = geom.marker_center(geom.position_from_value(value), &rail_rect);

            // Decide if we should add trailing fill.
            let trailing_fill = self
                .trailing_fill
                .unwrap_or_else(|| ui.visuals().slider_trailing_fill);

            if trailing_fill {
                // The fill runs from the start of the rail to the handle. A range slider hands
                // the same function two handles instead.
                let span = match self.orientation {
                    SliderOrientation::Horizontal => Rangef::new(rail_rect.left(), center.x),
                    SliderOrientation::Vertical => Rangef::new(center.y, rail_rect.bottom()),
                };
                slider_core::paint_fill(ui, rail_rect, span, self.orientation);
            }

            slider_core::paint_handle(ui, &geom, center, ui.style().interact(response));
        }
    }

    fn value_ui(&mut self, ui: &mut Ui, geom: &SliderGeometry) -> Response {
        // If [`DragValue`] is controlled from the keyboard and `step` is defined, set speed to `step`
        let change = ui.input(|input| {
            input.num_presses(Key::ArrowUp) as i32 + input.num_presses(Key::ArrowRight) as i32
                - input.num_presses(Key::ArrowDown) as i32
                - input.num_presses(Key::ArrowLeft) as i32
        });

        let any_change = change != 0;
        let mut value = self.get_value();

        let speed = if let (Some(step), true) = (self.step, any_change) {
            // If [`DragValue`] is controlled from the keyboard and `step` is defined, set speed to `step`
            step
        } else {
            self.drag_value_speed
                .unwrap_or_else(|| geom.gradient_at(value))
        };

        let response = slider_core::slider_drag_value(
            ui,
            &mut value,
            ValueOptions {
                speed,
                range: self.range(),
                clamping: self.clamping,
                format: self.format.clone(),
            },
        );

        if value != self.get_value() {
            self.set_value(value);
        }
        response
    }

    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        let old_value = self.get_value();

        if self.clamping == SliderClamping::Always {
            self.set_value(old_value);
        }

        let thickness = ui
            .text_style_height(&TextStyle::Body)
            .at_least(ui.spacing().interact_size.y);
        let mut response = self.allocate_slider_space(ui, thickness);
        self.slider_ui(ui, &response);

        let value = self.get_value();
        if value != old_value {
            response.mark_changed();
        }
        response.widget_info(|| WidgetInfo::slider(ui.is_enabled(), value, self.text.text()));

        let editable_range = if self.clamping == SliderClamping::Never {
            f64::NEG_INFINITY..=f64::INFINITY
        } else {
            self.range()
        };
        slider_core::declare_accesskit_slider(
            ui,
            response.id,
            value,
            &self.range(),
            &editable_range,
            self.step,
        );

        let slider_response = response.clone();

        let value_response = if self.show_value {
            let geom = self.geometry(response.rect, ui);
            let value_response = self.value_ui(ui, &geom);
            if value_response.gained_focus()
                || value_response.has_focus()
                || value_response.lost_focus()
            {
                // Use the [`DragValue`] id as the id of the whole widget,
                // so that the focus events work as expected.
                response = value_response.union(response);
            } else {
                // Use the slider id as the id for the whole widget
                response = response.union(value_response.clone());
            }
            Some(value_response)
        } else {
            None
        };

        if !self.text.is_empty() {
            let label_response =
                ui.add(Label::new(self.text.clone()).wrap_mode(TextWrapMode::Extend));
            // The slider already has an accessibility label via widget info,
            // but sometimes it's useful for a screen reader to know
            // that a piece of text is a label for another widget,
            // e.g. so the text itself can be excluded from navigation.
            slider_response.labelled_by(label_response.id);
            if let Some(value_response) = value_response {
                value_response.labelled_by(label_response.id);
            }
        }

        response
    }
}

impl Widget for Slider<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let inner_response = match self.orientation {
            SliderOrientation::Horizontal => ui.horizontal(|ui| self.add_contents(ui)),
            SliderOrientation::Vertical => ui.vertical(|ui| self.add_contents(ui)),
        };

        inner_response.inner | inner_response.response
    }
}
