use crate::{
    Atoms, Button, IntoAtoms, Rect, Response, Role, Sense, TabBar, TextStyle, TextWrapMode, Ui,
    Widget, WidgetAtom, WidgetAtomResponse, WidgetInfo,
    class::{ClassName, Classes, HasClasses},
    response::Flags,
    widget_style::TabStyle,
};

/// One tab out of several, either selected or not.
///
/// Usually you'd use [`Ui::tab_value`] inside a [`crate::TabBar`] instead.
///
/// A tab only reports clicks: it is up to you to remember which tab is selected,
/// and to show the contents that belong to it.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// #[derive(PartialEq)]
/// enum Page { Home, Settings }
/// let mut page = Page::Home;
///
/// ui.tab_value(&mut page, Page::Home, "Home");
///
/// // is equivalent to:
///
/// if ui.add(egui::Tab::new(page == Page::Home, "Home")).clicked() {
///     page = Page::Home;
/// }
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Tab<'a> {
    layout: WidgetAtom<'a>,
    selected: bool,
    classes: Classes,
}

impl<'a> Tab<'a> {
    /// Present on the selected tab.
    pub const CLASS_SELECTED: ClassName = Button::CLASS_SELECTED;

    /// A tab showing some text and/or images.
    ///
    /// When `selected`, [`Self::CLASS_SELECTED`] is added.
    pub fn new(selected: bool, atoms: impl IntoAtoms<'a>) -> Self {
        let mut classes = Classes::default();
        classes.set_class(Self::CLASS_SELECTED, selected);

        Self {
            layout: WidgetAtom::new(atoms.into_atoms())
                .sense(Sense::click())
                .fallback_font(TextStyle::Button),
            selected,
            classes,
        }
    }

    /// Set the wrap mode for the text.
    ///
    /// By default, [`crate::Ui::wrap_mode`] will be used, which can be overridden with [`crate::Style::wrap_mode`].
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.layout = self.layout.wrap_mode(wrap_mode);
        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Truncate`].
    #[inline]
    pub fn truncate(self) -> Self {
        self.wrap_mode(TextWrapMode::Truncate)
    }

    /// Output the tab's [`Atoms`].
    ///
    /// This includes any images you have on the tab.
    pub fn atoms(&self) -> &Atoms<'a> {
        &self.layout.atoms
    }

    /// Show the tab and return a [`WidgetAtomResponse`] for painting custom contents.
    pub fn atom_ui(self, ui: &mut Ui) -> WidgetAtomResponse {
        let Tab {
            layout,
            selected,
            classes,
        } = self;

        let text = layout.text().map(String::from);

        // Get the widget style by reading the response from the previous pass
        let id = ui.next_auto_id();
        let TabStyle {
            atom_layout,
            indicator,
        } = ui.widget_style(id, &classes);

        let prepared = atom_layout.apply(layout).allocate(ui);

        let mut prepared = prepared;
        if TabBar::register_tab(ui, prepared.response.id, text.as_deref(), selected) {
            // Picked from the list of all tabs of the `TabBar`:
            prepared
                .response
                .flags
                .set(Flags::FAKE_PRIMARY_CLICKED, true);
        }

        prepared.response.widget_info(|| {
            WidgetInfo::selected(
                Role::Tab,
                ui.is_enabled(),
                selected,
                text.as_deref().unwrap_or(""),
            )
        });

        if prepared.response.clicked() || prepared.response.gained_focus() {
            // The tab may be partly outside of a scrolled `TabBar`. Bring all of it into view:
            prepared.response.scroll_to_me(None);
        }

        if let Some(cursor) = ui.visuals().interact_cursor
            && prepared.response.hovered()
        {
            ui.ctx().set_cursor_icon(cursor);
        }

        if ui.is_rect_visible(prepared.response.rect) {
            let response = prepared.paint(ui);

            if !indicator.is_empty() {
                // Along the bottom edge, on top of the frame:
                let rect = response.response.rect;
                let indicator_rect = Rect::from_min_max(
                    rect.left_bottom() - emath::vec2(0.0, indicator.width),
                    rect.right_bottom(),
                );
                ui.painter()
                    .rect_filled(indicator_rect, 0.0, indicator.color);
            }

            response
        } else {
            WidgetAtomResponse::empty(prepared.response)
        }
    }
}

impl Widget for Tab<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.atom_ui(ui).response
    }
}

impl HasClasses for Tab<'_> {
    fn classes(&self) -> &Classes {
        &self.classes
    }

    fn classes_mut(&mut self) -> &mut Classes {
        &mut self.classes
    }
}
