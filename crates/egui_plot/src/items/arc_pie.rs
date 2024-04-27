use super::{Id, PlotBounds, PlotGeometry, PlotItem, PlotPoint, PlotTransform};
use crate::items::{ClosestElem, PlotConfig};
use crate::{
    Align2, Color32, Cursor, Hsva, LabelFormatter, LineStyle, Pos2, Shape, Stroke, TextStyle, Ui,
};
use std::ops::RangeInclusive;

/// A arc line in a plot.
#[derive(Clone, Debug, PartialEq)]
pub struct ArcLine {
    pub(crate) center: PlotPoint,
    pub(crate) radius: f64,
    pub(crate) start_angle: f32,
    pub(crate) end_angle: f32,
    pub(crate) name: String,
    pub(crate) highlight: bool,
    pub(crate) allow_hover: bool,
    pub(crate) stroke: Stroke,
    pub(crate) style: LineStyle,
    id: Option<Id>,
}

impl ArcLine {
    /// Create a new arc line.
    ///
    /// # Arguments
    ///
    /// * `center` - The center of the arc line.
    /// * `radius` - The radius of the arc line in plot coordinates.
    /// * `start_angle` - The start angle of the arc line in radians.
    /// * `end_angle` - The end angle of the arc line in radians.
    pub fn new(
        center: impl Into<PlotPoint>,
        radius: f64,
        start_angle: f32,
        end_angle: f32,
    ) -> Self {
        Self {
            center: center.into(),
            radius,
            start_angle,
            end_angle,
            name: Default::default(),
            highlight: false,
            allow_hover: true,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            style: LineStyle::Solid,
            id: None,
        }
    }

    /// Set the center of the arc line.
    #[inline]
    pub fn center(mut self, center: PlotPoint) -> Self {
        self.center = center;
        self
    }

    /// Set the radius of the arc line.
    #[inline]
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    /// Set the start angle of the arc line.
    #[inline]
    pub fn start_angle(mut self, start_angle: f32) -> Self {
        self.start_angle = start_angle;
        self
    }

    /// Set the end angle of the arc line.
    #[inline]
    pub fn end_angle(mut self, end_angle: f32) -> Self {
        self.end_angle = end_angle;
        self
    }

    /// Set the name of the arc line.
    #[inline]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the highlight state of the arc line.
    #[inline]
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Set the hover state of the arc line.
    #[inline]
    pub fn allow_hover(mut self, allow_hover: bool) -> Self {
        self.allow_hover = allow_hover;
        self
    }

    /// Set the stroke of the arc line.
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    /// Set the color of the arc line.
    ///
    /// This will override the color set in the stroke.
    #[inline]
    pub fn color(mut self, color: Color32) -> Self {
        self.stroke.color = color;
        self
    }

    /// Set the style of the arc line.
    #[inline]
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the id of the arc line.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl PlotItem for ArcLine {
    fn shapes(&self, _ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let center = transform.position_from_point(&self.center);
        let max_x_pos = transform
            .position_from_point(&PlotPoint::new(self.center.x + self.radius, self.center.y));
        let radius = max_x_pos.x - center.x;
        let start_angle = self.start_angle;
        let end_angle = self.end_angle;
        let mut stroke = self.stroke;

        // Expand the radius with stroke width if the item is highlighted
        if self.highlight {
            stroke.width *= 2.0;
        }

        shapes.push(Shape::arc(center, radius, start_angle, end_angle, stroke));
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn allow_hover(&self) -> bool {
        self.allow_hover
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        calculate_arc_bounds(self.center, self.radius, self.start_angle, self.end_angle)
    }

    fn id(&self) -> Option<Id> {
        self.id
    }
}

/// A pie in a plot.
#[derive(Clone, Debug, PartialEq)]
pub struct Pie {
    pub(crate) center: PlotPoint,
    pub(crate) radius: f64,
    pub(crate) start_angle: f32,
    pub(crate) end_angle: f32,
    pub(crate) name: String,
    pub(crate) highlight: bool,
    pub(crate) allow_hover: bool,
    pub(crate) fill: Color32,
    pub(crate) stroke: Stroke,
    pub(crate) style: LineStyle,
    shrink_or_expand: Option<f32>,
    id: Option<Id>,
}

impl Pie {
    /// Create a new pie.
    ///
    /// # Arguments
    ///
    /// * `center` - The center of the pie.
    /// * `radius` - The radius of the pie in plot coordinates.
    /// * `start_angle` - The start angle of the pie in radians.
    /// * `end_angle` - The end angle of the pie in radians.
    pub fn new(
        center: impl Into<PlotPoint>,
        radius: f64,
        start_angle: f32,
        end_angle: f32,
    ) -> Self {
        Self {
            center: center.into(),
            radius,
            start_angle,
            end_angle,
            name: Default::default(),
            highlight: false,
            allow_hover: true,
            fill: Color32::TRANSPARENT,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            style: LineStyle::Solid,
            shrink_or_expand: None,
            id: None,
        }
    }

    /// Set the center of the pie.
    #[inline]
    pub fn center(mut self, center: PlotPoint) -> Self {
        self.center = center;
        self
    }

    /// Set the radius of the pie.
    #[inline]
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    /// Set the start angle of the pie.
    #[inline]
    pub fn start_angle(mut self, start_angle: f32) -> Self {
        self.start_angle = start_angle;
        self
    }

    /// Set the end angle of the pie.
    #[inline]
    pub fn end_angle(mut self, end_angle: f32) -> Self {
        self.end_angle = end_angle;
        self
    }

    /// Set the name of the pie.
    #[inline]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the highlight state of the pie.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Set the hover state of the pie.
    #[inline]
    pub fn allow_hover(mut self, allow_hover: bool) -> Self {
        self.allow_hover = allow_hover;
        self
    }

    /// Set the fill color of the pie.
    #[inline]
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }

    /// Set the stroke of the pie.
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    /// Set the style of the pie.
    #[inline]
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Shrink the pie by a amount of pixels in screen coordinates.
    #[inline]
    pub fn shrink(mut self, amount: f32) -> Self {
        self.shrink_or_expand = Some(-amount);
        self
    }

    /// Expand the pie by a amount of pixels in screen coordinates.
    #[inline]
    pub fn expand(mut self, amount: f32) -> Self {
        self.shrink_or_expand = Some(amount);
        self
    }

    /// Set the id of the pie.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl PlotItem for Pie {
    fn shapes(&self, _ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let mut center = transform.position_from_point(&self.center);
        let max_x_pos = transform
            .position_from_point(&PlotPoint::new(self.center.x + self.radius, self.center.y));
        let mut radius = max_x_pos.x - center.x;
        let mut start_angle = self.start_angle;
        let mut end_angle = self.end_angle;
        let fill = self.fill;
        let stroke = self.stroke;

        // Shrink or expand the pie
        if let Some(mut amount) = self.shrink_or_expand {
            // Adjust the amount to fit within a smaller radius.
            if radius < 64.0 {
                amount *= radius / 64.0;
            }
            let (new_center, new_radius, new_start_angle, new_end_angle) =
                shrink_or_expand_pie(center, radius, start_angle, end_angle, amount);
            center = new_center;
            radius = new_radius;
            start_angle = new_start_angle;
            end_angle = new_end_angle;
        }

        // Expand the radius with stroke width if the item is highlighted
        if self.highlight {
            radius += (stroke.width * 2.0).clamp(2.0, 10.0);
        }

        shapes.push(Shape::pie(
            center,
            radius,
            start_angle,
            end_angle,
            fill,
            stroke,
        ));
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color32 {
        self.fill
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn allow_hover(&self) -> bool {
        self.allow_hover
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        calculate_arc_bounds(self.center, self.radius, self.start_angle, self.end_angle)
    }

    fn id(&self) -> Option<Id> {
        self.id
    }

    fn find_closest(&self, point: Pos2, transform: &PlotTransform) -> Option<ClosestElem> {
        let center = transform.position_from_point(&self.center);
        let radius = transform.position_from_point_x(self.center.x + self.radius) - center.x;
        // let angle_pairs = self.to_angle_pairs();
        // for (i, (start, end)) in angle_pairs.into_iter().enumerate() {
        if contains_in_pie(center, radius, self.start_angle, self.end_angle, point) {
            return Some(ClosestElem {
                index: 0,
                dist_sq: 0.0,
            });
        }
        // }

        None
    }

    fn on_hover(
        &self,
        _elem: ClosestElem,
        shapes: &mut Vec<Shape>,
        _cursors: &mut Vec<Cursor>,
        plot: &PlotConfig<'_>,
        _label_formatter: &LabelFormatter,
    ) {
        // let text = format!("{}", self.name);
        let font_id = TextStyle::Body.resolve(plot.ui.style());
        plot.ui.fonts(|f| {
            let center = center_of_pie(
                self.center.to_pos2(),
                self.radius as f32,
                self.start_angle,
                self.end_angle,
            );
            let color = auto_color_inverted(self.start_angle as f64, self.end_angle as f64);
            shapes.push(Shape::text(
                f,
                plot.transform
                    .position_from_point(&PlotPoint::new(center.x, center.y)),
                Align2::CENTER_CENTER,
                &self.name,
                font_id,
                color,
            ));
        });
    }
}

pub struct PieChart {
    pub(crate) center: PlotPoint,
    pub(crate) radius: f64,
    pub(crate) name: String,
    pub(crate) highlight: bool,
    pub(crate) allow_hover: bool,
    pub(crate) stroke: Stroke,
    pub(crate) data: Vec<f64>,
    pub(crate) colors: Vec<Color32>,
    pub(crate) labels: Vec<String>,
    pub(crate) style: LineStyle,
    pub(crate) shrink_or_expand: Option<f32>,
    id: Option<Id>,
}

impl PieChart {
    /// Create a new pie chart.
    ///
    /// # Arguments
    ///
    /// * `center` - The center of the pie chart.
    /// * `radius` - The radius of the pie chart in plot coordinates.
    /// * `data` - The data of the pie chart.
    pub fn new(center: impl Into<PlotPoint>, radius: f64, data: Vec<f64>) -> Self {
        Self {
            center: center.into(),
            radius,
            name: Default::default(),
            highlight: false,
            allow_hover: true,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            data,
            colors: vec![],
            labels: vec![],
            style: LineStyle::Solid,
            shrink_or_expand: None,
            id: None,
        }
    }

    /// Set the center of the pie chart.
    #[inline]
    pub fn center(mut self, center: PlotPoint) -> Self {
        self.center = center;
        self
    }

    /// Set the radius of the pie chart.
    #[inline]
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    /// Set the name of the pie chart.
    #[inline]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the highlight state of the pie chart.
    #[inline]
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Set the hover state of the pie chart.
    #[inline]
    pub fn allow_hover(mut self, allow_hover: bool) -> Self {
        self.allow_hover = allow_hover;
        self
    }

    /// Set the stroke of the pie chart.
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    /// Set the data of the pie chart.
    #[inline]
    pub fn data(mut self, data: Vec<f64>) -> Self {
        self.data = data;
        self
    }

    /// Set the fill colors of the pie chart.
    #[inline]
    pub fn colors(mut self, colors: Vec<Color32>) -> Self {
        self.colors = colors;
        self
    }

    #[inline]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Set the labels of the pie chart.
    #[inline]
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set the style of the pie chart
    #[inline]
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Shrink the pie chart by a amount of pixels in screen coordinates.
    #[inline]
    pub fn shrink(mut self, amount: f32) -> Self {
        self.shrink_or_expand = Some(-amount);
        self
    }

    /// Expand the pie chart by a amount of pixels in screen coordinates.
    #[inline]
    pub fn expand(mut self, amount: f32) -> Self {
        self.shrink_or_expand = Some(amount);
        self
    }

    /// Set the id of the pie chart.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Convert the pie chart to a list of pies.
    pub fn to_pies(&self) -> Vec<Pie> {
        use std::f64::consts::TAU;

        let sum: f64 = self.data.iter().sum();
        let mut start_angle = 0.0;
        let mut pies = vec![];
        for (i, v) in self.data.iter().enumerate() {
            let end_angle = start_angle + (v / sum) * TAU;
            let default_color = auto_color(start_angle, end_angle);
            let name = self.labels.get(i).map_or(Default::default(), |s| s.clone());
            let fill = self.colors.get(i).map_or(default_color, |v| *v);
            let pie = Pie::new(
                self.center,
                self.radius,
                start_angle as f32,
                end_angle as f32,
            )
            .name(name)
            .fill(fill)
            .stroke(self.stroke)
            .style(self.style);
            pies.push(pie);
            start_angle = end_angle;
        }
        pies
    }

    pub fn to_angle_pairs(&self) -> Vec<(f32, f32)> {
        use std::f64::consts::TAU;

        let sum: f64 = self.data.iter().sum();
        let mut start_angle = 0.0;
        let mut pies = vec![];
        for v in &self.data {
            let end_angle = start_angle + (v / sum) * TAU;
            pies.push((start_angle as f32, end_angle as f32));
            start_angle = end_angle;
        }
        pies
    }
}

impl PlotItem for PieChart {
    fn shapes(&self, ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let pies = self.to_pies();
        for pie in pies {
            pie.highlight(self.highlight).shapes(ui, transform, shapes);
        }
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color32 {
        Color32::TRANSPARENT
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn allow_hover(&self) -> bool {
        self.allow_hover
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        let min_point = [self.center.x - self.radius, self.center.y - self.radius];
        let max_point = [self.center.x + self.radius, self.center.y + self.radius];
        PlotBounds::from_min_max(min_point, max_point)
    }

    fn id(&self) -> Option<Id> {
        self.id
    }

    fn find_closest(&self, point: Pos2, transform: &PlotTransform) -> Option<ClosestElem> {
        let center = transform.position_from_point(&self.center);
        let radius = transform.position_from_point_x(self.center.x + self.radius) - center.x;
        let angle_pairs = self.to_angle_pairs();
        for (i, (start, end)) in angle_pairs.into_iter().enumerate() {
            if contains_in_pie(center, radius, start, end, point) {
                return Some(ClosestElem {
                    index: i,
                    dist_sq: 0.0,
                });
            }
        }

        None
    }

    fn on_hover(
        &self,
        elem: ClosestElem,
        shapes: &mut Vec<Shape>,
        _cursors: &mut Vec<Cursor>,
        plot: &PlotConfig<'_>,
        _label_formatter: &LabelFormatter,
    ) {
        let angles = self.to_angle_pairs();
        let value = self.data[elem.index];
        let text = if let Some(label) = self.labels.get(elem.index) {
            format!("{value}\n{label}")
        } else {
            format!("{value}")
        };
        let font_id = TextStyle::Body.resolve(plot.ui.style());
        plot.ui.fonts(|f| {
            let (start_angle, end_angle) = angles[elem.index];
            let center = center_of_pie(
                self.center.to_pos2(),
                self.radius as f32,
                start_angle,
                end_angle,
            );
            let color = auto_color_inverted(start_angle as f64, end_angle as f64);
            shapes.push(Shape::text(
                f,
                plot.transform
                    .position_from_point(&PlotPoint::new(center.x, center.y)),
                Align2::CENTER_CENTER,
                text,
                font_id,
                color,
            ));
        });
    }
}

/// Calculate the fill color of the pie chart.
#[inline]
fn auto_color(start_angle: f64, end_angle: f64) -> Color32 {
    let mid_angle = (start_angle + end_angle) / 2.0;
    let h = mid_angle.abs() / std::f64::consts::TAU;
    Hsva::new(h as f32, 0.95, 0.85, 0.95).into()
}

/// Calculate the inverted fill color of the pie chart.
#[inline]
fn auto_color_inverted(start_angle: f64, end_angle: f64) -> Color32 {
    let mid_angle = (start_angle + end_angle) / 2.0;
    let h = (mid_angle.abs() / std::f64::consts::TAU + 0.5) % 1.0;
    Hsva::new(h as f32, 0.95, 0.85, 0.95).into()
}
/// Calculate the bounds of a arc.
fn calculate_arc_bounds(
    center: PlotPoint,
    radius: f64,
    start_angle: f32,
    end_angle: f32,
) -> PlotBounds {
    let x = center.x;
    let y = center.y;
    let r = radius;
    let start_angle = start_angle as f64;
    let end_angle = end_angle as f64;
    let min_point = [x - r, y - r];
    let max_point = [x + r, y + r];
    let start_point = [x + r * start_angle.cos(), y + r * start_angle.sin()];
    let end_point = [x + r * end_angle.cos(), y + r * end_angle.sin()];
    let min_x = min_point[0].min(start_point[0]).min(end_point[0]);
    let max_x = max_point[0].max(start_point[0]).max(end_point[0]);
    let min_y = min_point[1].min(start_point[1]).min(end_point[1]);
    let max_y = max_point[1].max(start_point[1]).max(end_point[1]);
    PlotBounds::from_min_max([min_x, min_y], [max_x, max_y])
}

/// Shrink or expand a pie by a amount of pixels in screen coordinates.
fn shrink_or_expand_pie(
    center: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    amount: f32,
) -> (Pos2, f32, f32, f32) {
    let move_distance = amount * 2.0;
    let new_radius = radius + move_distance;

    // Calculate the direction of the midline
    let mid_angle = (start_angle + end_angle) / 2.0;
    let direction = Pos2 {
        x: mid_angle.cos(),
        y: mid_angle.sin(),
    };

    // Move the center along the midline
    let center = Pos2 {
        x: center.x - direction.x * move_distance,
        y: center.y - direction.y * move_distance,
    };

    (center, new_radius, start_angle, end_angle)
}

/// Check if a point is within a pie.
fn contains_in_pie(
    center: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    point: Pos2,
) -> bool {
    use std::f32::consts::PI;

    // Calculate the distance between the point and the center
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let distance = dx.hypot(dy);

    // Check if the point is within the radius
    if distance > radius {
        return false;
    }

    // Calculate the angle of the point
    let angle = dy.atan2(dx);
    let angle = if angle < 0.0 { angle + 2.0 * PI } else { angle };

    // Check if the angle is within the start and end angles
    start_angle <= angle && angle <= end_angle
}

fn center_of_pie(center: Pos2, radius: f32, start_angle: f32, end_angle: f32) -> Pos2 {
    let mid_angle = (start_angle + end_angle) / 2.0;
    let direction = Pos2 {
        x: mid_angle.cos(),
        y: mid_angle.sin(),
    };
    Pos2 {
        x: center.x + direction.x * radius / 2.0,
        y: center.y - direction.y * radius / 2.0,
    }
}
