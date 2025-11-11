# Accessibility in egui

egui’s AccessKit integration exposes native accessibility APIs on Windows and macOS (and any other platform that AccessKit supports). This guide explains how to enable it, what semantics you receive for free, and how to extend or debug the accessibility tree. It also covers the experimental web screen reader that eframe ships for platforms without AccessKit.

## Enabling accessibility

- **Supported platforms:** AccessKit’s winit adapter currently exposes native APIs on Windows (UIA) and macOS (NSAccessibility). Other targets (Linux/AT-SPI, Android, iOS, web) are still works in progress in the AccessKit project. On unsupported platforms, nothing breaks—AccessKit simply stays disabled unless you opt into eframe’s experimental web screen reader (described below).
- `egui` ships an `accesskit` Cargo feature. Enable it in your integration crate (for example `egui = { version = "...", features = ["accesskit"] }`).  
  `eframe` enables this feature by default; other integrations must opt in manually.
- `eframe`’s native (`winit`) backend automatically turns on `Context::enable_accesskit` when the OS requests the initial accessibility tree. If you build your own integration, call [`Context::enable_accesskit`](../crates/egui/src/context.rs#L3493) when you detect that AccessKit is active and [`Context::disable_accesskit`](../crates/egui/src/context.rs#L3527) once it deactivates.
- egui can emit spoken feedback on the web through an experimental SpeechSynthesis-based screen reader. Enable the `web_screen_reader` feature in `eframe`, then toggle `ctx.options_mut(|o| o.screen_reader = true)` (the egui web demo exposes this toggle in the “Backend” panel).

## What you get automatically

Most built-in widgets call [`Response::widget_info`](../crates/egui/src/response.rs#L781) so AccessKit receives roles, labels, values, and focus information without any work on your part. Examples:

- `Button`, `Checkbox`, `Slider`, `TextEdit`, `ProgressBar`, etc. map to the corresponding AccessKit roles.
- Labels provided to widgets (`ui.button("Save")`) become accessibility labels automatically.
- Widgets with sensed clicks or focus register the appropriate actions (focus, click) so assistive technologies can trigger them.

Because egui rebuilds the entire tree every frame, you don’t manage node lifetimes manually—just keep rendering your UI and AccessKit receives deltas through `FullOutput.platform_output.accesskit_update`.

## Adding semantics to custom widgets

For custom or composite widgets you may need to provide extra metadata:

```rust
ui.horizontal(|ui| {
    let label = ui.label("Radius (km)");
    let slider = ui.add(egui::Slider::new(&mut state.radius, 0.0..=10_000.0));
    slider.labelled_by(label.id); // hook up the textual label
});
```

- [`Response::labelled_by`](../crates/egui/src/response.rs#L897) links visible text to controls that render elsewhere (e.g. an icon-only button).
- If you are building a fully custom widget, call [`Response::widget_info`](../crates/egui/src/response.rs#L781) and fill a [`WidgetInfo`](../crates/egui/src/data/output.rs#L487) with the role, label, and values you want exposed.
- Use [`Response::output_event`](../crates/egui/src/response.rs#L821) to emit `OutputEvent::ValueChanged`, `OutputEvent::Clicked`, etc. when the widget changes state; the AccessKit node will mirror this info.

## Building hierarchy and groups

egui automatically infers parent/child relationships based on the widget tree, but some advanced layouts (e.g. detached panels or overlays) need manual control:

- [`UiBuilder::accessibility_parent`](../crates/egui/src/ui_builder.rs#L186) overrides the parent node for an entire `Ui`.
- [`Context::accesskit_node_builder`](../crates/egui/src/context.rs#L3493) lets you mutate the AccessKit node for a specific widget ID if you need to set extra fields (for example `live_regions`, `described_by`, custom actions, etc.).

## Responding to assistive-technology actions

When users activate controls via assistive technology, egui delivers an [`Event::AccessKitActionRequest`](../crates/egui/src/data/input.rs#L550) through the normal input stream. Handle these events the same way you handle mouse/keyboard input:

```rust
ctx.input(|i| {
    for event in &i.events {
        if let egui::Event::AccessKitActionRequest(request) = event {
            match request.action {
                accesskit::Action::Click => {
                    // Update your widget state and request a repaint.
                }
                accesskit::Action::Focus => { /* ... */ }
                _ => {}
            }
        }
    }
});
```

Treat these requests as user input: update your state and (if needed) call `ctx.request_repaint()` so the new tree is sent back to AccessKit.

## Inspecting the accessibility tree

- `FullOutput.platform_output.accesskit_update` (see [`PlatformOutput`](../crates/egui/src/data/output.rs#L128)) contains the delta sent to AccessKit. You can capture this to feed custom tooling or tests.
- The demo app ships with an [`AccessibilityInspectorPlugin`](../crates/egui_demo_app/src/accessibility_inspector.rs#L11) (shortcut `Cmd/Ctrl + Alt + I`). It visualizes the current tree, highlights nodes on hover, and lets you send AccessKit actions to widgets. Add the plugin to your app during development for a live view of the tree.

## Testing with AccessKit

[`egui_kittest`](../crates/egui_kittest/README.md) wraps AccessKit’s consumer API so you can write assertions against the accessibility tree:

```rust
let mut harness = egui_kittest::Harness::new_ui(|ui| ui.checkbox(&mut checked, "Enable radar"));
let checkbox = harness.get_by_label("Enable radar");
assert_eq!(checkbox.accesskit_node().toggled(), Some(accesskit::Toggled::False));
```

The harness lets you drive interactions (click, focus, type) and inspect node roles/labels/values in unit tests.

## Platforms without AccessKit

On platforms that lack native accessibility APIs (e.g. browsers today), consider enabling `NativeOptions::screen_reader` (web feature) or providing alternative UIs. The egui web demo exposes an experimental built-in reader that uses speech synthesis to read `PlatformOutput.events_description`.

As AccessKit grows new backends, you usually only need to upgrade `AccessKit`/`egui` and ensure your integration opts into the correct feature flag—your widget code stays unchanged as long as you provide the necessary metadata described above.
