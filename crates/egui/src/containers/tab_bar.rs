use emath::{Align, Rect, Vec2, vec2};

use crate::{
    AsIdSalt, Button, EventFilter, Id, IdSalt, InnerResponse, Key, Layout, Modifiers, Popup,
    Response, Role, ScrollArea, Sense, Ui, UiBuilder, UiStackInfo, WidgetInfo,
    scroll_area::ScrollBarVisibility,
};

/// What a [`TabBar`] remembers between passes.
#[derive(Clone, Default)]
struct State {
    /// The tabs added to the bar so far this pass, in order.
    tabs: Vec<TabInfo>,

    /// Did the tabs need more room than the bar had, last pass?
    overflowing: bool,

    /// A tab picked from the list of all tabs. It acts as if it was clicked, the next pass.
    picked: Option<Id>,
}

/// What a [`TabBar`] knows about one of its [`crate::Tab`]s.
#[derive(Clone)]
struct TabInfo {
    id: Id,
    text: Option<String>,
    selected: bool,
    enabled: bool,
}

/// A horizontal bar of [`crate::Tab`]s, with a line below it.
///
/// The result will look like this:
/// ```text
///                        parent Ui
///  ______________________________________________________
/// |                                                      |
/// |   Home     Settings     About                        |
/// |___________========___________________________________|
/// |                                                      |
/// |   the contents of the selected tab, added by you     |
/// |                                                      |
/// ```
///
/// The tab bar only shows the tabs.
/// You decide what to show below it, based on which tab is selected.
///
/// The bar takes up the full width of the parent [`Ui`].
/// If the tabs do not fit, the bar can be scrolled sideways.
/// It has no scroll bar: instead it fades out towards the side with more tabs,
/// and clicking or focusing a tab that is partly hidden scrolls all of it into view.
/// A button also shows up at the end of the bar, with a menu listing all the tabs.
///
/// Each tab is its own focus stop. Tab moves between them like between any other widgets,
/// the left and right arrow keys move to the previous and next tab,
/// and Space or Enter selects the focused one.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// #[derive(PartialEq)]
/// enum Page { Home, Settings }
/// let mut page = Page::Home;
///
/// egui::TabBar::new().show(ui, |ui| {
///     ui.tab_value(&mut page, Page::Home, "Home");
///     ui.tab_value(&mut page, Page::Settings, "Settings");
/// });
///
/// match page {
///     Page::Home => ui.label("Welcome home"),
///     Page::Settings => ui.label("Nothing to configure"),
/// };
/// # });
/// ```
#[must_use = "You should call tab_bar.show()"]
#[derive(Clone, Copy, Debug, Default)]
pub struct TabBar {
    id_salt: Option<IdSalt>,
}

impl TabBar {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// A source for the unique [`crate::Id`], e.g. `.id_salt("second_tab_bar")` or `.id_salt(loop_index)`.
    ///
    /// The scroll offset of the bar is stored using this id.
    /// Set this if the bar can move within its parent [`Ui`], e.g. because widgets above it come and go.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// Show the tab bar, adding the tabs left-to-right.
    ///
    /// You can add any widget to the bar, not only [`crate::Tab`]s,
    /// e.g. a button for opening a new tab.
    pub fn show<R>(self, ui: &mut Ui, add_tabs: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let Self { id_salt } = self;

        let mut ui_builder = UiBuilder::new().layout(Layout::top_down(Align::Min));
        if let Some(id_salt) = id_salt {
            ui_builder = ui_builder.id_salt(id_salt);
        }

        ui.scope_builder(ui_builder, |ui| {
            let id = ui.scope_id();
            let item_spacing = ui.spacing().item_spacing;
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;

            // The line goes right below the tabs:
            ui.spacing_mut().item_spacing.y = 0.0;

            // The bar has no scroll bar, so the fade is the only hint that there are more tabs.
            // Make it a strong one, unless the fade has been turned off:
            let fade = &mut ui.spacing_mut().scroll.fade;
            if 0.0 < fade.strength {
                fade.strength = 1.0;
            }

            // Forget the tabs of the previous pass:
            let was_overflowing = ui.data_mut(|data| {
                let state = data.get_temp_mut_or_default::<State>(id);
                state.tabs.clear();
                state.overflowing
            });

            // Start out as low as a single widget, like `Ui::horizontal` does:
            let bar_rect = Rect::from_min_size(
                ui.cursor().min,
                vec2(ui.available_width(), ui.spacing().interact_size.y),
            );

            // If the tabs did not fit last pass, leave room for the button listing them all:
            let list_button_size = Vec2::splat(ui.spacing().interact_size.y);
            let mut tabs_rect = bar_rect;
            if was_overflowing {
                tabs_rect.max.x -= list_button_size.x + item_spacing.x;
            }

            let output = ui
                .scope_builder(UiBuilder::new().max_rect(tabs_rect), |ui| {
                    ScrollArea::horizontal()
                        .auto_shrink([false, true])
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = item_spacing;

                            let max_rect = Rect::from_min_size(ui.max_rect().min, tabs_rect.size());

                            ui.scope_builder(
                                UiBuilder::new()
                                    .max_rect(max_rect)
                                    .layout(Layout::left_to_right(Align::Center))
                                    .accessibility_role(Role::TabList)
                                    .ui_stack_info(
                                        UiStackInfo::default().with_tag_value(TAB_BAR_TAG, id),
                                    ),
                                add_tabs,
                            )
                            .inner
                        })
                })
                .inner;

            let bar_rect = bar_rect.with_max_y(output.inner_rect.bottom());

            // Compare with the full width of the bar, and not with what is left beside the
            // button, or the button would come and go when the tabs only just fit.
            let overflowing = bar_rect.width() < output.content_size.x;

            let tabs = ui.data_mut(|data| {
                let state = data.get_temp_mut_or_default::<State>(id);
                state.overflowing = overflowing;

                // Whatever was picked last pass has been handled by now, or its tab is gone:
                state.picked = None;

                state.tabs.clone()
            });

            move_focus_with_arrow_keys(ui, &tabs);

            if overflowing != was_overflowing {
                // Show or hide the button:
                ui.request_repaint();
            }

            if was_overflowing {
                let button_rect = Rect::from_center_size(
                    emath::pos2(
                        bar_rect.right() - 0.5 * list_button_size.x,
                        bar_rect.center().y,
                    ),
                    list_button_size,
                );
                let response = tab_list_button(ui, button_rect);

                if let Some(picked) = tab_list_menu(&response, &tabs) {
                    ui.data_mut(|data| {
                        data.get_temp_mut_or_default::<State>(id).picked = Some(picked);
                    });
                    ui.request_repaint();
                }
            }

            let (line_rect, _response) =
                ui.allocate_exact_size(vec2(bar_rect.width(), stroke.width), Sense::hover());
            ui.painter().rect_filled(line_rect, 0.0, stroke.color);

            output.inner
        })
    }

    /// Called by each [`crate::Tab`], so that the [`TabBar`] it is in (if any) knows about it.
    ///
    /// Returns `true` if the tab was picked from the list of all tabs,
    /// in which case it should act as if it was clicked.
    pub(crate) fn register_tab(ui: &Ui, id: Id, text: Option<&str>, selected: bool) -> bool {
        let Some(tab_bar_id) = ui
            .stack()
            .iter()
            .find_map(|stack| stack.info.tags.get_downcast::<Id>(TAB_BAR_TAG).copied())
        else {
            return false;
        };

        if ui.memory(|memory| memory.has_focus(id)) {
            // The left and right arrow keys are handled by `move_focus_with_arrow_keys`:
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            });
        }

        ui.data_mut(|data| {
            let state = data.get_temp_mut_or_default::<State>(tab_bar_id);
            state.tabs.push(TabInfo {
                id,
                text: text.map(String::from),
                selected,
                enabled: ui.is_enabled(),
            });
            state.picked == Some(id)
        })
    }
}

/// The tag in the [`crate::UiStack`] holding the [`Id`] of the enclosing [`TabBar`].
const TAB_BAR_TAG: &str = "egui_tab_bar";

/// If one of the tabs has the keyboard focus, the left and right arrow keys move it
/// to the previous or next enabled tab.
///
/// The default arrow key navigation of egui looks for the closest widget in that direction.
/// That cannot reach a tab that is scrolled out of view, and can end up outside of the bar.
fn move_focus_with_arrow_keys(ui: &Ui, tabs: &[TabInfo]) {
    let Some(focused) = ui.memory(|memory| tabs.iter().position(|tab| memory.has_focus(tab.id)))
    else {
        return;
    };

    let (left, right) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::ArrowLeft),
            input.consume_key(Modifiers::NONE, Key::ArrowRight),
        )
    });

    let next = if right {
        tabs.iter().skip(focused + 1).find(|tab| tab.enabled)
    } else if left {
        tabs.iter().take(focused).rev().find(|tab| tab.enabled)
    } else {
        None
    };

    if let Some(next) = next {
        // Next pass, so that the tab sees `Response::gained_focus` and scrolls into view:
        ui.memory_mut(|memory| memory.request_focus_next_frame(next.id));
        ui.request_repaint();
    }
}

/// The button at the end of an overflowing [`TabBar`], opening a list of all its tabs.
fn tab_list_button(ui: &mut Ui, rect: Rect) -> Response {
    let response = ui.allocate_rect(rect, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), "Show all tabs"));

    if ui.is_rect_visible(rect) {
        let is_open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
        let visuals = if is_open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };

        // Like a tab: no frame at rest.
        if is_open || response.hovered() || response.has_focus() {
            ui.painter().rect(
                rect,
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                epaint::StrokeKind::Inside,
            );
        }

        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ui.spacing().icon_width));
        super::combo_box::paint_default_icon(ui.painter(), icon_rect, visuals);
    }

    response
}

/// The menu of [`tab_list_button`]. Returns the tab that was picked, if any.
fn tab_list_menu(button_response: &Response, tabs: &[TabInfo]) -> Option<Id> {
    let mut picked = None;

    Popup::menu(button_response).show(|ui| {
        // Same as a `ComboBox`, which also lists alternatives:
        ScrollArea::vertical()
            .max_height(ui.spacing().combo_height)
            .show(ui, |ui| {
                // Justified, so that the whole row of each tab can be clicked:
                ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                    for (index, tab) in tabs.iter().enumerate() {
                        let text = match &tab.text {
                            Some(text) => text.clone(),

                            // A tab with only an image on it:
                            None => format!("Tab {}", index + 1),
                        };
                        let button = Button::new(text).selected(tab.selected);
                        if ui.add_enabled(tab.enabled, button).clicked() {
                            picked = Some(tab.id);
                        }
                    }
                });
            });
    });

    picked
}
