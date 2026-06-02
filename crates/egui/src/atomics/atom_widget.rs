use crate::{
    Atom, AtomKind, AtomLayout, Button, Id, InnerResponse, Response, Sense, Ui, WidgetRect,
};
use emath::Rect;

pub fn atom() -> Atom<'static> {
    Atom::default()
}

pub trait AtomWidget<'a> {
    fn atom_ui(self, ui: &mut Ui, response: &mut Response) -> AtomLayout<'a>;

    fn show_for(self, ui: &mut Ui) -> (AtomLayout<'a>, Response)
    where
        Self: Sized,
    {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);

        let mut response = read_or_default_response(ui, id, Sense::hover());
        let mut layout = self.atom_ui(ui, &mut response);
        layout = layout.id(id);

        (layout, response)
    }
}

#[macro_export]
macro_rules! impl_widget_for_atom_widget {
    ($widget:ty) => {
        impl Widget for $widget {
            fn ui(self, ui: &mut Ui) -> Response {
                let layout = self.show_for(ui).0;
                ui.add(layout)
            }
        }
    };
}

pub struct AtomUi<'ui, 'layout> {
    ui: &'ui mut Ui,
    layout: AtomLayout<'layout>,
}

impl<'ui, 'layout> AtomUi<'ui, 'layout> {
    pub fn new(ui: &'ui mut Ui) -> Self {
        let layout = AtomLayout::new(());
        Self { ui, layout }
    }

    pub fn add(&mut self, mut config: Atom<'layout>, widget: impl AtomWidget<'layout>) -> Response {
        let (layout, response) = widget.show_for(self.ui);

        config.kind = AtomKind::Layout(Box::new(layout));

        self.layout.push_right(config);

        response
    }
}

impl Ui {
    pub fn atom_builder<T>(
        &mut self,
        layout: AtomLayout,
        add_contents: impl FnOnce(&mut AtomUi) -> T,
    ) -> InnerResponse<T> {
        let mut ui = AtomUi { ui: self, layout };
        let inner = add_contents(&mut ui);
        let AtomUi { ui, layout } = ui;
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
