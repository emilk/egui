# egui changelog

All notable changes to the egui crate will be documented in this file.

NOTE: `epi`, `eframe`, `egui_web` and `egui_glium` has their own changelogs!

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).


## Unreleased

### Added ⭐

* Add support for secondary and middle mouse buttons.
* Add `Label` methods for code, strong, strikethrough, underline and italics.
* Add `ui.group(|ui| { … })` to visually group some widgets within a frame
* Text will now wrap at newlines, spaces, dashes, punctuation or in the middle of a words if necessary, in that order of priority.
* `egui::popup::popup_below_widget`: show a popup area below another widget.
* Add `Slider::clamp_to_range(bool)`: if set, clamp the incoming and outgoing values to the slider range.
* Add: `ui.spacing()`, `ui.spacing_mut()`, `ui.visuals()`, `ui.visuals_mut()`.
* Add: `ctx.set_visuals()`.

### Changed 🔧

* `mouse` has be renamed `pointer` everywhere (to make it clear it includes touches too).
* Most parts of `Response` are now methods, so `if ui.button("…").clicked {` is now `if ui.button("…").clicked() {`.
* `Response::active` is now gone. You can use `response.dragged()` or `response.clicked()` instead.
* Backend: pointer (mouse/touch) position and buttons are now passed to egui in the event stream.
* `DragValue::range` is now called `clamp_range` and also clamps incoming values.
* Renamed `Triangles` to `Mesh`.
* The tesselator now wraps the clip rectangle and mesh in `struct ClippedMesh(Rect, Mesh)`.
* `Mesh::split_to_u16` now returns a 16-bit indexed `Mesh16`.

### Fixed 🐛

* It is now possible to click widgets even when FPS is very low.


## 0.8.0 - 2021-01-17 - Grid layout & new visual style

<img src="media/widget_gallery_0.8.0.gif" width="50%">

### Added ⭐

* Added a simple grid layout (`Grid`).
* Added `ui.allocate_at_least` and `ui.allocate_exact_size`.
* Added function `InputState::key_down`.
* Added `Window::current_pos` to position a window.

### Changed 🔧

* New simpler and sleeker look!
* Rename `PaintCmd` to `Shape`.
* Replace tuple `(Rect, Shape)` with tuple-struct `ClippedShape`.
* Rename feature `"serde"` to `"persistence"`.
* Break out the modules `math` and `paint` into separate crates `emath` and `epaint`.

### Fixed 🐛

* Fixed a bug that would sometimes trigger a "Mismatching panels" panic in debug builds.
* `Image` and `ImageButton` will no longer stretch to fill a justified layout.


## 0.7.0 - 2021-01-04

### Added ⭐

* Add `ui.scroll_to_cursor` and `response.scroll_to_me` ([#81](https://github.com/emilk/egui/pull/81) by [lucaspoffo](https://github.com/lucaspoffo)).
* Add `window.id(…)` and `area.id(…)` for overriding the default `Id`.

### Changed 🔧

* Renamed `Srgba` to `Color32`.
* All color contructions now starts with `from_`, e.g. `Color32::from_rgb`.
* Renamed `FontFamily::VariableWidth` to `FontFamily::Proportional`.
* Removed `pixels_per_point` from `FontDefinitions`.

### Fixed 🐛

* `RepaintSignal` now implements `Sync` so it can be sent to a background thread.
* `TextEdit` widgets are now slightly larger to accommodate their frames.

### Deprecated

* Deprecated `color::srgba`.


## 0.6.0 - 2020-12-26

### Added ⭐

* Turn off `Window` title bars with `window.title_bar(false)`.
* `ImageButton` - `ui.add(ImageButton::new(...))`.
* `ui.vertical_centered` and `ui.vertical_centered_justified`.
* `ui.allocate_painter` helper.
* Mouse-over explanation to duplicate ID warning.
* You can now easily constrain egui to a portion of the screen using `RawInput::screen_rect`.
* You can now control the minimum and maixumum number of decimals to show in a `Slider` or `DragValue`.
* Add `egui::math::Rot2`: rotation helper.
* `Response` now contains the `Id` of the widget it pertains to.
* `ui.allocate_response` that allocates space and checks for interactions.
* Add `response.interact(sense)`, e.g. to check for clicks on labels.

### Changed 🔧

* `ui.allocate_space` now returns an `(Id, Rect)` tuple.
* `Arc<Context>` has been replaced with `CtxRef` everywhere.
* Slight tweak of the default `Style` and font sizes.
* `SidePanel::left` and `TopPanel::top` now takes `impl Hash` as first argument.
* A `Window` may now cover an existing `CentralPanel`.
* `ui.image` now takes `impl Into<Vec2>` as a `size` argument.
* Made some more fields of `RawInput` optional.
* `Slider` and `DragValue` uses fewer decimals by default. See the full precision by hovering over the value.
* `egui::App`: added `fn name(&self)` and `fn clear_color(&self)`.
* Combo boxes has scroll bars when needed.
* Expand `Window` + `Resize` containers to be large enough for last frames content
* `ui.columns`: Columns now defaults to justified top-to-down layouts.
* Rename `Sense::nothing()` to  `Sense::hover()`.
* Replaced `parking_lot` dependency with `atomic_refcell` by default.

### Fixed 🐛

* The background for `CentralPanel` will now cover unused space too.
* `ui.columns`: Improve allocated size estimation.

### Deprecated
* `RawInput::screen_size` - use `RawInput::screen_rect` instead.
* left/centered/right column functions on `Ui`.
* `ui.interact_hover` and `ui.hovered`.


## 0.5.0 - 2020-12-13

### Added ⭐

* Emoji support: 1216 different emojis that work in any text.
  * The Demo app comes with a Font Book to explore the available glyphs.
* `ui.horizontal_wrapped(|ui| ...)`: Add widgets on a row but wrap at `max_size`.
* `ui.horizontal_wrapped_for_text`: Like `ui.horizontal_wrapped`, but with spacing made for embedding text.
* `ui.horizontal_for_text`: Like `ui.horizontal`, but with spacing made for embedding text.
* `egui::Layout` now supports justified layouts where contents is _also_ centered, right-aligned, etc.
* `ui.allocate_ui(size, |ui| ...)`: Easily create a child-`Ui` of a given size.
* `SelectableLabel` (`ui.selectable_label` and `ui.selectable_value`): A text-button that can be selected.
* `ui.small_button`: A smaller button that looks good embedded in text.
* `ui.drag_angle_tau`: For those who want to specify angles as fractions of τ (a full turn).
* Add `Resize::id_source` and `ScrollArea::id_source` to let the user avoid Id clashes.

### Changed 🔧

* New default font: [Ubuntu-Light](https://fonts.google.com/specimen/Ubuntu).
* Make it simpler to override fonts in `FontDefinitions`.
* Remove minimum button width.
* Refactor `egui::Layout` substantially, changing its interface.
* Calling `on_hover_text`/`on_hover_ui` multiple times will stack tooltips underneath the previous ones.
* Text wrapping on labels, buttons, checkboxes and radio buttons is now based on the layout.

### Removed 🔥

* Removed the `label!` macro.


## 0.4.0 - 2020-11-28

### Added ⭐

* `TextEdit` improvements:
  * Much improved text editing, with better navigation and selection.
  * Move focus between `TextEdit` widgets with tab and shift-tab.
  * Undo edtis in a `TextEdit`.
  * You can now check if a `TextEdit` lost keyboard focus with `response.lost_kb_focus`.
  * Added `ui.text_edit_singleline` and `ui.text_edit_multiline`.
* You can now debug why your `Ui` is unexpectedly wide with `ui.style_mut().visuals.debug_expand_width = true;`

### Changed 🔧

* Pressing enter in a single-line `TextEdit` will now surrender keyboard focus for it.
* You must now be explicit when creating a `TextEdit` if you want it to be singeline or multiline.
* Improved automatic `Id` generation, making `Id` clashes less likely.
* egui now requires modifier key state from the integration
* Added, renamed and removed some keys in the `Key` enum.
* Fixed incorrect text wrapping width on radio buttons

### Fixed 🐛

* Fixed bug where a lost widget could still retain keyboard focus.


## 0.3.0 - 2020-11-07

### Added ⭐

* Panels: you can now create panels using `SidePanel`, `TopPanel` and `CentralPanel`.
* You can now override the default egui fonts.
* Add ability to override text color with `visuals.override_text_color`.
* The demo now includes a simple drag-and-drop example.
* The demo app now has a slider to scale all of egui.

### Changed 🔧

* `ui.horizontal(...)` etc returns `Response`.
* Refactored the interface for `egui::app::App`.
* Windows are now constrained to the screen.
* `Context::begin_frame()` no longer returns a `Ui`. Instead put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
* `Context::end_frame()` now returns shapes that need to be converted to triangles with `Context::tessellate()`.
* Anti-aliasing is now off by default in debug builds.

### Removed 🔥

* You can no longer throw windows.

### Fixed 🐛

* Fix a bug where some regions would slowly grow for non-integral scales (`pixels_per_point`).


## 0.2.0 - 2020-10-10

* Color picker
* Unicode characters in labels (limited by [what the default font supports](https://fonts.google.com/specimen/Comfortaa#glyphs))
* Simple drop-down combo box menu
* Logarithmic sliders
* Optimization: coarse culling in the tessellator
* CHANGED: switch argument order of `ui.checkbox` and `ui.radio`

## 0.1.4 - 2020-09-08

This is when I started the CHANGELOG.md, after almost two years of development. Better late than never.

* Widgets: label, text button, hyperlink, checkbox, radio button, slider, draggable value, text editing
* Layouts: horizontal, vertical, columns
* Text input: very basic, multiline, copy/paste
* Windows: move, resize, name, minimize and close. Automatically sized and positioned.
* Regions: resizing, vertical scrolling, collapsing headers (sections)
* Rendering: Anti-aliased rendering of lines, circles, text and convex polygons.
* Tooltips on hover
