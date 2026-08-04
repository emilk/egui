use std::any::TypeId;

use crate::widget_style::StyleArgs;

/// A Theme plugin that implement a style computation for a defined `WidgetStyle`
pub trait StyleProvider<S> {
    /// The style according to the classes and state of the widget
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> S;

    /// Help to differ the different themes
    fn theme_type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}
