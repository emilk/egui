//! Frame container

use crate::{
    InnerResponse, Response, Sense, Style, Ui, UiBuilder, UiKind, UiStackInfo, epaint,
    layers::ShapeIdx,
};
use epaint::{Color32, CornerRadius, Margin, MarginF32, Rect, Shadow, Shape, Stroke};

/// A frame around some content, including margin, colors, etc.
///
/// ## Definitions
/// The total (outer) size of a frame is
/// `content_size + inner_margin + 2 * stroke.width + outer_margin`.
///
/// Everything within the stroke is filled with the fill color (if any).
///
/// ```text
/// +-----------------^-------------------------------------- -+
/// |                 | outer_margin                           |
/// |    +------------v----^------------------------------+    |
/// |    |                 | stroke width                 |    |
/// |    |    +------------v---^---------------------+    |    |
/// |    |    |                | inner_margin        |    |    |
/// |    |    |    +-----------v----------------+    |    |    |
/// |    |    |    |             ^              |    |    |    |
/// |    |    |    |             |              |    |    |    |
/// |    |    |    |<------ content_size ------>|    |    |    |
/// |    |    |    |             |              |    |    |    |
/// |    |    |    |             v              |    |    |    |
/// |    |    |    +------- content_rect -------+    |    |    |
/// |    |    |                                      |    |    |
/// |    |    +-------------fill_rect ---------------+    |    |
/// |    |                                                |    |
/// |    +----------------- widget_rect ------------------+    |
/// |                                                          |
/// +---------------------- outer_rect ------------------------+
/// ```
///
/// The four rectangles, from inside to outside, are:
/// * `content_rect`: the rectangle that is made available to the inner [`Ui`] or widget.
/// * `fill_rect`: the rectangle that is filled with the fill color (inside the stroke, if any).
/// * `widget_rect`: is the interactive part of the widget (what sense clicks etc).
/// * `outer_rect`: what is allocated in the outer [`Ui`], and is what is returned by [`Response::rect`].
///
/// ## Usage
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Frame::NONE
///     .fill(egui::Color32::RED)
///     .show(ui, |ui| {
///         ui.label("Label with red background");
///     });
/// # });
/// ```
///
/// ## Dynamic color
/// If you want to change the color of the frame based on the response of
/// the widget, you need to break it up into multiple steps:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let mut frame = egui::Frame::default().inner_margin(4.0).begin(ui);
/// {
///     let response = frame.content_ui.label("Inside the frame");
///     if response.hovered() {
///         frame.frame.fill = egui::Color32::RED;
///     }
/// }
/// frame.end(ui); // Will "close" the frame.
/// # });
/// ```
///
/// You can also respond to the hovering of the frame itself:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let mut frame = egui::Frame::default().inner_margin(4.0).begin(ui);
/// {
///     frame.content_ui.label("Inside the frame");
///     frame.content_ui.label("This too");
/// }
/// let response = frame.allocate_space(ui);
/// if response.hovered() {
///     frame.frame.fill = egui::Color32::RED;
/// }
/// frame.paint(ui);
/// # });
/// ```
///
/// Note that you cannot change the margins after calling `begin`.
#[doc(alias = "border")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[must_use = "You should call .show()"]
pub struct Frame {
    // Fields are ordered inside-out.
    // TODO(emilk): add `min_content_size: Vec2`
    //
    /// Margin within the painted frame.
    ///
    /// Known as `padding` in CSS.
    #[doc(alias = "padding")]
    pub inner_margin: Margin,

    /// The background fill color of the frame, within the [`Self::stroke`].
    ///
    /// Known as `background` in CSS.
    #[doc(alias = "background")]
    pub fill: Color32,

    /// The width and color of the outline around the frame.
    ///
    /// The width of the stroke is part of the total margin/padding of the frame.
    #[doc(alias = "border")]
    pub stroke: Stroke,

    /// The rounding of the _outer_ corner of the [`Self::stroke`]
    /// (or, if there is no stroke, the outer corner of [`Self::fill`]).
    ///
    /// In other words, this is the corner radius of the _widget rect_.
    pub corner_radius: CornerRadius,

    /// Margin outside the painted frame.
    ///
    /// Similar to what is called `margin` in CSS.
    /// However, egui does NOT do "Margin Collapse" like in CSS,
    /// i.e. when placing two frames next to each other,
    /// the distance between their borders is the SUM
    /// of their other margins.
    /// In CSS the distance would be the MAX of their outer margins.
    /// Supporting margin collapse is difficult, and would
    /// requires complicating the already complicated egui layout code.
    ///
    /// Consider using [`crate::Spacing::item_spacing`]
    /// for adding space between widgets.
    pub outer_margin: Margin,

    /// Optional drop-shadow behind the frame.
    pub shadow: Shadow,
}

#[test]
fn frame_size() {
    assert_eq!(
        std::mem::size_of::<Frame>(),
        32,
        "Frame changed size! If it shrank - good! Update this test. If it grew - bad! Try to find a way to avoid it."
    );
    assert!(
        std::mem::size_of::<Frame>() <= 64,
        "Frame is getting way too big!"
    );
}

/// ## Constructors
impl Frame {
    /// No colors, no margins, no border.
    ///
    /// This is also the default.
    pub const NONE: Self = Self {
        inner_margin: Margin::ZERO,
        stroke: Stroke::NONE,
        fill: Color32::TRANSPARENT,
        corner_radius: CornerRadius::ZERO,
        outer_margin: Margin::ZERO,
        shadow: Shadow::NONE,
    };

    /// No colors, no margins, no border.
    ///
    /// Same as [`Frame::NONE`].
    pub const fn new() -> Self {
        Self::NONE
    }

    #[deprecated = "Use `Frame::NONE` or `Frame::new()` instead."]
    pub const fn none() -> Self {
        Self::NONE
    }

    /// For when you want to group a few widgets together within a frame.
    pub fn group(style: &Style) -> Self {
        Self::new()
            .inner_margin(6)
            .corner_radius(style.visuals.widgets.noninteractive.corner_radius)
            .stroke(style.visuals.widgets.noninteractive.bg_stroke)
    }

    pub fn side_top_panel(style: &Style) -> Self {
        Self::new()
            .inner_margin(Margin::symmetric(8, 2))
            .fill(style.visuals.panel_fill)
    }

    pub fn central_panel(style: &Style) -> Self {
        Self::new().inner_margin(8).fill(style.visuals.panel_fill)
    }

    pub fn window(style: &Style) -> Self {
        Self::new()
            .inner_margin(style.spacing.window_margin)
            .corner_radius(style.visuals.window_corner_radius)
            .shadow(style.visuals.window_shadow)
            .fill(style.visuals.window_fill())
            .stroke(style.visuals.window_stroke())
    }

    pub fn menu(style: &Style) -> Self {
        Self::new()
            .inner_margin(style.spacing.menu_margin)
            .corner_radius(style.visuals.menu_corner_radius)
            .shadow(style.visuals.popup_shadow)
            .fill(style.visuals.window_fill())
            .stroke(style.visuals.window_stroke())
    }

    pub fn popup(style: &Style) -> Self {
        Self::new()
            .inner_margin(style.spacing.menu_margin)
            .corner_radius(style.visuals.menu_corner_radius)
            .shadow(style.visuals.popup_shadow)
            .fill(style.visuals.window_fill())
            .stroke(style.visuals.window_stroke())
    }

    /// A canvas to draw on.
    ///
    /// In bright mode this will be very bright,
    /// and in dark mode this will be very dark.
    pub fn canvas(style: &Style) -> Self {
        Self::new()
            .inner_margin(2)
            .corner_radius(style.visuals.widgets.noninteractive.corner_radius)
            .fill(style.visuals.extreme_bg_color)
            .stroke(style.visuals.window_stroke())
    }

    /// A dark canvas to draw on.
    pub fn dark_canvas(style: &Style) -> Self {
        Self::canvas(style).fill(Color32::from_black_alpha(250))
    }
}

/// ## Builders
impl Frame {
    /// Margin within the painted frame.
    ///
    /// Known as `padding` in CSS.
    #[doc(alias = "padding")]
    #[inline]
    pub fn inner_margin(mut self, inner_margin: impl Into<Margin>) -> Self {
        self.inner_margin = inner_margin.into();
        self
    }

    /// The background fill color of the frame, within the [`Self::stroke`].
    ///
    /// Known as `background` in CSS.
    #[doc(alias = "background")]
    #[inline]
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }

    /// The width and color of the outline around the frame.
    ///
    /// The width of the stroke is part of the total margin/padding of the frame.
    #[inline]
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// The rounding of the _outer_ corner of the [`Self::stroke`]
    /// (or, if there is no stroke, the outer corner of [`Self::fill`]).
    ///
    /// In other words, this is the corner radius of the _widget rect_.
    #[inline]
    pub fn corner_radius(mut self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = corner_radius.into();
        self
    }

    /// The rounding of the _outer_ corner of the [`Self::stroke`]
    /// (or, if there is no stroke, the outer corner of [`Self::fill`]).
    ///
    /// In other words, this is the corner radius of the _widget rect_.
    #[inline]
    #[deprecated = "Renamed to `corner_radius`"]
    pub fn rounding(self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius(corner_radius)
    }

    /// Margin outside the painted frame.
    ///
    /// Similar to what is called `margin` in CSS.
    /// However, egui does NOT do "Margin Collapse" like in CSS,
    /// i.e. when placing two frames next to each other,
    /// the distance between their borders is the SUM
    /// of their other margins.
    /// In CSS the distance would be the MAX of their outer margins.
    /// Supporting margin collapse is difficult, and would
    /// requires complicating the already complicated egui layout code.
    ///
    /// Consider using [`crate::Spacing::item_spacing`]
    /// for adding space between widgets.
    #[inline]
    pub fn outer_margin(mut self, outer_margin: impl Into<Margin>) -> Self {
        self.outer_margin = outer_margin.into();
        self
    }

    /// Optional drop-shadow behind the frame.
    #[inline]
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = shadow;
        self
    }

    /// Opacity multiplier in gamma space.
    ///
    /// For instance, multiplying with `0.5`
    /// will make the frame half transparent.
    #[inline]
    pub fn multiply_with_opacity(mut self, opacity: f32) -> Self {
        self.fill = self.fill.gamma_multiply(opacity);
        self.stroke.color = self.stroke.color.gamma_multiply(opacity);
        self.shadow.color = self.shadow.color.gamma_multiply(opacity);
        self
    }
}

/// ## Inspectors
impl Frame {
    /// How much extra space the frame uses up compared to the content.
    ///
    /// [`Self::inner_margin`] + [`Self::stroke`]`.width` + [`Self::outer_margin`].
    #[inline]
    pub fn total_margin(&self) -> MarginF32 {
        MarginF32::from(self.inner_margin)
            + MarginF32::from(self.stroke.width)
            + MarginF32::from(self.outer_margin)
    }

    /// Calculate the `fill_rect` from the `content_rect`.
    ///
    /// This is the rectangle that is filled with the fill color (inside the stroke, if any).
    pub fn fill_rect(&self, content_rect: Rect) -> Rect {
        content_rect + self.inner_margin
    }

    /// Calculate the `widget_rect` from the `content_rect`.
    ///
    /// This is the visible and interactive rectangle.
    pub fn widget_rect(&self, content_rect: Rect) -> Rect {
        content_rect + self.inner_margin + MarginF32::from(self.stroke.width)
    }

    /// Calculate the `outer_rect` from the `content_rect`.
    ///
    /// This is what is allocated in the outer [`Ui`], and is what is returned by [`Response::rect`].
    pub fn outer_rect(&self, content_rect: Rect) -> Rect {
        content_rect + self.inner_margin + MarginF32::from(self.stroke.width) + self.outer_margin
    }
}

// ----------------------------------------------------------------------------

pub struct Prepared {
    /// The frame that was prepared.
    ///
    /// The margin has already been read and used,
    /// but the rest of the fields may be modified.
    pub frame: Frame,

    /// This is where we will insert the frame shape so it ends up behind the content.
    where_to_put_background: ShapeIdx,

    /// Add your widgets to this UI so it ends up within the frame.
    pub content_ui: Ui,
}

impl Frame {
    /// Begin a dynamically colored frame.
    ///
    /// This is a more advanced API.
    /// Usually you want to use [`Self::show`] instead.
    ///
    /// See docs for [`Frame`] for an example.
    pub fn begin(self, ui: &mut Ui) -> Prepared {
        let where_to_put_background = ui.painter().add(Shape::Noop);
        let outer_rect_bounds = ui.available_rect_before_wrap();

        let mut max_content_rect = outer_rect_bounds - self.total_margin();

        // Make sure we don't shrink to the negative:
        max_content_rect.max.x = max_content_rect.max.x.max(max_content_rect.min.x);
        max_content_rect.max.y = max_content_rect.max.y.max(max_content_rect.min.y);

        let content_ui = ui.new_child(
            UiBuilder::new()
                .ui_stack_info(UiStackInfo::new(UiKind::Frame).with_frame(self))
                .max_rect(max_content_rect),
        );

        Prepared {
            frame: self,
            where_to_put_background,
            content_ui,
        }
    }

    /// Show the given ui surrounded by this frame.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.show_dyn(ui, Box::new(add_contents))
    }

    /// Show using dynamic dispatch.
    pub fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let mut prepared = self.begin(ui);
        let ret = add_contents(&mut prepared.content_ui);
        let response = prepared.end(ui);
        InnerResponse::new(ret, response)
    }

    /// Paint this frame as a shape.
    pub fn paint(&self, content_rect: Rect) -> Shape {
        let Self {
            inner_margin: _,
            fill,
            stroke,
            corner_radius,
            outer_margin: _,
            shadow,
        } = *self;

        let widget_rect = self.widget_rect(content_rect);

        let frame_shape = Shape::Rect(epaint::RectShape::new(
            widget_rect,
            corner_radius,
            fill,
            stroke,
            epaint::StrokeKind::Inside,
        ));

        if shadow == Default::default() {
            frame_shape
        } else {
            let shadow = shadow.as_shape(widget_rect, corner_radius);
            Shape::Vec(vec![Shape::from(shadow), frame_shape])
        }
    }
}

impl Prepared {
    fn outer_rect(&self) -> Rect {
        let content_rect = self.content_ui.min_rect();
        content_rect
            + self.frame.inner_margin
            + MarginF32::from(self.frame.stroke.width)
            + self.frame.outer_margin
    }

    /// Allocate the space that was used by [`Self::content_ui`].
    ///
    /// This MUST be called, or the parent ui will not know how much space this widget used.
    ///
    /// This can be called before or after [`Self::paint`].
    pub fn allocate_space(&self, ui: &mut Ui) -> Response {
        ui.allocate_rect(self.outer_rect(), Sense::hover())
    }

    /// Paint the frame.
    ///
    /// This can be called before or after [`Self::allocate_space`].
    pub fn paint(&self, ui: &Ui) {
        let content_rect = self.content_ui.min_rect();
        let widget_rect = self.frame.widget_rect(content_rect);

        if ui.is_rect_visible(widget_rect) {
            let shape = self.frame.paint(content_rect);
            ui.painter().set(self.where_to_put_background, shape);
        }
    }

    /// Convenience for calling [`Self::allocate_space`] and [`Self::paint`].
    ///
    /// Returns the outer rect, i.e. including the outer margin.
    pub fn end(self, ui: &mut Ui) -> Response {
        self.paint(ui);
        self.allocate_space(ui)
    }
}
