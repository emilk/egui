#![expect(deprecated, clippy::new_ret_no_self)]

use crate::WidgetText;

#[deprecated = "Use `Button::selectable()` instead"]
pub struct SelectableLabel {}

impl SelectableLabel {
    #[deprecated = "Use `Button::selectable()` instead"]
    pub fn new(selected: bool, text: impl Into<WidgetText>) -> super::Button<'static> {
        crate::Button::selectable(selected, text)
    }
}
