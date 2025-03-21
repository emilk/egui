use std::{hash::Hash, sync::Arc};

use crate::close_tag::ClosableTag;
#[allow(unused_imports)] // Used for doclinks
use crate::Ui;
use crate::{Id, LayerId, Layout, Rect, Sense, Style, UiStackInfo};

/// Build a [`Ui`] as the child of another [`Ui`].
///
/// By default, everything is inherited from the parent,
/// except for `max_rect` which by default is set to
/// the parent [`Ui::available_rect_before_wrap`].
#[must_use]
#[derive(Clone, Default)]
pub struct UiBuilder {
    pub id_salt: Option<Id>,
    pub ui_stack_info: UiStackInfo,
    pub layer_id: Option<LayerId>,
    pub max_rect: Option<Rect>,
    pub layout: Option<Layout>,
    pub disabled: bool,
    pub invisible: bool,
    pub sizing_pass: bool,
    pub style: Option<Arc<Style>>,
    pub sense: Option<Sense>,
}

impl UiBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the child `Ui` with this `id_salt`, which will be mixed
    /// with the [`Ui::id`] of the parent.
    ///
    /// You should give each [`Ui`] an `id_salt` that is unique
    /// within the parent, or give it none at all.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl Hash) -> Self {
        self.id_salt = Some(Id::new(id_salt));
        self
    }

    /// Provide some information about the new `Ui` being built.
    #[inline]
    pub fn ui_stack_info(mut self, ui_stack_info: UiStackInfo) -> Self {
        self.ui_stack_info = ui_stack_info;
        self
    }

    /// Show the [`Ui`] in a different [`LayerId`] from its parent.
    #[inline]
    pub fn layer_id(mut self, layer_id: LayerId) -> Self {
        self.layer_id = Some(layer_id);
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

    /// Set if you want sense clicks and/or drags. Default is [`Sense::hover`].
    ///
    /// The sense will be registered below the Senses of any widgets contained in this [`Ui`], so
    /// if the user clicks a button contained within this [`Ui`], that button will receive the click
    /// instead.
    ///
    /// The response can be read early with [`Ui::response`].
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }

    /// Make this [`Ui`] closable.
    ///
    /// Calling [`Ui::close`] in a child [`Ui`] will mark this [`Ui`] for closing.
    /// After [`Ui::close`] was called, [`Ui::should_close`] and [`crate::Response::should_close`] will
    /// return `true` (for this frame).
    ///
    /// This works by adding a [`ClosableTag`] to the [`UiStackInfo`].
    #[inline]
    pub fn closable(mut self) -> Self {
        self.ui_stack_info
            .tags
            .insert(ClosableTag::NAME, Some(Arc::new(ClosableTag::default())));
        self
    }
}
