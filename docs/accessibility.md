# Accessibility

egui can expose its widget tree through [AccessKit](https://accesskit.dev/).
When using `eframe`, AccessKit support is enabled by default on native
platforms where an adapter is available. Integrations that build directly on
`egui` can opt in with `Context::enable_accesskit()` before running frames.

## Labels

Most built-in widgets register their role and accessible name when shown
through the usual egui APIs. Buttons, checkboxes, radio buttons, sliders, combo
boxes, text edits, links, windows, images, and progress indicators all contribute
widget information to the AccessKit tree.

For inputs with a separate visible label, connect the input response to the
label:

```rust
let label = ui.label("User name:");
ui.text_edit_singleline(&mut user_name).labelled_by(label.id);
```

## Custom widgets

If a widget is visual-only or draws custom shapes, give it explicit widget
information:

```rust
use egui::{Sense, WidgetInfo, WidgetType};

let (_rect, response) = ui.allocate_exact_size(size, Sense::click());
// Paint using ui.painter().
response.widget_info(|| {
    WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), "Open color picker")
});
```

Choose the closest `WidgetType`. For non-interactive custom content, prefer a
label or image role where possible rather than leaving the node unnamed.

## Testing

`egui_kittest` builds on AccessKit, so tests can query the same roles and names
that screen readers consume:

```rust
use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable as _};

let mut harness = Harness::new_ui_state(
    |ui, accepted| {
        ui.checkbox(accepted, "Accept terms");
    },
    false,
);

harness
    .get_by_role_and_label(Role::CheckBox, "Accept terms")
    .click();
harness.run();

assert!(*harness.state());
```

You can also inspect the tree in the demo app when it is built with the
`accessibility_inspector` feature.
