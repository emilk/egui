//! Old and deprecated API for popups. Use [`Popup`] instead.
#![expect(deprecated)]

use crate::containers::tooltip::Tooltip;
use crate::{
    Align, Context, Id, LayerId, Layout, Popup, PopupAnchor, PopupCloseBehavior, Pos2, Rect,
    Response, Ui, Widget as _, WidgetText,
};
use emath::RectAlign;
// ----------------------------------------------------------------------------

/// Show a tooltip at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// Returns `None` if the tooltip could not be placed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # #[expect(deprecated)]
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("my_tooltip"), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// # });
/// ```
#[deprecated = "Use `egui::Tooltip` instead"]
pub fn show_tooltip<R>(
    ctx: &Context,
    parent_layer: LayerId,
    widget_id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    show_tooltip_at_pointer(ctx, parent_layer, widget_id, add_contents)
}

/// Show a tooltip at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// Returns `None` if the tooltip could not be placed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("my_tooltip"), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// # });
/// ```
#[deprecated = "Use `egui::Tooltip` instead"]
pub fn show_tooltip_at_pointer<R>(
    ctx: &Context,
    parent_layer: LayerId,
    widget_id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    Tooltip::always_open(ctx.clone(), parent_layer, widget_id, PopupAnchor::Pointer)
        .gap(12.0)
        .show(add_contents)
        .map(|response| response.inner)
}

/// Show a tooltip under the given area.
///
/// If the tooltip does not fit under the area, it tries to place it above it instead.
#[deprecated = "Use `egui::Tooltip` instead"]
pub fn show_tooltip_for<R>(
    ctx: &Context,
    parent_layer: LayerId,
    widget_id: Id,
    widget_rect: &Rect,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    Tooltip::always_open(ctx.clone(), parent_layer, widget_id, *widget_rect)
        .show(add_contents)
        .map(|response| response.inner)
}

/// Show a tooltip at the given position.
///
/// Returns `None` if the tooltip could not be placed.
#[deprecated = "Use `egui::Tooltip` instead"]
pub fn show_tooltip_at<R>(
    ctx: &Context,
    parent_layer: LayerId,
    widget_id: Id,
    suggested_position: Pos2,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    Tooltip::always_open(ctx.clone(), parent_layer, widget_id, suggested_position)
        .show(add_contents)
        .map(|response| response.inner)
}

/// Show some text at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_text`].
///
/// See also [`show_tooltip`].
///
/// Returns `None` if the tooltip could not be placed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip_text(ui.ctx(), ui.layer_id(), egui::Id::new("my_tooltip"), "Helpful text");
/// }
/// # });
/// ```
#[deprecated = "Use `egui::Tooltip` instead"]
pub fn show_tooltip_text(
    ctx: &Context,
    parent_layer: LayerId,
    widget_id: Id,
    text: impl Into<WidgetText>,
) -> Option<()> {
    show_tooltip(ctx, parent_layer, widget_id, |ui| {
        crate::widgets::Label::new(text).ui(ui);
    })
}

/// Was this tooltip visible last frame?
#[deprecated = "Use `Tooltip::was_tooltip_open_last_frame` instead"]
pub fn was_tooltip_open_last_frame(ctx: &Context, widget_id: Id) -> bool {
    Tooltip::was_tooltip_open_last_frame(ctx, widget_id)
}

/// Indicate whether a popup will be shown above or below the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AboveOrBelow {
    Above,
    Below,
}

/// Helper for [`popup_above_or_below_widget`].
#[deprecated = "Use `egui::Popup` instead"]
pub fn popup_below_widget<R>(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    close_behavior: PopupCloseBehavior,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    popup_above_or_below_widget(
        ui,
        popup_id,
        widget_response,
        AboveOrBelow::Below,
        close_behavior,
        add_contents,
    )
}

/// Shows a popup above or below another widget.
///
/// Useful for drop-down menus (combo boxes) or suggestion menus under text fields.
///
/// The opened popup will have a minimum width matching its parent.
///
/// You must open the popup with [`crate::Memory::open_popup`] or  [`crate::Memory::toggle_popup`].
///
/// Returns `None` if the popup is not open.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let response = ui.button("Open popup");
/// let popup_id = ui.make_persistent_id("my_unique_id");
/// if response.clicked() {
///     ui.memory_mut(|mem| mem.toggle_popup(popup_id));
/// }
/// let below = egui::AboveOrBelow::Below;
/// let close_on_click_outside = egui::PopupCloseBehavior::CloseOnClickOutside;
/// # #[expect(deprecated)]
/// egui::popup_above_or_below_widget(ui, popup_id, &response, below, close_on_click_outside, |ui| {
///     ui.set_min_width(200.0); // if you want to control the size
///     ui.label("Some more info, or things you can select:");
///     ui.label("…");
/// });
/// # });
/// ```
#[deprecated = "Use `egui::Popup` instead"]
pub fn popup_above_or_below_widget<R>(
    _parent_ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    above_or_below: AboveOrBelow,
    close_behavior: PopupCloseBehavior,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let response = Popup::from_response(widget_response)
        .layout(Layout::top_down_justified(Align::LEFT))
        .open_memory(None)
        .close_behavior(close_behavior)
        .id(popup_id)
        .align(match above_or_below {
            AboveOrBelow::Above => RectAlign::TOP_START,
            AboveOrBelow::Below => RectAlign::BOTTOM_START,
        })
        .width(widget_response.rect.width())
        .show(|ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui)
        })?;
    Some(response.inner)
}
