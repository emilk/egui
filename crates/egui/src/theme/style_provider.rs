use core::any::TypeId;

use crate::widget_style::StyleArgs;

/// A Theme plugin that implement a style computation for a defined `WidgetStyle`
#[cfg_attr(
    not(feature = "experimental"),
    expect(
        unnameable_types,
        reason = "Only nameable with the `experimental` feature"
    )
)]
pub trait StyleProvider<S> {
    /// The style according to the classes and state of the widget
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> S;

    /// Used to tell different themes apart
    fn type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}
