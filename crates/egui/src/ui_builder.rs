use std::{hash::Hash, sync::Arc};

use crate::{Id, Layout, Style, UiStackInfo};

#[derive(Default)]
pub struct UiBuilder {
    pub id_source: Option<Id>,
    pub ui_stack_info: UiStackInfo,
    pub layout: Option<Layout>,
    pub disabled: bool,
    pub invisible: bool,
    pub sizing_pass: bool,
    pub style: Option<Arc<Style>>,
}

impl UiBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the child `Ui` with this `id_source`, which will be mixed
    /// with the [`Ui::id`] of the parent.
    ///
    /// You should give each [`Ui`] an `id_source` that is unique
    /// within the parent, or give it none at all.
    #[inline]
    pub fn id_source(mut self, id_source: impl Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    /// Provide some information about the new `Ui` being built.
    #[inline]
    pub fn ui_stack_info(mut self, ui_stack_info: UiStackInfo) -> Self {
        self.ui_stack_info = ui_stack_info;
        self
    }

    /// Override the layout.
    ///
    /// Will otherwise be inherited from the parent.
    #[inline]
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Make the new `Ui` disabled, i.e. grayed-out and non-interactive.
    ///
    /// Note that if the parent `Ui` is disabled, the child will always be disabled.
    #[inline]
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Make the contents invisible.
    ///
    /// If the parent `Ui` is invisible, the child will always be invisible.
    #[inline]
    pub fn invisible(mut self) -> Self {
        self.invisible = true;
        self
    }

    /// Set to true in special cases where we do one frame
    /// where we size up the contents of the Ui, without actually showing it.
    ///
    /// If the `sizing_pass` flag is set on the parent,
    /// the child will inherit it automatically.
    #[inline]
    pub fn sizing_pass(mut self) -> Self {
        self.sizing_pass = true;
        self
    }

    /// Override the style.
    ///
    /// Otherwise will inherit the style of the parent.
    #[inline]
    pub fn style(mut self, style: impl Into<Arc<Style>>) -> Self {
        self.style = Some(style.into());
        self
    }
}