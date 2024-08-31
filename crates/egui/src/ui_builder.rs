use std::{hash::Hash, sync::Arc};

use crate::{Id, Layout, Rect, Style, UiStackInfo};

#[allow(unused_imports)] // Used for doclinks
use crate::Ui;

/// Build a [`Ui`] as the chlild of another [`Ui`].
///
/// By default, everything is inherited from the parent,
/// except for `max_rect` which by default is set to
/// the parent [`Ui::available_rect_before_wrap`].
#[must_use]
#[derive(Clone, Default)]
pub struct UiBuilder {
    pub id_source: Option<Id>,
    pub ui_stack_info: UiStackInfo,
    pub max_rect: Option<Rect>,
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

    /// Set the max rectangle, within which widgets will go.
    ///
    /// New widgets will *try* to fit within this rectangle.
    ///
    /// Text labels will wrap to fit within `max_rect`.
    /// Separator lines will span the `max_rect`.
    ///
    /// If a new widget doesn't fit within the `max_rect` then the
    /// [`Ui`] will make room for it by expanding both `min_rect` and
    ///
    /// If not set, this will be set to the parent
    /// [`Ui::available_rect_before_wrap`].
    #[inline]
    pub fn max_rect(mut self, max_rect: Rect) -> Self {
        self.max_rect = Some(max_rect);
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
    /// Will also disable the `Ui` (see [`Self::disabled`]).
    ///
    /// If the parent `Ui` is invisible, the child will always be invisible.
    #[inline]
    pub fn invisible(mut self) -> Self {
        self.invisible = true;
        self.disabled = true;
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
        self.invisible = true;
        self.disabled = true;
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
