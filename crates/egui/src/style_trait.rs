use crate::style::WidgetVisuals;
use crate::{Context, Frame, Id, Painter, Response, TextStyle, Ui};
use emath::{Rect, TSTransform};
use epaint::text::TextFormat;
use epaint::{CornerRadius, Stroke};
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum WidgetName {
    Button,
    Checkbox,
    Slider,
    TextInput,
    Custom(Cow<'static, str>),
}

#[derive(Clone)]
pub struct WidgetContext<'c> {
    pub ui: &'c Ui,
    pub response: &'c Response,
    pub classes: &'c Classes,
    pub name: WidgetName,
}

impl Debug for WidgetContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetContext")
            .field("name", &self.name)
            .field("classes", &self.classes.classes)
            .finish()
    }
}

const CLASS_SELECTED: &str = "selected";

pub trait StyleEngine<T>: Send + Sync {
    fn get(&self, ctx: &WidgetContext) -> T;
}

#[derive(Clone)]
pub struct DefaultWidgetStyle;

impl StyleEngine<WidgetVisuals> for DefaultWidgetStyle {
    fn get(&self, ctx: &WidgetContext) -> WidgetVisuals {
        ctx.ui
            .style()
            .interact_selectable(ctx.response, ctx.classes.has(CLASS_SELECTED))
    }
}

fn widget_style_id() -> Id {
    Id::new("WidgetStyle")
}

impl Ui {
    pub fn widget_style<T: 'static>(
        &self,
        name: WidgetName,
        response: &Response,
        classes: &Classes,
    ) -> T {
        let style = self.data_mut(|d| {
            let style: StyleEngineContainer<T> =
                d.get_temp(widget_style_id()).expect("Widget style not set");
            style
        });
        let ctx = WidgetContext {
            ui: self,
            response: &response,
            classes,
            name,
        };

        style.0.get(&ctx)
    }
}

impl Context {
    pub fn set_style_engine<T: 'static>(&self, style: impl StyleEngine<T> + 'static) {
        self.data_mut(|d| {
            d.insert_temp(widget_style_id(), StyleEngineContainer(Arc::new(style)));
        });
    }
}

struct StyleEngineContainer<T>(Arc<dyn StyleEngine<T>>);

impl<T> Clone for StyleEngineContainer<T> {
    fn clone(&self) -> Self {
        StyleEngineContainer(self.0.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Classes {
    pub classes: smallvec::SmallVec<[&'static str; 1]>,
}

impl Classes {
    pub fn with_if(mut self, class: &'static str, condition: bool) -> Self {
        if condition {
            self.classes.push(class);
        }
        self
    }

    pub fn add_if(&mut self, class: &'static str, condition: bool) {
        if condition {
            self.classes.push(class);
        }
    }

    pub fn has(&self, class: &str) -> bool {
        self.classes.contains(&class)
    }
}

pub trait HasClasses {
    fn classes(&self) -> &Classes;

    fn classes_mut(&mut self) -> &mut Classes;

    fn add_class(&mut self, class: &'static str) -> &Self {
        self.classes_mut().add_if(class, true);
        self
    }

    fn with_class(mut self, class: &'static str) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class, true);
        self
    }

    fn with_class_if(mut self, class: &'static str, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class, condition);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct WidgetStyle {
    /// Background color, stroke, margin, and shadow.
    pub frame: Frame,

    /// What font to use and at what size.
    pub text: TextFormat,

    /// Color and width of e.g. checkbox checkmark.
    /// Also text color.
    ///
    /// Note that this is different from the frame border color.
    pub stroke: Stroke,

    pub transform: TSTransform,
}

/// TODO: Maybe each widget would have its own style struct? Suggested by juancampa
/// Pros:
/// - More and expressive flexible per widget (avoids things like the confusing weak_bg_fill we currently have)
/// - Improved performance, e.g. a checkbox doesn't need a Frame.
///
/// Cons:
/// - Style changes across all widgets would require more code changes.
///   - Maybe there could be a shared base WidgetStyle though that defines things like strokes and base colors?
/// - More boilerplate code for each widget.
///
pub struct CheckboxStyle {
    pub text: TextFormat,
    pub checkmark_stroke: Stroke,
    box_fill: epaint::Color32,
    box_rounding: CornerRadius,
    // Could even define closures for custom painting
    custom_checkmark_painter: Option<Box<dyn Fn(&Painter, Rect)>>,
}

pub struct ButtonStyle {}

impl From<WidgetStyle> for WidgetVisuals {
    fn from(value: WidgetStyle) -> Self {
        Self {
            bg_stroke: value.frame.stroke,
            bg_fill: value.frame.fill,
            fg_stroke: value.stroke,
            corner_radius: value.frame.corner_radius,
            weak_bg_fill: value.frame.fill,
            expansion: value.frame.inner_margin.sum().length(),
        }
    }
}
