use crate::{AtomKind, AtomLayout, Atoms, Button, Context, Frame, Id, Response, Ui, Widget};

// We would rename AtomLayout to AtomElement or similar, and it would be the "container" element
type AtomElement<'a> = AtomLayout<'a>;

trait AtomWidget {
    fn show(self, ui: &mut AtomUi, response: Response) -> AtomElement;
}

impl<'a> AtomWidget for Button<'a> {
    // We could pass in the response via Context::read_response for convenience,
    fn show(self, ui: &mut AtomUi, response: Response) -> AtomElement {
        // self.atom_ui(ui) // this should return the AtomElement instead
    }
}

/// The [Ui] equivalent. Instead of producing a list of shapes to paint, it produces a tree of
/// [AtomElement]s to layout and paint
struct AtomUi<'a> {
    // This would probably just be another AtomElement and there would be a AtomKind::Layout
    content: AtomElement<'a>,
    id: Id,
    next_id: usize,
    context: Context,
}

impl AtomUi {
    pub fn new(context: Context) -> Self {
        AtomUi {
            content: AtomElement::default(),
            id: Id::new(0),
            next_id: 0,
            context,
        }
    }

    pub fn scope(&mut self, layout: AtomElement, content: impl FnOnce(&mut AtomUi)) {}

    pub fn horizontal(&mut self, content: impl FnOnce(&mut AtomUi)) {
        let layout = AtomElement::default();
        //.direction(egui::Direction::LeftToRight); We don't have a Direction yet
        self.scope(layout, content);
    }

    pub fn vertical(&mut self, content: impl FnOnce(&mut AtomUi)) {
        let layout = AtomElement::default();
        //.direction(egui::Direction::TopDown); We don't have a Direction yet
        self.scope(layout, content);
    }

    fn next_id(&mut self) -> Id {
        let id = self.id.with(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn add(&mut self, widget: impl AtomWidget<'_>) {
        let id = self.next_id();
        // It would be really convenient if this could return a "default" response instead of Option
        let response = self.context.read_response(id);

        // TODO: We need a AtomKind::AtomElement or similar
        self.content
            .push_right(widget.show(self, response.unwrap()));
    }
}

impl Widget for AtomUi {
    /// This is where the actual layout and rendering happens.
    /// This could be a advanced version of our current `AtomLayout`
    /// (we could draw inspiration from https://github.com/nicbarker/clay),
    /// or could be based on e.g. `taffy`.
    ///
    /// Since we now have a tree of elements, we can do fancy multipass measuring and layouting.
    /// Everything that `taffy` can do should be possible.
    ///
    /// We'd probably want to do some kind of caching here so we don't have to re-layout parts of the
    /// tree that haven't changed. (Taffy has built in support for that, I believe.)
    fn ui(self, ui: &mut Ui) -> Response {
        self.content.show(ui).response
    }
}

pub fn my_single_pass_ui(ui: &mut Ui) {
    // Ideally there would be something like this:
    // let mut atom_ui = ui.atom_ui(AtomElement::default().vertical());
    // the atom ui should inherit the UiStack, Style, Id, etc...
    let mut atom_ui = AtomUi::new(ui.ctx().clone());

    // Then we can build our element tree
    my_atom_ui(&mut atom_ui);

    // And add it to our classic ui
    ui.add(atom_ui);
}

/// The atom-based uis could feel very similar to the classic uis
pub fn my_atom_ui(ui: &mut AtomUi) {
    ui.add(Button::new("Atom Widget Button!"));

    ui.horizontal(|ui| {
        ui.add(Button::new("Button 2"));
    });

    // If we add a AtomKind::Ui(&dyn Widget) we could even integrate these both ways. Lifetimes
    // could get baaad though, ideally we would evaluate closures as soon as they are added to the
    // AtomElement. We could call the single pass ui based on the rect from last frame.
    ui.add(AtomKind::ui(|ui: &mut Ui| {
        my_single_pass_ui(ui);
    }))
}
