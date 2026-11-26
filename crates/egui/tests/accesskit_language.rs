//! Tests for the UI language on the AccessKit tree:
//! [`Context::set_language`] on the root node, [`UiBuilder::language`] on a `Ui` node,
//! and [`Response::accessible_language`] on a single widget.

use egui::{Context, RawInput, Ui, UiBuilder, accesskit::TreeUpdate};

fn accesskit_output_single_egui_frame(run_ui: impl FnMut(&mut Ui)) -> TreeUpdate {
    let ctx = Context::default();
    // Disable animations, so we do not need to wait for animations to end to see the result.
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    ctx.enable_accesskit();

    let mut output = ctx.run_ui(RawInput::default(), run_ui);
    output.textures_delta.clear(); // Don't panic on drop with unapplied deltas

    output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update")
}

fn node_language(update: &TreeUpdate, id: egui::Id) -> Option<String> {
    update
        .nodes
        .iter()
        .find(|(node_id, _)| *node_id == id.accesskit_id())
        .map(|(_, node)| node.language().map(str::to_owned))
        .unwrap_or_else(|| panic!("No AccessKit node for {id:?}"))
}

#[test]
fn context_language_goes_on_the_root_node() {
    let ctx = Context::default();
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    ctx.enable_accesskit();
    ctx.set_language(Some("ar".to_owned()));
    assert_eq!(ctx.language().as_deref(), Some("ar"));

    let mut output = ctx.run_ui(RawInput::default(), |ui| {
        ui.label("Hello");
    });
    output.textures_delta.clear();
    let update = output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update");

    assert_eq!(
        node_language(&update, egui::accesskit_root_id()),
        Some("ar".to_owned()),
        "The root node should carry the UI language"
    );
}

#[test]
fn no_language_set_means_no_language_on_the_root() {
    let update = accesskit_output_single_egui_frame(|ui| {
        ui.label("Hello");
    });
    assert_eq!(node_language(&update, egui::accesskit_root_id()), None);
}

#[test]
fn ui_builder_language_overrides_and_resolves() {
    let ctx = Context::default();
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    ctx.enable_accesskit();
    ctx.set_language(Some("en".to_owned()));

    let mut outer_language = None;
    let mut inner_language = None;
    let mut inner_id = None;
    let mut output = ctx.run_ui(RawInput::default(), |ui| {
        outer_language = ui.language();
        ui.scope_builder(UiBuilder::new().language("he"), |ui| {
            inner_language = ui.language();
            inner_id = Some(ui.unique_id());
            ui.label("שלום");
        });
    });
    output.textures_delta.clear();
    let update = output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update");

    // Resolution order: nearest UiBuilder::language up the stack, else Context::language().
    assert_eq!(outer_language.as_deref(), Some("en"));
    assert_eq!(inner_language.as_deref(), Some("he"));

    // The language goes on the Ui's own AccessKit node.
    let inner_id = inner_id.expect("scope_builder should have run");
    assert_eq!(node_language(&update, inner_id), Some("he".to_owned()));
}

#[test]
fn response_accessible_language_on_one_widget() {
    let mut label_id = None;
    let update = accesskit_output_single_egui_frame(|ui| {
        let response = ui.label("Привет").accessible_language("ru");
        label_id = Some(response.id);
    });

    let label_id = label_id.expect("label should have run");
    assert_eq!(node_language(&update, label_id), Some("ru".to_owned()));
}
