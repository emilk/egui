use crate::{
    Atom, AtomExt, AtomKind, AtomLayout, Atoms, Button, Color32, Context, Id, InnerResponse,
    IntoAtoms, Layout, Response, Sense, Spacing, Style, Ui, UiBuilder, Visuals, Widget, WidgetRect,
};
use emath::{Align, Pos2, Rect, Vec2};
use epaint::Direction;

pub fn atom() -> Atom<'static> {
    Atom::default()
}

pub trait AtomWidget<'a> {
    fn atom_ui(self, ui: &mut AtomWidgetContext, response: &mut Response) -> AtomLayout<'a>;

    fn show_for(self, ui: &mut AtomWidgetContext) -> (AtomLayout<'a>, Response)
    where
        Self: Sized,
    {
        let id = ui.make_auto_id();
        let mut response = ui.read_response(id);

        let mut layout = self.atom_ui(ui, &mut response);
        layout = layout.id(id);

        (layout, response)
    }
}

impl<'a> AtomWidget<'a> for AtomLayout<'a> {
    fn atom_ui(self, ui: &mut AtomWidgetContext, response: &mut Response) -> AtomLayout<'a> {
        self
    }
}

impl<'a> AtomWidget<'a> for Atom<'a> {
    fn atom_ui(self, ui: &mut AtomWidgetContext, response: &mut Response) -> AtomLayout<'a> {
        AtomLayout::new(self)
    }
}
impl<'a> AtomWidget<'a> for AtomKind<'a> {
    fn atom_ui(self, ui: &mut AtomWidgetContext, response: &mut Response) -> AtomLayout<'a> {
        AtomLayout::new(self)
    }
}
impl<'a> AtomWidget<'a> for Atoms<'a> {
    fn atom_ui(self, ui: &mut AtomWidgetContext, response: &mut Response) -> AtomLayout<'a> {
        AtomLayout::new(self)
    }
}

#[macro_export]
macro_rules! impl_widget_for_atom_widget {
    ($widget:ty) => {
        impl $crate::Widget for $widget {
            fn ui(self, ui: &mut $crate::Ui) -> $crate::Response {
                let layout = self.show_for(ui).0;
                ui.add(layout)
            }
        }
    };
}

pub trait IsAtomWidgetContext {
    fn ctx(&self) -> &crate::Context;
    fn make_auto_id(&mut self) -> Id;

    fn is_enabled(&self) -> bool;

    fn style(&self) -> &Style;
    fn style_mut(&mut self) -> &mut Style;

    fn spacing(&self) -> &Spacing {
        &self.style().spacing
    }
    fn spacing_mut(&mut self) -> &mut Spacing {
        &mut self.style_mut().spacing
    }

    fn visuals(&self) -> &Visuals {
        &self.style().visuals
    }
    fn visuals_mut(&mut self) -> &mut Visuals {
        &mut self.style_mut().visuals
    }

    fn read_response(&self, id: Id) -> Response;

    fn child_ui(&mut self, builder: UiBuilder) -> Ui;
}

pub type AtomWidgetContext = dyn IsAtomWidgetContext;

impl IsAtomWidgetContext for Ui {
    fn ctx(&self) -> &Context {
        self.ctx()
    }

    fn make_auto_id(&mut self) -> Id {
        let id = self.next_auto_id();
        self.skip_ahead_auto_ids(1);
        id
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled()
    }

    fn style(&self) -> &Style {
        self.style()
    }

    fn style_mut(&mut self) -> &mut Style {
        self.style_mut()
    }

    fn read_response(&self, id: Id) -> Response {
        read_or_default_response(&self, id, Sense::hover())
    }

    fn child_ui(&mut self, builder: UiBuilder) -> Ui {
        Ui::new_child(self, builder)
    }
}

pub struct AtomUi<'ui, 'layout> {
    ctx: &'ui mut AtomWidgetContext,
    layout: AtomLayout<'layout>,
}

impl<'ui, 'layout> AtomUi<'ui, 'layout> {
    pub fn new(ctx: &'ui mut AtomWidgetContext, builder: AtomLayout<'layout>) -> Self {
        let layout = builder.id(ctx.make_auto_id());
        Self { ctx, layout }
    }

    pub fn response(&self) -> Response {
        self.ctx
            .read_response(self.layout.id.expect("set in constructor"))
    }

    pub fn add(&mut self, mut config: Atom<'layout>, widget: impl AtomWidget<'layout>) -> Response {
        let (layout, response) = widget.show_for(self.ctx);

        config.kind = AtomKind::Layout(std::rc::Rc::new(layout));
        self.layout.push_right(config);

        response
    }

    pub fn scope_builder<R>(
        &mut self,
        builder: AtomLayout<'layout>,
        mut atom: Atom<'layout>,
        add_content: impl FnOnce(&mut AtomUi) -> R,
    ) -> InnerResponse<R> {
        let mut child = AtomUi::new(self.ctx, builder);
        let inner = add_content(&mut child);
        let response = InnerResponse {
            inner,
            response: child.response(),
        };
        atom.kind = AtomKind::Layout(std::rc::Rc::new(child.layout));
        self.layout.push_right(atom);
        response
    }

    pub fn vertical<R>(
        &mut self,
        atom: Atom<'layout>,
        add_content: impl FnOnce(&mut AtomUi) -> R,
    ) -> InnerResponse<R> {
        self.scope_builder(
            AtomLayout::default().direction(Direction::TopDown),
            atom,
            add_content,
        )
    }

    pub fn show(self) -> (AtomLayout<'layout>, Response) {
        let response = self.response();
        (self.layout, response)
    }

    pub fn immediate_scope<R>(
        &mut self,
        mut ui_builder: UiBuilder,
        mut atom: Atom<'layout>,
        add_content: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let sizing_id = self.ctx.make_auto_id();
        let mut sizing_response = self.ctx.read_response(sizing_id);

        let mut size = Vec2::ZERO;
        if sizing_response.rect.is_finite() && sizing_response.rect.is_positive() {
            size = sizing_response
                .intrinsic_size()
                .unwrap_or(sizing_response.rect.size());
        }

        let placement_response = self.add(atom.clone(), AtomLayout::new(atom.atom_size(size)));

        if placement_response.rect.is_finite() && placement_response.rect.is_positive() {
            ui_builder = ui_builder.max_rect(Rect::from_min_size(
                placement_response.rect.min,
                Vec2::INFINITY,
            ));
        } else {
            ui_builder = ui_builder
                .max_rect(Rect::from_min_size(Pos2::ZERO, Vec2::INFINITY))
                .invisible();
        }
        ui_builder = ui_builder
            .id(sizing_id)
            .layout(Layout::left_to_right(Align::Min));

        let mut immediate_ui = self.ctx.child_ui(ui_builder);
        let inner = add_content(&mut immediate_ui);

        InnerResponse {
            inner,
            response: sizing_response,
        }
    }

    /// Show a label which can be selected or not.
    ///
    /// See also [`Button::selectable`] and [`Self::toggle_value`].
    #[must_use = "You should check if the user clicked this with `if ui.selectable_label(…).clicked() { … } "]
    pub fn selectable_label(&mut self, checked: bool, text: impl IntoAtoms<'layout>) -> Response {
        self.add(atom().atom_grow(true), Button::selectable(checked, text))
    }

    /// Show selectable text. It is selected if `*current_value == selected_value`.
    /// If clicked, `selected_value` is assigned to `*current_value`.
    ///
    /// Example: `ui.selectable_value(&mut my_enum, Enum::Alternative, "Alternative")`.
    ///
    /// See also [`Button::selectable`] and [`Self::toggle_value`].
    pub fn selectable_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl IntoAtoms<'layout>,
    ) -> Response {
        let mut response = self.selectable_label(*current_value == selected_value, text);
        if response.clicked() && *current_value != selected_value {
            *current_value = selected_value;
            response.mark_changed();
        }
        response
    }
}

impl Ui {
    pub fn atom_builder<T>(
        &mut self,
        builder: AtomLayout,
        add_contents: impl FnOnce(&mut AtomUi) -> T,
    ) -> InnerResponse<T> {
        let mut ui = AtomUi::new(self, builder);
        let inner = add_contents(&mut ui);
        let AtomUi { ctx, layout } = ui;
        InnerResponse {
            inner,
            response: self.add(layout),
        }
    }

    pub fn atom<T>(&mut self, add_contents: impl FnOnce(&mut AtomUi) -> T) -> InnerResponse<T> {
        self.atom_builder(AtomLayout::default(), add_contents)
    }
}

/// Read this widget's [`Response`] from a previous frame for state-based styling, or synthesize a
/// default (inactive) one if it hasn't been registered yet (e.g. the first frame).
///
/// Mirrors the old `read_response(id).map(..).unwrap_or_default()` pattern, but yields a real
/// [`Response`] so it can feed [`Button::into_atom_ui`].
fn read_or_default_response(ui: &Ui, id: Id, sense: Sense) -> Response {
    ui.ctx().read_response(id).unwrap_or_else(|| {
        ui.ctx().get_response(WidgetRect {
            id,
            parent_id: ui.id(),
            layer_id: ui.layer_id(),
            rect: Rect::NOTHING,
            interact_rect: Rect::NOTHING,
            sense,
            enabled: ui.is_enabled(),
        })
    })
}
