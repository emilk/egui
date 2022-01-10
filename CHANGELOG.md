# egui changelog

All notable changes to the egui crate will be documented in this file.

NOTE: [`epaint`](epaint/CHANGELOG.md), [`eframe`](eframe/CHANGELOG.md), [`egui_web`](egui_web/CHANGELOG.md), [`egui-winit`](egui-winit/CHANGELOG.md), [`egui_glium`](egui_glium/CHANGELOG.md), and [`egui_glow`](egui_glow/CHANGELOG.md) have their own changelogs!


## Unreleased

### Added ⭐
* Added `Ui::add_visible` and `Ui::add_visible_ui`.

### Changed 🔧
* Renamed `Ui::visible` to `Ui::is_visible`.
* Split `Event::Text` into `Event::Text` and `Event::Paste` ([#1057](https://github.com/emilk/egui/issues/1057).

### Fixed 🐛
* Context menu now respects the theme ([#1043](https://github.com/emilk/egui/pull/1043))

## 0.16.1 - 2021-12-31 - Add back `CtxRef::begin_frame,end_frame`

### Added ⭐
* Add back `CtxRef::begin_frame,end_frame` as an alternative to `CtxRef::run`.


## 0.16.0 - 2021-12-29 - Context menus and rich text

### Added ⭐
* Added context menus: See `Ui::menu_button` and `Response::context_menu` ([#543](https://github.com/emilk/egui/pull/543)).
* Most widgets containing text (`Label`, `Button` etc) now supports rich text ([#855](https://github.com/emilk/egui/pull/855)).
* Plots:
  * Added bar charts and box plots ([#863](https://github.com/emilk/egui/pull/863)).
  * You can now query information about the plot (e.g. get the mouse position in plot coordinates, or the plot
    bounds) while adding items. `Plot` ([#766](https://github.com/emilk/egui/pull/766) and
    [#892](https://github.com/emilk/egui/pull/892)).
* You can now read and write the cursor of a `TextEdit` ([#848](https://github.com/emilk/egui/pull/848)).
* When using a custom font you can now specify a font index ([#873](https://github.com/emilk/egui/pull/873)).
* Added vertical sliders with `Slider::new(…).vertical()` ([#875](https://github.com/emilk/egui/pull/875)).
* Added `Button::image_and_text` ([#832](https://github.com/emilk/egui/pull/832)).
* Added `CollapsingHeader::open` to control if it is open or collapsed ([#1006](https://github.com/emilk/egui/pull/1006)).
* Added `egui::widgets::color_picker::color_picker_color32` to show the color picker.

### Changed 🔧
* MSRV (Minimum Supported Rust Version) is now `1.56.0`.
* `ui.add(Button::new("…").text_color(…))` is now `ui.button(RichText::new("…").color(…))` (same for `Label` )([#855](https://github.com/emilk/egui/pull/855)).
* Plots now provide a `show` method that has to be used to add items to and show the plot ([#766](https://github.com/emilk/egui/pull/766)).
* `menu::menu(ui, ...)` is now `ui.menu_button(...)` ([#543](https://github.com/emilk/egui/pull/543))
* Replaced `CtxRef::begin_frame` and `end_frame` with `CtxRef::run` ([#872](https://github.com/emilk/egui/pull/872)).
* Replaced `scroll_delta` and `zoom_delta` in `RawInput` with `Event::Scroll` and `Event::Zoom`.
* Unified the four `Memory` data buckets (`data`, `data_temp`, `id_data` and `id_data_temp`) into a single `Memory::data`, with a new interface ([#836](https://github.com/emilk/egui/pull/836)).
* Replaced `Ui::__test` with `egui::__run_test_ui` ([#872](https://github.com/emilk/egui/pull/872)).

### Fixed 🐛
* Fixed `ComboBox` and other popups getting clipped to parent window ([#885](https://github.com/emilk/egui/pull/885)).
* The color picker is now better at keeping the same hue even when saturation goes to zero ([#886](https://github.com/emilk/egui/pull/886)).

### Removed 🔥
* Removed `egui::math` (use `egui::emath` instead).
* Removed `egui::paint` (use `egui::epaint` instead).

### Contributors 🙏
* [5225225](https://github.com/5225225): [#849](https://github.com/emilk/egui/pull/849).
* [aevyrie](https://github.com/aevyrie): [#966](https://github.com/emilk/egui/pull/966).
* [B-Reif](https://github.com/B-Reif): [#875](https://github.com/emilk/egui/pull/875).
* [Bromeon](https://github.com/Bromeon): [#863](https://github.com/emilk/egui/pull/863), [#918](https://github.com/emilk/egui/pull/918).
* [d10sfan](https://github.com/d10sfan): [#832](https://github.com/emilk/egui/pull/832).
* [EmbersArc](https://github.com/EmbersArc): [#766](https://github.com/emilk/egui/pull/766), [#892](https://github.com/emilk/egui/pull/892).
* [Hperigo](https://github.com/Hperigo): [#905](https://github.com/emilk/egui/pull/905).
* [isegal](https://github.com/isegal): [#934](https://github.com/emilk/egui/pull/934).
* [mankinskin](https://github.com/mankinskin): [#543](https://github.com/emilk/egui/pull/543).
* [niladic](https://github.com/niladic): [#499](https://github.com/emilk/egui/pull/499), [#863](https://github.com/emilk/egui/pull/863).
* [singalen](https://github.com/singalen): [#973](https://github.com/emilk/egui/pull/973).
* [sumibi-yakitori](https://github.com/sumibi-yakitori): [#830](https://github.com/emilk/egui/pull/830), [#870](https://github.com/emilk/egui/pull/870).
* [t18b219k](https://github.com/t18b219k): [#868](https://github.com/emilk/egui/pull/868), [#888](https://github.com/emilk/egui/pull/888).


## 0.15.0 - 2021-10-24 - Syntax highlighting and hscroll

<img src="media/egui-0.15-code-editor.gif">

### Added ⭐
* Add horizontal scrolling support to `ScrollArea` and `Window` (opt-in).
* `TextEdit::layouter`: Add custom text layout for e.g. syntax highlighting or WYSIWYG.
* `Fonts::layout_job`: New text layout engine allowing mixing fonts, colors and styles, with underlining and strikethrough.
* Add `ui.add_enabled(bool, widget)` to easily add a possibly disabled widget.
* Add `ui.add_enabled_ui(bool, |ui| …)` to create a possibly disabled UI section.
* Add feature `"serialize"` separatedly from `"persistence"`.
* Add `egui::widgets::global_dark_light_mode_buttons` to easily add buttons for switching the egui theme.
* `TextEdit` can now be used to show text which can be selected and copied, but not edited.
* Add `Memory::caches` for caching things from one frame to the next.

### Changed 🔧
* Change the default monospace font to [Hack](https://github.com/source-foundry/Hack).
* Label text will now be centered, right-aligned and/or justified based on the layout of the `Ui` it is in.
* `Hyperlink` will now word-wrap just like a `Label`.
* All `Ui`:s must now have a finite `max_rect`.
  * Deprecated: `max_rect_finite`, `available_size_before_wrap_finite` and `available_rect_before_wrap_finite`.
* `Painter`/`Fonts`: text layout now expect a color when creating a `Galley`. You may override that color with `Painter::galley_with_color`.
* MSRV (Minimum Supported Rust Version) is now `1.54.0`.
* By default, `DragValue`:s no longer show a tooltip when hovered. Change with `Style::explanation_tooltips`.
* Smaller and nicer color picker.
* `ScrollArea` will auto-shrink to content size unless told otherwise using `ScollArea::auto_shrink`.
* By default, `Slider`'s `clamp_to_range` is set to true.
* Rename `TextEdit::enabled` to `TextEdit::interactive`.
* `ui.label` (and friends) now take `impl ToString` as argument instead of `impl Into<Label>`.

### Fixed 🐛
* Fix wrongly sized multiline `TextEdit` in justified layouts.
* Fix clip rectangle of windows that don't fit the central area.
* Show tooltips above widgets on touch screens.
* Fix popups sometimes getting clipped by panels.

### Removed 🔥
* Replace `Button::enabled` with `ui.add_enabled`.

### Contributors 🙏
* [AlexApps99](https://github.com/AlexApps99)
* [baysmith](https://github.com/baysmith)
* [bpostlethwaite](https://github.com/bpostlethwaite)
* [cwfitzgerald](https://github.com/cwfitzgerald)
* [DrOptix](https://github.com/DrOptix)
* [JerzySpendel](https://github.com/JerzySpendel)
* [NiceneNerd](https://github.com/NiceneNerd)
* [parasyte](https://github.com/parasyte)
* [spersson](https://github.com/spersson)
* [Stock84-dev](https://github.com/Stock84-dev)
* [sumibi-yakitori](https://github.com/sumibi-yakitori)
* [t18b219k](https://github.com/t18b219k)
* [TobTobXX](https://github.com/TobTobXX)
* [zu1k](https://github.com/zu1k)


## 0.14.2 - 2021-08-28 - Window resize fix

### Fixed 🐛
* Fix window resize bug introduced in `0.14.1`.


## 0.14.1 - 2021-08-28 - Layout bug fixes

### Added ⭐
* Add `Ui::horizontal_top`.

### Fixed 🐛
* Fix `set_width/set_min_width/set_height/set_min_height/expand_to_include_x/expand_to_include_y`.
* Make minimum grid column width propagate properly.
* Make sure `TextEdit` contents expand to fill width if applicable.
* `ProgressBar`: add a minimum width and fix for having it in an infinite layout.
* Fix sometimes not being able to click inside a combo box or popup menu.


## 0.14.0 - 2021-08-24 - Ui panels and bug fixes

### Added ⭐
* Panels can now be added to any `Ui`.
* Plot:
  * [Line styles](https://github.com/emilk/egui/pull/482).
  * Add `show_background` and `show_axes` methods to `Plot`.
* [Progress bar](https://github.com/emilk/egui/pull/519).
* `Grid::num_columns`: allow the last column to take up the rest of the space of the parent `Ui`.
* Add an API for dropping files into egui (see `RawInput`).
* `CollapsingHeader` can now optionally be selectable.

### Changed 🔧
* A single-line `TextEdit` will now clip text that doesn't fit in it, and scroll.
* Return closure return value from `Area::show`, `ComboBox::show_ui`, `ComboBox::combo_box_with_label`, `Window::show`, `popup::*`, `menu::menu`.
* Only move/resize windows with primary mouse button.
* Tooltips are now moved to not cover the widget they are attached to.

### Fixed 🐛
* Fix custom font definitions getting replaced when `pixels_per_point` is changed.
* Fix `lost_focus` for `TextEdit`.
* Clicking the edge of a menu button will now properly open the menu.
* Fix hover detection close to an `Area`.
* Fix case where `Plot`'s `min_auto_bounds` could be ignored after the first call to `Plot::ui`.
* Fix slow startup when using large font files.

### Contributors 🙏
* [barrowsys](https://github.com/barrowsys)
* [EmbersArc](https://github.com/EmbersArc)
* [gents83](https://github.com/gents83 )
* [lucaspoffo](https://github.com/lucaspoffo)
* [mankinskin](https://github.com/mankinskin)
* [mental32](https://github.com/mental32)
* [mitchmindtree](https://github.com/mitchmindtree)
* [parasyte](https://github.com/parasyte)
* [rekka](https://github.com/rekka)
* [zu1k](https://github.com/zu1k)


## 0.13.1 - 2021-06-28 - Plot fixes

### Added ⭐
* Plot: you can now set the stroke of a `HLine/VLine`.

### Changed 🔧
* `Plot::new` now takes an `id_source: impl Hash` instead of a `name: impl ToString`. Functionally it is the same.


## 0.13.0 - 2021-06-24 - Better panels, plots and new visual style

### Added ⭐
* Plot:
  * [More plot items: Arrows, Polygons, Text, Images](https://github.com/emilk/egui/pull/471).
  * [Plot legend improvements](https://github.com/emilk/egui/pull/410).
  * [Line markers for plots](https://github.com/emilk/egui/pull/363).
* Panels:
  * Add right and bottom panels (`SidePanel::right` and `Panel::bottom`).
  * Panels can now be resized.
  * Add an option to overwrite frame of a `Panel`.
* [Improve accessibility / screen reader](https://github.com/emilk/egui/pull/412).
* Add `ScrollArea::show_rows` for efficient scrolling of huge UI:s.
* Add `ScrollArea::enable_scrolling` to allow freezing scrolling when editing TextEdit widgets within it
* Add `Ui::set_visible` as a way to hide widgets.
* Add `Style::override_text_style` to easily change the text style of everything in a `Ui` (or globally).
* You can now change `TextStyle` on checkboxes, radio buttons and `SelectableLabel`.
* Add support for [cint](https://crates.io/crates/cint) under `cint` feature.
* Add features `extra_asserts` and `extra_debug_asserts` to enable additional checks.
* `TextEdit` now supports edits on a generic buffer using `TextBuffer`.
* Add `Context::set_debug_on_hover` and `egui::trace!(ui)`

### Changed 🔧
* Minimum Rust version is now 1.51 (used to be 1.52)
* [Tweaked the default visuals style](https://github.com/emilk/egui/pull/450).
* Plot: Renamed `Curve` to `Line`.
* `TopPanel::top` is now `TopBottomPanel::top`.
* `SidePanel::left` no longet takes the default width by argument, but by a builder call.
* `SidePanel::left` is resizable by default.

### Fixed 🐛
* Fix uneven lettering on non-integral device scales ("extortion lettering").
* Fix invisible scroll bar when native window is too narrow for egui.


## 0.12.0 - 2021-05-10 - Multitouch, user memory, window pivots, and improved plots

### Added ⭐
* Add anchors to windows and areas so you can put a window in e.g. the top right corner.
* Make labels interactive with `Label::sense(Sense::click())`.
* Add `Response::request_focus` and `Response::surrender_focus`.
* Add `TextEdit::code_editor` (VERY basic).
* [Pan and zoom plots](https://github.com/emilk/egui/pull/317).
* [Add plot legends](https://github.com/emilk/egui/pull/349).
* [Users can now store custom state in `egui::Memory`](https://github.com/emilk/egui/pull/257).
* Add `Response::on_disabled_hover_text` to show tooltip for disabled widgets.
* Zoom input: ctrl-scroll and (on `egui_web`) trackpad-pinch gesture.
* Support for raw [multi touch](https://github.com/emilk/egui/pull/306) events,
  enabling zoom, rotate, and more. Works with `egui_web` on mobile devices,
  and should work with `egui_glium` for certain touch devices/screens.
* Add (optional) compatibility with [mint](https://docs.rs/mint).

### Changed 🔧
* Make `Memory::has_focus` public (again).
* `Plot` must now be given a name that is unique within its scope.
* Tab only selects labels if the `screen_reader` option is turned on.
* Rename `ui.wrap` to `ui.scope`.

### Fixed 🐛
* Fix [defocus-bug on touch screens](https://github.com/emilk/egui/issues/288).
* Fix bug with the layout of wide `DragValue`:s.

### Removed 🔥
* Moved experimental markup language to `egui_demo_lib`


## 0.11.0 - 2021-04-05 - Optimization, screen reader & new layout logic

### Added ⭐
* You can now give focus to any clickable widget with tab/shift-tab.
  * Use space or enter to click the selected widget.
  * Use arrow keys to adjust sliders and `DragValue`s.
* egui will now output events when widgets gain keyboard focus.
  * This can be hooked up to a screen reader to aid the visually impaired
* Add the option to restrict the dragging bounds of `Window` and `Area` to a specified area using `drag_bounds(rect)`.
* Add support for small and raised text.
* Add `ui.set_row_height`.
* Add `DebugOptions::show_widgets` to debug layouting by hovering widgets.
* Add `ComboBox` to more easily customize combo boxes.
* Add `Slider::new` and `DragValue::new` to replace old type-specific constructors.
* Add `TextEdit::password` to hide input characters.

### Changed 🔧
* `ui.advance_cursor` is now called `ui.add_space`.
* `kb_focus` is now just called `focus`.

### Fixed 🐛
* Fix some bugs related to centered layouts.
* Fixed secondary-click to open a menu.
* [Fix panic for zero-range sliders and zero-speed drag values](https://github.com/emilk/egui/pull/216).
* Fix false id clash error for wrapping text.
* Fix bug that would close a popup (e.g. the color picker) when clicking inside of it.

### Deprecated ☢️
* Deprectated `combo_box_with_label` in favor of new `ComboBox`.
* Deprectated type-specific constructors for `Slider` and `DragValue` (`Slider::f32`, `DragValue::usize` etc).


## 0.10.0 - 2021-02-28 - Plot and polish

<img src="media/egui-0.10-plot.gif" width="50%">

### Added ⭐
* Add `egui::plot::Plot` to plot some 2D data.
* Add `Ui::hyperlink_to(label, url)`.
* Sliders can now have a value prefix and suffix (e.g. the suffix `"°"` works like a unit).
* `Context::set_pixels_per_point` to control the scale of the UI.
* Add `Response::changed()` to query if e.g. a slider was dragged, text was entered or a checkbox was clicked.
* Add support for all integers in `DragValue` and `Slider` (except 128-bit).

### Changed 🔧
* Improve the positioning of tooltips.
* Only show tooltips if mouse is still.
* `Slider` will now show the value display by default, unless turned off with `.show_value(false)`.
* The `Slider` value is now a `DragValue` which when dragged can pick values outside of the slider range (unless `clamp_to_range` is set).


## 0.9.0 - 2021-02-07 - Light Mode and much more

<img src="media/0.9.0-disabled.gif" width="50%">

### Added ⭐
* Add support for secondary and middle mouse buttons.
* Add `Label` methods for code, strong, strikethrough, underline and italics.
* Add `ui.group(|ui| { … })` to visually group some widgets within a frame.
* Add `Ui` helpers for doing manual layout (`ui.put`, `ui.allocate_ui_at_rect` and more).
* Add `ui.set_enabled(false)` to disable all widgets in a `Ui` (grayed out and non-interactive).
* Add `TextEdit::hint_text` for showing a weak hint text when empty.
* `egui::popup::popup_below_widget`: show a popup area below another widget.
* Add `Slider::clamp_to_range(bool)`: if set, clamp the incoming and outgoing values to the slider range.
* Add: `ui.spacing()`, `ui.spacing_mut()`, `ui.visuals()`, `ui.visuals_mut()`.
* Add: `ctx.set_visuals()`.
* You can now control text wrapping with `Style::wrap`.
* Add `Grid::max_col_width`.

### Changed 🔧
* Text will now wrap at newlines, spaces, dashes, punctuation or in the middle of a words if necessary, in that order of priority.
* Widgets will now always line break at `\n` characters.
* Widgets will now more intelligently choose wether or not to wrap text.
* `mouse` has been renamed `pointer` everywhere (to make it clear it includes touches too).
* Most parts of `Response` are now methods, so `if ui.button("…").clicked {` is now `if ui.button("…").clicked() {`.
* `Response::active` is now gone. You can use `response.dragged()` or `response.clicked()` instead.
* Backend: pointer (mouse/touch) position and buttons are now passed to egui in the event stream.
* `DragValue::range` is now called `clamp_range` and also clamps incoming values.
* Renamed `Triangles` to `Mesh`.
* The tessellator now wraps the clip rectangle and mesh in `struct ClippedMesh(Rect, Mesh)`.
* `Mesh::split_to_u16` now returns a 16-bit indexed `Mesh16`.

### Fixed 🐛
* It is now possible to click widgets even when FPS is very low.
* Tessellator: handle sharp path corners better (switch to bevel instead of miter joints for > 90°).


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

### Deprecated ☢️
* Deprecated `color::srgba`.


## 0.6.0 - 2020-12-26

### Added ⭐
* Turn off `Window` title bars with `window.title_bar(false)`.
* `ImageButton` - `ui.add(ImageButton::new(…))`.
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

### Deprecated ☢️
* `RawInput::screen_size` - use `RawInput::screen_rect` instead.
* left/centered/right column functions on `Ui`.
* `ui.interact_hover` and `ui.hovered`.


## 0.5.0 - 2020-12-13

### Added ⭐
* Emoji support: 1216 different emojis that work in any text.
  * The Demo app comes with a Font Book to explore the available glyphs.
* `ui.horizontal_wrapped(|ui| …)`: Add widgets on a row but wrap at `max_size`.
* `ui.horizontal_wrapped_for_text`: Like `ui.horizontal_wrapped`, but with spacing made for embedding text.
* `ui.horizontal_for_text`: Like `ui.horizontal`, but with spacing made for embedding text.
* `egui::Layout` now supports justified layouts where contents is _also_ centered, right-aligned, etc.
* `ui.allocate_ui(size, |ui| …)`: Easily create a child-`Ui` of a given size.
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
  * You can now check if a `TextEdit` lost keyboard focus with `response.lost_focus`.
  * Added `ui.text_edit_singleline` and `ui.text_edit_multiline`.
* You can now debug why your `Ui` is unexpectedly wide with `ui.style_mut().debug.show_expand_width = true;`

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
* `ui.horizontal(…)` etc returns `Response`.
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
