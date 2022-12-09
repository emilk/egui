# egui changelog
All notable changes to the `egui` crate will be documented in this file.

NOTE: [`epaint`](crates/epaint/CHANGELOG.md), [`eframe`](crates/eframe/CHANGELOG.md), [`egui-winit`](crates/egui-winit/CHANGELOG.md), [`egui_glium`](crates/egui_glium/CHANGELOG.md), [`egui_glow`](crates/egui_glow/CHANGELOG.md) and [`egui-wgpu`](crates/egui-wgpu/CHANGELOG.md) have their own changelogs!


## Unreleased
### Fixed 🐛
* Fix key-repeat for backspace and arrow keys in `TextEdit` ([#2416](https://github.com/emilk/egui/pull/2416)).


## 0.20.0 - 2022-12-08 - AccessKit, prettier text, overlapping widgets
* MSRV (Minimum Supported Rust Version) is now `1.65.0` ([#2314](https://github.com/emilk/egui/pull/2314)).
* ⚠️ BREAKING: egui now expects integrations to do all color blending in gamma space ([#2071](https://github.com/emilk/egui/pull/2071)).
* ⚠️ BREAKING: if you have overlapping interactive widgets, only the top widget (last added) will be interactive ([#2244](https://github.com/emilk/egui/pull/2244)).

### Added ⭐
* Added helper functions for animating panels that collapse/expand ([#2190](https://github.com/emilk/egui/pull/2190)).
* Added `Context::os/Context::set_os` to query/set what operating system egui believes it is running on ([#2202](https://github.com/emilk/egui/pull/2202)).
* Added `Button::shortcut_text` for showing keyboard shortcuts in menu buttons ([#2202](https://github.com/emilk/egui/pull/2202)).
* Added `egui::KeyboardShortcut` for showing keyboard shortcuts in menu buttons ([#2202](https://github.com/emilk/egui/pull/2202)).
* Texture loading now takes a `TexureOptions` with minification and magnification filters ([#2224](https://github.com/emilk/egui/pull/2224)).
* Added `Key::Minus` and `Key::Equals` ([#2239](https://github.com/emilk/egui/pull/2239)).
* Added `egui::gui_zoom` module with helpers for scaling the whole GUI of an app ([#2239](https://github.com/emilk/egui/pull/2239)).
* You can now put one interactive widget on top of another, and only one will get interaction at a time ([#2244](https://github.com/emilk/egui/pull/2244)).
* Added `spacing.menu_margin` for customizing menu spacing ([#2036](https://github.com/emilk/egui/pull/2036))
* Added possibility to enable text wrap for the selected text of `egui::ComboBox` ([#2272](https://github.com/emilk/egui/pull/2272))
* Added `Area::constrain` and `Window::constrain` which constrains area to the screen bounds ([#2270](https://github.com/emilk/egui/pull/2270)).
* Added `Area::pivot` and `Window::pivot` which controls what part of the window to position ([#2303](https://github.com/emilk/egui/pull/2303)).
* Added support for [thin space](https://en.wikipedia.org/wiki/Thin_space).
* Added optional integration with [AccessKit](https://accesskit.dev/) for implementing platform accessibility APIs ([#2294](https://github.com/emilk/egui/pull/2294)).
* Added `panel_fill`, `window_fill` and `window_stroke` to `Visuals` for your theming pleasure ([#2406](https://github.com/emilk/egui/pull/2406)).
* Plots:
  * Allow linking plot cursors ([#1722](https://github.com/emilk/egui/pull/1722)).
  * Added `Plot::auto_bounds_x/y` and `Plot::reset` ([#2029](https://github.com/emilk/egui/pull/2029)).
  * Added `PlotUi::translate_bounds` ([#2145](https://github.com/emilk/egui/pull/2145)).
  * Added `PlotUi::set_plot_bounds` ([#2320](https://github.com/emilk/egui/pull/2320)).
  * Added `PlotUi::plot_secondary_clicked` ([#2318](https://github.com/emilk/egui/pull/2318)).

### Changed 🔧
* Panels always have a separator line, but no stroke on other sides. Their spacing has also changed slightly ([#2261](https://github.com/emilk/egui/pull/2261)).
* Tooltips are only shown when mouse pointer is still ([#2263](https://github.com/emilk/egui/pull/2263)).
* Make it slightly easier to click buttons ([#2304](https://github.com/emilk/egui/pull/2304)).
* `egui::color` has been renamed `egui::ecolor` ([#2399](https://github.com/emilk/egui/pull/2399)).

### Fixed 🐛
* ⚠️ BREAKING: Fix text being too small ([#2069](https://github.com/emilk/egui/pull/2069)).
* Improve mixed CJK/Latin line-breaking ([#1986](https://github.com/emilk/egui/pull/1986)).
* Improved text rendering ([#2071](https://github.com/emilk/egui/pull/2071)).
* Constrain menu popups to the screen ([#2191](https://github.com/emilk/egui/pull/2191)).
* Less jitter when calling `Context::set_pixels_per_point` ([#2239](https://github.com/emilk/egui/pull/2239)).
* Fixed popups and color edit going outside the screen.
* Fixed keyboard support in `DragValue` ([#2342](https://github.com/emilk/egui/pull/2342)).
* If you nest `ScrollAreas` inside each other, the inner area will now move its scroll bar so it is always visible ([#2371](https://github.com/emilk/egui/pull/2371)).
* Ignore key-repeats for `input.key_pressed` ([#2334](https://github.com/emilk/egui/pull/2334), [#2389](https://github.com/emilk/egui/pull/2389)).
* Fixed issue with calling `set_pixels_per_point` each frame ([#2352](https://github.com/emilk/egui/pull/2352)).
* Fix bug in `ScrollArea::show_rows` ([#2258](https://github.com/emilk/egui/pull/2258)).
* Fix bug in `plot::Line::fill` ([#2275](https://github.com/emilk/egui/pull/2275)).
* Only emit `changed` events in `radio_value` and `selectable_value` if the value actually changed ([#2343](https://github.com/emilk/egui/pull/2343)).
* Fixed sizing bug in `Grid` ([#2384](https://github.com/emilk/egui/pull/2384)).
* `ComboBox::width` now correctly sets the outer width ([#2406](https://github.com/emilk/egui/pull/2406)).


## 0.19.0 - 2022-08-20
### Added ⭐
* Added `*_released` & `*_clicked` methods for `PointerState` ([#1582](https://github.com/emilk/egui/pull/1582)).
* Added `PointerButton::Extra1` and `PointerButton::Extra2` ([#1592](https://github.com/emilk/egui/pull/1592)).
* Added `egui::hex_color!` to create `Color32`'s from hex strings under the `color-hex` feature ([#1596](https://github.com/emilk/egui/pull/1596)).
* Optimized painting of filled circles (e.g. for scatter plots) by 10x or more ([#1616](https://github.com/emilk/egui/pull/1616)).
* Added opt-in feature `deadlock_detection` to detect double-lock of mutexes on the same thread ([#1619](https://github.com/emilk/egui/pull/1619)).
* Added `InputState::stable_dt`: a more stable estimate for the delta-time in reactive mode ([#1625](https://github.com/emilk/egui/pull/1625)).
* You can now specify a texture filter for your textures ([#1636](https://github.com/emilk/egui/pull/1636)).
* Added functions keys in `egui::Key` ([#1665](https://github.com/emilk/egui/pull/1665)).
* Added support for using `PaintCallback` shapes with the WGPU backend ([#1684](https://github.com/emilk/egui/pull/1684)).
* Added `Context::request_repaint_after` ([#1694](https://github.com/emilk/egui/pull/1694)).
* `ctrl-h` now acts like backspace in `TextEdit` ([#1812](https://github.com/emilk/egui/pull/1812)).
* Added `custom_formatter` method for `Slider` and `DragValue` ([#1851](https://github.com/emilk/egui/issues/1851)).
* Added `RawInput::has_focus` which backends can set to indicate whether the UI as a whole has the keyboard focus ([#1859](https://github.com/emilk/egui/pull/1859)).
* Added `PointerState::button_double_clicked()` and `PointerState::button_triple_clicked()` ([#1906](https://github.com/emilk/egui/issues/1906)).
* Added `custom_formatter`, `binary`, `octal`, and `hexadecimal` to `DragValue` and `Slider` ([#1953](https://github.com/emilk/egui/issues/1953))

### Changed 🔧
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* `PaintCallback` shapes now require the whole callback to be put in an `Arc<dyn Any>` with the value being a backend-specific callback type ([#1684](https://github.com/emilk/egui/pull/1684)).
* Replaced `needs_repaint` in `FullOutput` with `repaint_after`. Used to force repaint after the set duration in reactive mode ([#1694](https://github.com/emilk/egui/pull/1694)).
* `Layout::left_to_right` and `Layout::right_to_left` now takes the vertical align as an argument. Previous default was `Align::Center`.
* Improved ergonomics of adding plot items. All plot items that take a series of 2D coordinates can now be created directly from `Vec<[f64; 2]>`. The `Value` and `Values` types were removed in favor of `PlotPoint` and `PlotPoints` respectively ([#1816](https://github.com/emilk/egui/pull/1816)).
* `TextBuffer` no longer needs to implement `AsRef<str>` ([#1824](https://github.com/emilk/egui/pull/1824)).

### Fixed 🐛
* Fixed `Response::changed` for `ui.toggle_value` ([#1573](https://github.com/emilk/egui/pull/1573)).
* Fixed `ImageButton`'s changing background padding on hover ([#1595](https://github.com/emilk/egui/pull/1595)).
* Fixed `Plot` auto-bounds bug ([#1599](https://github.com/emilk/egui/pull/1599)).
* Fixed dead-lock when alt-tabbing while also showing a tooltip ([#1618](https://github.com/emilk/egui/pull/1618)).
* Fixed `ScrollArea` scrolling when editing an unrelated `TextEdit` ([#1779](https://github.com/emilk/egui/pull/1779)).
* Fixed `Slider` not always generating events on change ([#1854](https://github.com/emilk/egui/pull/1854)).
* Fixed jitter of anchored windows for the first frame ([#1856](https://github.com/emilk/egui/pull/1856)).
* Fixed focus behavior when pressing Tab in a UI with no focused widget ([#1861](https://github.com/emilk/egui/pull/1861)).
* Fixed automatic plot bounds ([#1865](https://github.com/emilk/egui/pull/1865)).


## 0.18.1 - 2022-05-01
* Change `Shape::Callback` from `&dyn Any` to `&mut dyn Any` to support more backends.


## 0.18.0 - 2022-04-30

### Added ⭐
* Added `Shape::Callback` for backend-specific painting, [with an example](https://github.com/emilk/egui/tree/master/examples/custom_3d_glow) ([#1351](https://github.com/emilk/egui/pull/1351)).
* Added `Frame::canvas` ([#1362](https://github.com/emilk/egui/pull/1362)).
* `Context::request_repaint` will now wake up UI thread, if integrations has called `Context::set_request_repaint_callback` ([#1366](https://github.com/emilk/egui/pull/1366)).
* Added `Plot::allow_scroll`, `Plot::allow_zoom` no longer affects scrolling ([#1382](https://github.com/emilk/egui/pull/1382)).
* Added `Ui::push_id` to resolve id clashes ([#1374](https://github.com/emilk/egui/pull/1374)).
* Added `ComboBox::icon` ([#1405](https://github.com/emilk/egui/pull/1405)).
* Added `Ui::scroll_with_delta`.
* Added `Frame::outer_margin`.
* Added `Painter::hline` and `Painter::vline`.
* Added `Link` and `ui.link` ([#1506](https://github.com/emilk/egui/pull/1506)).
* Added triple-click support; triple-clicking a TextEdit field will select the whole paragraph ([#1512](https://github.com/emilk/egui/pull/1512)).
* Added `Plot::x_grid_spacer` and `Plot::y_grid_spacer` for custom grid spacing ([#1180](https://github.com/emilk/egui/pull/1180)).
* Added `Ui::spinner()` shortcut method ([#1494](https://github.com/emilk/egui/pull/1494)).
* Added `CursorIcon`s for resizing columns, rows, and the eight cardinal directions.
* Added `Ui::toggle_value`.
* Added ability to add any widgets to the header of a collapsing region ([#1538](https://github.com/emilk/egui/pull/1538)).

### Changed 🔧
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* `ClippedMesh` has been replaced with `ClippedPrimitive` ([#1351](https://github.com/emilk/egui/pull/1351)).
* Renamed `Frame::margin` to `Frame::inner_margin`.
* Renamed `AlphaImage` to `FontImage` to discourage any other use for it ([#1412](https://github.com/emilk/egui/pull/1412)).
* Warnings will be painted on screen when there is an `Id` clash for `Grid`, `Plot` or `ScrollArea` ([#1452](https://github.com/emilk/egui/pull/1452)).
* `Checkbox` and `RadioButton` with an empty label (`""`) will now take up much less space ([#1456](https://github.com/emilk/egui/pull/1456)).
* Replaced `Memory::top_most_layer` with more flexible `Memory::layer_ids`.
* Renamed the feature `convert_bytemuck` to `bytemuck` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Renamed the feature `serialize` to `serde` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Renamed `Painter::sub_region` to `Painter::with_clip_rect`.

### Fixed 🐛
* Fixed `ComboBox`es always being rendered left-aligned ([#1304](https://github.com/emilk/egui/pull/1304)).
* Fixed ui code that could lead to a deadlock ([#1380](https://github.com/emilk/egui/pull/1380)).
* Text is darker and more readable in bright mode ([#1412](https://github.com/emilk/egui/pull/1412)).
* Fixed a lot of broken/missing doclinks  ([#1419](https://github.com/emilk/egui/pull/1419)).
* Fixed `Ui::add_visible` sometimes leaving the `Ui` in a disabled state ([#1436](https://github.com/emilk/egui/issues/1436)).
* Added line breaking rules for Japanese text ([#1498](https://github.com/emilk/egui/pull/1498)).

### Deprecated ☢️
* Deprecated `CollapsingHeader::selectable` ([#1538](https://github.com/emilk/egui/pull/1538)).

### Removed 🔥
* Removed the `single_threaded/multi_threaded` flags - egui is now always thread-safe ([#1390](https://github.com/emilk/egui/pull/1390)).

### Contributors 🙏
* [4JX](https://github.com/4JX)
* [AlexxxRu](https://github.com/AlexxxRu)
* [ascclemens](https://github.com/ascclemens)
* [awaken1ng](https://github.com/awaken1ng)
* [bigfarts](https://github.com/bigfarts)
* [bobyclaws](https://github.com/bobyclaws)
* [Bromeon](https://github.com/Bromeon)
* [cloudhead](https://github.com/cloudhead)
* [collin-kemper](https://github.com/collin-kemper)
* [cpterry](https://github.com/cpterry)
* [dbuch](https://github.com/dbuch)
* [DusterTheFirst](https://github.com/DusterTheFirst)
* [Edgeworth ](https://github.com/Edgeworth )
* [elwerene](https://github.com/elwerene)
* [follower](https://github.com/follower)
* [Friz64](https://github.com/Friz64)
* [Hunter522 ](https://github.com/Hunter522 )
* [Jake-Shadle](https://github.com/Jake-Shadle)
* [jean-airoldie ](https://github.com/jean-airoldie )
* [JelNiSlaw](https://github.com/JelNiSlaw)
* [juancampa](https://github.com/juancampa)
* [LU15W1R7H](https://github.com/LU15W1R7H)
* [mbillingr](https://github.com/mbillingr)
* [nicklasmoeller](https://github.com/nicklasmoeller)
* [rukai](https://github.com/rukai)
* [tami5](https://github.com/tami5)
* [Titaniumtown](https://github.com/Titaniumtown)
* [trevyn](https://github.com/trevyn)
* [waynr](https://github.com/waynr)
* [zam-5 ](https://github.com/zam-5 )


## 0.17.0 - 2022-02-22 - Improved font selection and image handling

### Added ⭐
* Much improved font selection ([#1154](https://github.com/emilk/egui/pull/1154)):
  * You can now select any font size and family using `RichText::size` amd `RichText::family` and the new `FontId`.
  * Easily change text styles with `Style::text_styles`.
  * Added `Ui::text_style_height`.
  * Added `TextStyle::resolve`.
  * Made the v-align and scale of user fonts tweakable ([#1241](https://github.com/emilk/egui/pull/1027)).
* Plot:
  * Added `Plot::x_axis_formatter` and `Plot::y_axis_formatter` for custom axis labels ([#1130](https://github.com/emilk/egui/pull/1130)).
  * Added `Plot::allow_boxed_zoom()`, `Plot::boxed_zoom_pointer()` for boxed zooming on plots ([#1188](https://github.com/emilk/egui/pull/1188)).
  * Added plot pointer coordinates with `Plot::coordinates_formatter` ([#1235](https://github.com/emilk/egui/pull/1235)).
  * Added linked axis support for plots via `plot::LinkedAxisGroup` ([#1184](https://github.com/emilk/egui/pull/1184)).
* `Context::load_texture` to convert an image into a texture which can be displayed using e.g. `ui.image(texture, size)` ([#1110](https://github.com/emilk/egui/pull/1110)).
* `Ui::input_mut` to modify how subsequent widgets see the `InputState` and a convenience method `InputState::consume_key` for shortcuts or hotkeys ([#1212](https://github.com/emilk/egui/pull/1212)).
* Added `Ui::add_visible` and `Ui::add_visible_ui`.
* Added `CollapsingHeader::icon` to override the default open/close icon using a custom function. ([1147](https://github.com/emilk/egui/pull/1147)).
* Added `ui.data()`, `ctx.data()`, `ctx.options()` and `ctx.tessellation_options()` ([#1175](https://github.com/emilk/egui/pull/1175)).
* Added `Response::on_hover_text_at_pointer` as a convenience akin to `Response::on_hover_text` ([1179](https://github.com/emilk/egui/pull/1179)).
* Opt-in dependency on `tracing` crate for logging warnings ([#1192](https://github.com/emilk/egui/pull/1192)).
* Added `ui.weak(text)`.
* Added `Slider::step_by` ([1225](https://github.com/emilk/egui/pull/1225)).
* Added `Context::move_to_top` and `Context::top_most_layer` for managing the layer on the top ([#1242](https://github.com/emilk/egui/pull/1242)).
* Support a subset of macOS' emacs input field keybindings in `TextEdit` ([#1243](https://github.com/emilk/egui/pull/1243)).
* Added ability to scroll an UI into view without specifying an alignment ([1247](https://github.com/emilk/egui/pull/1247)).
* Added `Ui::scroll_to_rect` ([1252](https://github.com/emilk/egui/pull/1252)).

### Changed 🔧
* ⚠️ `Context::input` and `Ui::input` now locks a mutex. This can lead to a dead-lock is used in an `if let` binding!
  * `if let Some(pos) = ui.input().pointer.latest_pos()` and similar must now be rewritten on two lines.
  * Search for this problem in your code using the regex `if let .*input`.
* Better contrast in the default light mode style ([#1238](https://github.com/emilk/egui/pull/1238)).
* Renamed `CtxRef` to `Context` ([#1050](https://github.com/emilk/egui/pull/1050)).
* `Context` can now be cloned and stored between frames ([#1050](https://github.com/emilk/egui/pull/1050)).
* Renamed `Ui::visible` to `Ui::is_visible`.
* Split `Event::Text` into `Event::Text` and `Event::Paste` ([#1058](https://github.com/emilk/egui/pull/1058)).
* Replaced `Style::body_text_style` with more generic `Style::text_styles` ([#1154](https://github.com/emilk/egui/pull/1154)).
* `TextStyle` is no longer `Copy` ([#1154](https://github.com/emilk/egui/pull/1154)).
* Replaced `TextEdit::text_style` with `TextEdit::font` ([#1154](https://github.com/emilk/egui/pull/1154)).
* `Plot::highlight` now takes a `bool` argument ([#1159](https://github.com/emilk/egui/pull/1159)).
* `ScrollArea::show` now returns a `ScrollAreaOutput`, so you might need to add `.inner` after the call to it ([#1166](https://github.com/emilk/egui/pull/1166)).
* Replaced `corner_radius: f32` with `rounding: Rounding`, allowing per-corner rounding settings ([#1206](https://github.com/emilk/egui/pull/1206)).
* Replaced Frame's `margin: Vec2` with `margin: Margin`, allowing for different margins on opposing sides ([#1219](https://github.com/emilk/egui/pull/1219)).
* Renamed `Plot::custom_label_func` to `Plot::label_formatter` ([#1235](https://github.com/emilk/egui/pull/1235)).
* `Areas::layer_id_at` ignores non-interatable layers (i.e. Tooltips) ([#1240](https://github.com/emilk/egui/pull/1240)).
* `ScrollArea`s will not shrink below a certain minimum size, set by `min_scrolled_width/min_scrolled_height` ([1255](https://github.com/emilk/egui/pull/1255)).
* For integrations:
  * `Output` has now been renamed `PlatformOutput` and `Context::run` now returns the new `FullOutput` ([#1292](https://github.com/emilk/egui/pull/1292)).
  * `FontImage` has been replaced by `TexturesDelta` (found in `FullOutput`), describing what textures were loaded and freed each frame ([#1110](https://github.com/emilk/egui/pull/1110)).
  * The painter must support partial texture updates ([#1149](https://github.com/emilk/egui/pull/1149)).
  * Added `RawInput::max_texture_side` which should be filled in with e.g. `GL_MAX_TEXTURE_SIZE` ([#1154](https://github.com/emilk/egui/pull/1154)).

### Fixed 🐛
* Plot `Orientation` was not public, although fields using this type were ([#1130](https://github.com/emilk/egui/pull/1130)).
* Context menus now respects the theme ([#1043](https://github.com/emilk/egui/pull/1043)).
* Calling `Context::set_pixels_per_point` before the first frame will now work.
* Tooltips that don't fit the window don't flicker anymore ([#1240](https://github.com/emilk/egui/pull/1240)).
* Scroll areas now follow text cursor ([#1252](https://github.com/emilk/egui/pull/1252)).
* Slider: correctly respond with drag and focus events when interacting with the value directly ([1270](https://github.com/emilk/egui/pull/1270)).

### Contributors 🙏
* [4JX](https://github.com/4JX)
* [55nknown](https://github.com/55nknown)
* [AlanRace](https://github.com/AlanRace)
* [AlexxxRu](https://github.com/AlexxxRu)
* [awaken1ng](https://github.com/awaken1ng)
* [BctfN0HUK7Yg](https://github.com/BctfN0HUK7Yg)
* [Bromeon](https://github.com/Bromeon)
* [cat-state](https://github.com/cat)
* [danielkeller](https://github.com/danielkeller)
* [dvec](https://github.com/dvec)
* [Friz64](https://github.com/Friz64)
* [Gordon01](https://github.com/Gordon01)
* [HackerFoo](https://github.com/HackerFoo)
* [juancampa](https://github.com/juancampa)
* [justinj](https://github.com/justinj)
* [lampsitter](https://github.com/lampsitter)
* [LordMZTE](https://github.com/LordMZTE)
* [manuel-i](https://github.com/manuel)
* [Mingun](https://github.com/Mingun)
* [niklaskorz](https://github.com/niklaskorz)
* [nongiach](https://github.com/nongiach)
* [parasyte](https://github.com/parasyte)
* [psiphi75](https://github.com/psiphi75)
* [s-nie](https://github.com/s)
* [t18b219k](https://github.com/t18b219k)
* [terhechte](https://github.com/terhechte)
* [xudesheng](https://github.com/xudesheng)
* [yusdacra](https://github.com/yusdacra)


## 0.16.1 - 2021-12-31 - Add back `CtxRef::begin_frame,end_frame`

### Added ⭐
* Added back `CtxRef::begin_frame,end_frame` as an alternative to `CtxRef::run`.


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
* Added horizontal scrolling support to `ScrollArea` and `Window` (opt-in).
* `TextEdit::layouter`: Add custom text layout for e.g. syntax highlighting or WYSIWYG.
* `Fonts::layout_job`: New text layout engine allowing mixing fonts, colors and styles, with underlining and strikethrough.
* Added `ui.add_enabled(bool, widget)` to easily add a possibly disabled widget.
* Added `ui.add_enabled_ui(bool, |ui| …)` to create a possibly disabled UI section.
* Added feature `"serialize"` separatedly from `"persistence"`.
* Added `egui::widgets::global_dark_light_mode_buttons` to easily add buttons for switching the egui theme.
* `TextEdit` can now be used to show text which can be selected and copied, but not edited.
* Added `Memory::caches` for caching things from one frame to the next.

### Changed 🔧
* Change the default monospace font to [Hack](https://github.com/source-foundry/Hack).
* Label text will now be centered, right-aligned and/or justified based on the layout of the `Ui` it is in.
* `Hyperlink` will now word-wrap just like a `Label`.
* All `Ui`s must now have a finite `max_rect`.
  * Deprecated: `max_rect_finite`, `available_size_before_wrap_finite` and `available_rect_before_wrap_finite`.
* `Painter`/`Fonts`: text layout now expect a color when creating a `Galley`. You may override that color with `Painter::galley_with_color`.
* MSRV (Minimum Supported Rust Version) is now `1.54.0`.
* By default, `DragValue`s no longer show a tooltip when hovered. Change with `Style::explanation_tooltips`.
* Smaller and nicer color picker.
* `ScrollArea` will auto-shrink to content size unless told otherwise using `ScollArea::auto_shrink`.
* By default, `Slider`'s `clamp_to_range` is set to true.
* Renamed `TextEdit::enabled` to `TextEdit::interactive`.
* `ui.label` (and friends) now take `impl ToString` as argument instead of `impl Into<Label>`.

### Fixed 🐛
* Fixed wrongly sized multiline `TextEdit` in justified layouts.
* Fixed clip rectangle of windows that don't fit the central area.
* Show tooltips above widgets on touch screens.
* Fixed popups sometimes getting clipped by panels.

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
* Fixed window resize bug introduced in `0.14.1`.


## 0.14.1 - 2021-08-28 - Layout bug fixes

### Added ⭐
* Added `Ui::horizontal_top`.

### Fixed 🐛
* Fixed `set_width/set_min_width/set_height/set_min_height/expand_to_include_x/expand_to_include_y`.
* Make minimum grid column width propagate properly.
* Make sure `TextEdit` contents expand to fill width if applicable.
* `ProgressBar`: add a minimum width and fix for having it in an infinite layout.
* Fixed sometimes not being able to click inside a combo box or popup menu.


## 0.14.0 - 2021-08-24 - Ui panels and bug fixes

### Added ⭐
* Panels can now be added to any `Ui`.
* Plot:
  * [Line styles](https://github.com/emilk/egui/pull/482).
  * Added `show_background` and `show_axes` methods to `Plot`.
* [Progress bar](https://github.com/emilk/egui/pull/519).
* `Grid::num_columns`: allow the last column to take up the rest of the space of the parent `Ui`.
* Added an API for dropping files into egui (see `RawInput`).
* `CollapsingHeader` can now optionally be selectable.

### Changed 🔧
* A single-line `TextEdit` will now clip text that doesn't fit in it, and scroll.
* Return closure return value from `Area::show`, `ComboBox::show_ui`, `ComboBox::combo_box_with_label`, `Window::show`, `popup::*`, `menu::menu`.
* Only move/resize windows with primary mouse button.
* Tooltips are now moved to not cover the widget they are attached to.

### Fixed 🐛
* Fixed custom font definitions getting replaced when `pixels_per_point` is changed.
* Fixed `lost_focus` for `TextEdit`.
* Clicking the edge of a menu button will now properly open the menu.
* Fixed hover detection close to an `Area`.
* Fixed case where `Plot`'s `min_auto_bounds` could be ignored after the first call to `Plot::ui`.
* Fixed slow startup when using large font files.

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
  * Added right and bottom panels (`SidePanel::right` and `Panel::bottom`).
  * Panels can now be resized.
  * Added an option to overwrite frame of a `Panel`.
* [Improve accessibility / screen reader](https://github.com/emilk/egui/pull/412).
* Added `ScrollArea::show_rows` for efficient scrolling of huge UI:s.
* Added `ScrollArea::enable_scrolling` to allow freezing scrolling when editing TextEdit widgets within it
* Added `Ui::set_visible` as a way to hide widgets.
* Added `Style::override_text_style` to easily change the text style of everything in a `Ui` (or globally).
* You can now change `TextStyle` on checkboxes, radio buttons and `SelectableLabel`.
* Added support for [cint](https://crates.io/crates/cint) under `cint` feature.
* Added features `extra_asserts` and `extra_debug_asserts` to enable additional checks.
* `TextEdit` now supports edits on a generic buffer using `TextBuffer`.
* Added `Context::set_debug_on_hover` and `egui::trace!(ui)`

### Changed 🔧
* Minimum Rust version is now 1.51 (used to be 1.52)
* [Tweaked the default visuals style](https://github.com/emilk/egui/pull/450).
* Plot: Renamed `Curve` to `Line`.
* `TopPanel::top` is now `TopBottomPanel::top`.
* `SidePanel::left` no longet takes the default width by argument, but by a builder call.
* `SidePanel::left` is resizable by default.

### Fixed 🐛
* Fixed uneven lettering on non-integral device scales ("extortion lettering").
* Fixed invisible scroll bar when native window is too narrow for egui.


## 0.12.0 - 2021-05-10 - Multitouch, user memory, window pivots, and improved plots

### Added ⭐
* Added anchors to windows and areas so you can put a window in e.g. the top right corner.
* Make labels interactive with `Label::sense(Sense::click())`.
* Added `Response::request_focus` and `Response::surrender_focus`.
* Added `TextEdit::code_editor` (VERY basic).
* [Pan and zoom plots](https://github.com/emilk/egui/pull/317).
* [Add plot legends](https://github.com/emilk/egui/pull/349).
* [Users can now store custom state in `egui::Memory`](https://github.com/emilk/egui/pull/257).
* Added `Response::on_disabled_hover_text` to show tooltip for disabled widgets.
* Zoom input: ctrl-scroll and (on `eframe` web) trackpad-pinch gesture.
* Support for raw [multi touch](https://github.com/emilk/egui/pull/306) events,
  enabling zoom, rotate, and more. Works with `eframe` web on mobile devices,
  and should work with `egui_glium` for certain touch devices/screens.
* Added (optional) compatibility with [mint](https://docs.rs/mint).

### Changed 🔧
* Make `Memory::has_focus` public (again).
* `Plot` must now be given a name that is unique within its scope.
* Tab only selects labels if the `screen_reader` option is turned on.
* Renamed `ui.wrap` to `ui.scope`.

### Fixed 🐛
* Fixed [defocus-bug on touch screens](https://github.com/emilk/egui/issues/288).
* Fixed bug with the layout of wide `DragValue`s.

### Removed 🔥
* Moved experimental markup language to `egui_demo_lib`


## 0.11.0 - 2021-04-05 - Optimization, screen reader & new layout logic

### Added ⭐
* You can now give focus to any clickable widget with tab/shift-tab.
  * Use space or enter to click the selected widget.
  * Use arrow keys to adjust sliders and `DragValue`s.
* egui will now output events when widgets gain keyboard focus.
  * This can be hooked up to a screen reader to aid the visually impaired
* Added the option to restrict the dragging bounds of `Window` and `Area` to a specified area using `drag_bounds(rect)`.
* Added support for small and raised text.
* Added `ui.set_row_height`.
* Added `DebugOptions::show_widgets` to debug layouting by hovering widgets.
* Added `ComboBox` to more easily customize combo boxes.
* Added `Slider::new` and `DragValue::new` to replace old type-specific constructors.
* Added `TextEdit::password` to hide input characters.

### Changed 🔧
* `ui.advance_cursor` is now called `ui.add_space`.
* `kb_focus` is now just called `focus`.

### Fixed 🐛
* Fixed some bugs related to centered layouts.
* Fixed secondary-click to open a menu.
* [Fix panic for zero-range sliders and zero-speed drag values](https://github.com/emilk/egui/pull/216).
* Fixed false id clash error for wrapping text.
* Fixed bug that would close a popup (e.g. the color picker) when clicking inside of it.

### Deprecated ☢️
* Deprectated `combo_box_with_label` in favor of new `ComboBox`.
* Deprectated type-specific constructors for `Slider` and `DragValue` (`Slider::f32`, `DragValue::usize` etc).


## 0.10.0 - 2021-02-28 - Plot and polish

<img src="media/egui-0.10-plot.gif" width="50%">

### Added ⭐
* Added `egui::plot::Plot` to plot some 2D data.
* Added `Ui::hyperlink_to(label, url)`.
* Sliders can now have a value prefix and suffix (e.g. the suffix `"°"` works like a unit).
* `Context::set_pixels_per_point` to control the scale of the UI.
* Added `Response::changed()` to query if e.g. a slider was dragged, text was entered or a checkbox was clicked.
* Added support for all integers in `DragValue` and `Slider` (except 128-bit).

### Changed 🔧
* Improve the positioning of tooltips.
* Only show tooltips if mouse is still.
* `Slider` will now show the value display by default, unless turned off with `.show_value(false)`.
* The `Slider` value is now a `DragValue` which when dragged can pick values outside of the slider range (unless `clamp_to_range` is set).


## 0.9.0 - 2021-02-07 - Light Mode and much more

<img src="media/0.9.0-disabled.gif" width="50%">

### Added ⭐
* Added support for secondary and middle mouse buttons.
* Added `Label` methods for code, strong, strikethrough, underline and italics.
* Added `ui.group(|ui| { … })` to visually group some widgets within a frame.
* Added `Ui` helpers for doing manual layout (`ui.put`, `ui.allocate_ui_at_rect` and more).
* Added `ui.set_enabled(false)` to disable all widgets in a `Ui` (grayed out and non-interactive).
* Added `TextEdit::hint_text` for showing a weak hint text when empty.
* `egui::popup::popup_below_widget`: show a popup area below another widget.
* Added `Slider::clamp_to_range(bool)`: if set, clamp the incoming and outgoing values to the slider range.
* Add: `ui.spacing()`, `ui.spacing_mut()`, `ui.visuals()`, `ui.visuals_mut()`.
* Add: `ctx.set_visuals()`.
* You can now control text wrapping with `Style::wrap`.
* Added `Grid::max_col_width`.

### Changed 🔧
* Text will now wrap at newlines, spaces, dashes, punctuation or in the middle of a words if necessary, in that order of priority.
* Widgets will now always line break at `\n` characters.
* Widgets will now more intelligently choose whether or not to wrap text.
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
* Renamed `PaintCmd` to `Shape`.
* Replace tuple `(Rect, Shape)` with tuple-struct `ClippedShape`.
* Renamed feature `"serde"` to `"persistence"`.
* Break out the modules `math` and `paint` into separate crates `emath` and `epaint`.

### Fixed 🐛
* Fixed a bug that would sometimes trigger a "Mismatching panels" panic in debug builds.
* `Image` and `ImageButton` will no longer stretch to fill a justified layout.


## 0.7.0 - 2021-01-04

### Added ⭐
* Added `ui.scroll_to_cursor` and `response.scroll_to_me` ([#81](https://github.com/emilk/egui/pull/81) by [lucaspoffo](https://github.com/lucaspoffo)).
* Added `window.id(…)` and `area.id(…)` for overriding the default `Id`.

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
* Added `egui::math::Rot2`: rotation helper.
* `Response` now contains the `Id` of the widget it pertains to.
* `ui.allocate_response` that allocates space and checks for interactions.
* Added `response.interact(sense)`, e.g. to check for clicks on labels.

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
* Renamed `Sense::nothing()` to  `Sense::hover()`.
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
* Added `Resize::id_source` and `ScrollArea::id_source` to let the user avoid Id clashes.

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
* Added ability to override text color with `visuals.override_text_color`.
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
* Fixed a bug where some regions would slowly grow for non-integral scales (`pixels_per_point`).


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


## Earlier:

* 2020-08-10: renamed the project to "egui"
* 2020-05-30: first release on crates.io (0.1.0)
* 2020-04-01: serious work starts (pandemic project)
* 2019-03-12: gave a talk about what would later become egui: https://www.youtube.com/watch?v=-pmwLHw5Gbs
* 2018-12-23: [initial commit](https://github.com/emilk/egui/commit/856bbf4dae4a69693a0324da34e8b0dd3754dfdf)
* 2018-11-04: started tinkering on a train
