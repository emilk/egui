# egui changelog
All notable changes to the `egui` crate will be documented in this file.

This is just the changelog for the core `egui` crate. Every crate in this repository has their own changelog:
* [`epaint` changelog](crates/epaint/CHANGELOG.md)
* [`egui-winit` changelog](crates/egui-winit/CHANGELOG.md)
* [`egui-wgpu` changelog](crates/egui-wgpu/CHANGELOG.md)
* [`egui_kittest` changelog](crates/egui_kittest/CHANGELOG.md)
* [`egui_glow` changelog](crates/egui_glow/CHANGELOG.md)
* [`ecolor` changelog](crates/ecolor/CHANGELOG.md)
* [`eframe` changelog](crates/eframe/CHANGELOG.md)

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.33.3 - 2025-12-11
* Treat `.` as a word-splitter in text navigation [#7741](https://github.com/emilk/egui/pull/7741) by [@emilk](https://github.com/emilk)
* Change text color of selected text [#7691](https://github.com/emilk/egui/pull/7691) by [@emilk](https://github.com/emilk)


## 0.33.2 - 2025-11-13
### ⭐ Added
* Add `Plugin::on_widget_under_pointer` to support widget inspector [#7652](https://github.com/emilk/egui/pull/7652) by [@juancampa](https://github.com/juancampa)
* Add `Response::total_drag_delta` and `PointerState::total_drag_delta` [#7708](https://github.com/emilk/egui/pull/7708) by [@emilk](https://github.com/emilk)

### 🔧 Changed
* Improve accessibility and testability of `ComboBox` [#7658](https://github.com/emilk/egui/pull/7658) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🐛 Fixed
* Fix `profiling::scope` compile error when profiling using `tracing` backend [#7646](https://github.com/emilk/egui/pull/7646) by [@PPakalns](https://github.com/PPakalns)
* Fix edge cases in "smart aiming" in sliders [#7680](https://github.com/emilk/egui/pull/7680) by [@emilk](https://github.com/emilk)
* Hide scroll bars when dragging other things [#7689](https://github.com/emilk/egui/pull/7689) by [@emilk](https://github.com/emilk)
* Prevent widgets sometimes appearing to move relative to each other [#7710](https://github.com/emilk/egui/pull/7710) by [@emilk](https://github.com/emilk)
* Fix `ui.response().interact(Sense::click())` being flakey [#7713](https://github.com/emilk/egui/pull/7713) by [@lucasmerlin](https://github.com/lucasmerlin)


## 0.33.0 - 2025-10-09 - `egui::Plugin`, better kerning, kitdiff viewer
Highlights from this release:
- `egui::Plugin` a improved way to create and access egui plugins
- [kitdiff](https://github.com/rerun-io/kitdiff), a viewer for egui_kittest image snapshots (and a general image diff tool)
- better kerning


### Improved kerning
As a step towards using [parley](https://github.com/linebender/parley) for font rendering, @valadaptive has refactored the font loading and rendering code. A result of this (next to the font rendering code being much nicer now) is improved kerning.
Notice how the c moved away from the k:

![Oct-09-2025 16-21-58](https://github.com/user-attachments/assets/d4a17e87-5e98-40db-a85a-fa77fa77aceb)


### `egui::Plugin` trait
We've added a new trait-based plugin api, meant to replace `Context::on_begin_pass` and `Context::on_end_pass`.
This makes it a lot easier to handle state in your plugins. Instead of having to write to egui memory it can live right on your plugin struct.
The trait based api also makes easier to add new hooks that plugins can use. In addition to `on_begin_pass` and `on_end_pass`, the `Plugin` trait now has a `input_hook` and `output_hook` which you can use to inspect / modify the `RawInput` / `FullOutput`.

### kitdiff, a image diff viewer
At rerun we have a ton of snapshots. Some PRs will change most of them (e.g. [the](https://github.com/rerun-io/rerun/pull/11253/files) [one](https://rerun-io.github.io/kitdiff/?url=https://github.com/rerun-io/rerun/pull/11253/files) that updated egui and introduced the kerning improvements, ~500 snapshots changed!).
If you really want to look at every changed snapshot it better be as efficient as possible, and the experience on github, fiddeling with the sliders, is kind of frustrating.
In order to fix this, we've made [kitdiff](https://rerun-io.github.io/kitdiff/).
You can use it locally via
- `kitdiff files .` will search for .new.png and .diff.png files
- `kitdiff git` will compare the current files to the default branch (main/master)
  Or in the browser via
- going to https://rerun-io.github.io/kitdiff/ and pasting a PR or github artifact url
- linking to kitdiff via e.g. a github workflow `https://rerun-io.github.io/kitdiff/?url=<link_to_pr_or_artifact>`

To install kitdiff run `cargo install --git https://github.com/rerun-io/kitdiff`

Here is a video showing the kerning changes in kitdiff ([try it yourself](https://rerun-io.github.io/kitdiff/?url=https://github.com/rerun-io/rerun/pull/11253/files)):

https://github.com/user-attachments/assets/74640af1-09ba-435a-9d0c-2cbeee140c8f

###  Migration guide
- `egui::Mutex` now has a timeout as a simple deadlock detection
    - If you use a `egui::Mutex` in some place where it's held for longer than a single frame, you should switch to the std mutex or parking_lot instead (egui mutexes are wrappers around parking lot)
- `screen_rect` is deprecated
    - In order to support safe areas, egui now has `viewport_rect` and `content_rect`.
    - Update all usages of `screen_rect` to `content_rect`, unless you are sure that you want to draw outside the `safe area` (which would mean your Ui may be covered by notches, system ui, etc.)


### ⭐ Added
* New Plugin trait [#7385](https://github.com/emilk/egui/pull/7385) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `Ui::take_available_space()` helper function, which sets the Ui's minimum size to the available space [#7573](https://github.com/emilk/egui/pull/7573) by [@IsseW](https://github.com/IsseW)
* Add support for the safe area on iOS [#7578](https://github.com/emilk/egui/pull/7578) by [@irh](https://github.com/irh)
* Add `UiBuilder::global_scope` and `UiBuilder::id` [#7372](https://github.com/emilk/egui/pull/7372) by [@Icekey](https://github.com/Icekey)
* Add `emath::fast_midpoint` [#7435](https://github.com/emilk/egui/pull/7435) by [@emilk](https://github.com/emilk)
* Make the `hex_color` macro `const` [#7444](https://github.com/emilk/egui/pull/7444) by [@YgorSouza](https://github.com/YgorSouza)
* Add `SurrenderFocusOn` option [#7471](https://github.com/emilk/egui/pull/7471) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `Memory::move_focus` [#7476](https://github.com/emilk/egui/pull/7476) by [@darkwater](https://github.com/darkwater)
* Support on hover tooltip that is noninteractable even with interactable content [#5543](https://github.com/emilk/egui/pull/5543) by [@PPakalns](https://github.com/PPakalns)
* Add rotation gesture support for trackpad sources [#7453](https://github.com/emilk/egui/pull/7453) by [@thatcomputerguy0101](https://github.com/thatcomputerguy0101)

### 🔧 Changed
* Document platform compatibility on `viewport::WindowLevel` and dependents [#7432](https://github.com/emilk/egui/pull/7432) by [@lkdm](https://github.com/lkdm)
* Deprecated `ImageButton` and removed `WidgetType::ImageButton` [#7483](https://github.com/emilk/egui/pull/7483) by [@Stelios-Kourlis](https://github.com/Stelios-Kourlis)
* More even text kerning [#7431](https://github.com/emilk/egui/pull/7431) by [@valadaptive](https://github.com/valadaptive)
* Increase default text size from 12.5 to 13.0 [#7521](https://github.com/emilk/egui/pull/7521) by [@emilk](https://github.com/emilk)
* Update accesskit to 0.21.0 [#7550](https://github.com/emilk/egui/pull/7550) by [@fundon](https://github.com/fundon)
* Update MSRV from 1.86 to 1.88 [#7579](https://github.com/emilk/egui/pull/7579) by [@Wumpf](https://github.com/Wumpf)
* Group AccessKit nodes by `Ui` [#7386](https://github.com/emilk/egui/pull/7386) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🔥 Removed
* Remove the `deadlock_detection` feature [#7497](https://github.com/emilk/egui/pull/7497) by [@lucasmerlin](https://github.com/lucasmerlin)
* Remove deprecated fields from `PlatformOutput` [#7523](https://github.com/emilk/egui/pull/7523) by [@emilk](https://github.com/emilk)
* Remove `log` feature [#7583](https://github.com/emilk/egui/pull/7583) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Enable `clippy::iter_over_hash_type` lint [#7421](https://github.com/emilk/egui/pull/7421) by [@emilk](https://github.com/emilk)
* Fixes sense issues in TextEdit when vertical alignment is used [#7436](https://github.com/emilk/egui/pull/7436) by [@RndUsr123](https://github.com/RndUsr123)
* Fix stuck menu when submenu vanishes [#7589](https://github.com/emilk/egui/pull/7589) by [@lucasmerlin](https://github.com/lucasmerlin)
* Change Spinner widget to account for width as well as height [#7560](https://github.com/emilk/egui/pull/7560) by [@bryceberger](https://github.com/bryceberger)


## 0.32.3 - 2025-09-12
* Preserve text format in truncated label tooltip [#7514](https://github.com/emilk/egui/pull/7514) [#7535](https://github.com/emilk/egui/pull/7535) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix `TextEdit`'s in RTL layouts [#5547](https://github.com/emilk/egui/pull/5547) by [@zakarumych](https://github.com/zakarumych)


## 0.32.2 - 2025-09-04
* Fix: `SubMenu` should not display when ui is disabled [#7428](https://github.com/emilk/egui/pull/7428) by [@ozwaldorf](https://github.com/ozwaldorf)
* Remove line breaks when pasting into single line TextEdit [#7441](https://github.com/emilk/egui/pull/7441) by [@YgorSouza](https://github.com/YgorSouza)
* Panic mutexes that can't lock for 30 seconds, in debug builds [#7468](https://github.com/emilk/egui/pull/7468) by [@emilk](https://github.com/emilk)
* Add `Ui::place`, to place widgets without changing the cursor [#7359](https://github.com/emilk/egui/pull/7359) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix: prevent calendar popup from closing on dropdown change [#7409](https://github.com/emilk/egui/pull/7409) by [@AStrizh](https://github.com/AStrizh)


## 0.32.1 - 2025-08-15 - Misc bug fixes
### ⭐ Added
* Add `ComboBox::popup_style` [#7360](https://github.com/emilk/egui/pull/7360) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🐛 Fixed
* Fix glyph rendering: clamp coverage to [0, 1] [#7415](https://github.com/emilk/egui/pull/7415) by [@emilk](https://github.com/emilk)
* Fix manual `Popup` not closing [#7383](https://github.com/emilk/egui/pull/7383) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix `WidgetText::Text` ignoring fallback font and overrides [#7361](https://github.com/emilk/egui/pull/7361) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix `override_text_color` priority [#7439](https://github.com/emilk/egui/pull/7439) by [@YgorSouza](https://github.com/YgorSouza)
* Fix debug-panic in ScrollArea if contents fit without scrolling [#7440](https://github.com/emilk/egui/pull/7440) by [@YgorSouza](https://github.com/YgorSouza)


## 0.32.0 - 2025-07-10 - Atoms, popups, and better SVG support
This is a big egui release, with several exciting new features!

* _Atoms_ are new layout primitives in egui, for text and images
* Popups, tooltips and menus have undergone a complete rewrite
* Much improved SVG support
* Crisper graphics (especially text!)

Let's dive in!

### ⚛️ Atoms

`egui::Atom` is the new, indivisible building blocks of egui (hence their name).
An `Atom` is an `enum` that can be either `WidgetText`, `Image`, or `Custom`.

The new `AtomLayout` can be used within widgets to do basic layout.
The initial implementation is as minimal as possible, doing just enough to implement what `Button` could do before.
There is a new `IntoAtoms` trait that works with tuples of `Atom`s. Each atom can be customized with the `AtomExt` trait
which works on everything that implements `Into<Atom>`, so e.g. `RichText` or `Image`.
So to create a `Button` with text and image you can now do:
```rs
let image = include_image!("my_icon.png").atom_size(Vec2::splat(12.0));
ui.button((image, "Click me!"));
```

Anywhere you see `impl IntoAtoms` you can add any number of images and text, in any order.

As of 0.32, we have ported the `Button`, `Checkbox`, `RadioButton` to use atoms
(meaning they support adding Atoms and are built on top of `AtomLayout`).
The `Button` implementation is not only more powerful now, but also much simpler, removing ~130 lines of layout math.

In combination with `ui.read_response`, custom widgets are really simple now, here is a minimal button implementation:

```rs
pub struct ALButton<'a> {
    al: AtomLayout<'a>,
}

impl<'a> ALButton<'a> {
    pub fn new(content: impl IntoAtoms<'a>) -> Self {
        Self {
            al: AtomLayout::new(content.into_atoms()).sense(Sense::click()),
        }
    }
}

impl<'a> Widget for ALButton<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let Self { al } = self;
        let response = ui.ctx().read_response(ui.next_auto_id());

        let visuals = response.map_or(&ui.style().visuals.widgets.inactive, |response| {
            ui.style().interact(&response)
        });

        let al = al.frame(
            Frame::new()
                .inner_margin(ui.style().spacing.button_padding)
                .fill(visuals.bg_fill)
                .stroke(visuals.bg_stroke)
                .corner_radius(visuals.corner_radius),
        );

        al.show(ui).response
    }
}
```

You can even use `Atom::custom` to add custom content to Widgets. Here is a button in a button:

https://github.com/user-attachments/assets/8c649784-dcc5-4979-85f8-e735b9cdd090

```rs
let custom_button_id = Id::new("custom_button");
let response = Button::new((
    Atom::custom(custom_button_id, Vec2::splat(18.0)),
    "Look at my mini button!",
))
.atom_ui(ui);
if let Some(rect) = response.rect(custom_button_id) {
    ui.put(rect, Button::new("🔎").frame_when_inactive(false));
}
```
Currently, you need to use `atom_ui` to get a `AtomResponse` which will have the `Rect` to use, but in the future
this could be streamlined, e.g. by adding a `AtomKind::Callback` or by passing the Rects back with `egui::Response`.

Basing our widgets on `AtomLayout` also allowed us to improve `Response::intrinsic_size`, which will now report the
correct size even if widgets are truncated. `intrinsic_size` is the size that a non-wrapped, non-truncated,
non-justified version of the widget would have, and can be useful in advanced layout
calculations like [egui_flex](https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_flex).

##### Details
* Add `AtomLayout`, abstracting layouting within widgets [#5830](https://github.com/emilk/egui/pull/5830) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `Galley::intrinsic_size` and use it in `AtomLayout` [#7146](https://github.com/emilk/egui/pull/7146) by [@lucasmerlin](https://github.com/lucasmerlin)


### ❕ Improved popups, tooltips, and menus

Introduces a new `egui::Popup` api. Checkout the new demo on https://egui.rs:

https://github.com/user-attachments/assets/74e45243-7d05-4fc3-b446-2387e1412c05

We introduced a new `RectAlign` helper to align a rect relative to an other rect. The `Popup` will by default try to find the best `RectAlign` based on the source widgets position (previously submenus would annoyingly overlap if at the edge of the window):

https://github.com/user-attachments/assets/0c5adb6b-8310-4e0a-b936-646bb4ec02f7

`Tooltip` and `menu` have been rewritten based on the new `Popup` api. They are now compatible with each other, meaning you can just show a `ui.menu_button()` in any `Popup` to get a sub menu. There are now customizable `MenuButton` and `SubMenuButton` structs, to help with customizing your menu buttons. This means menus now also support `PopupCloseBehavior` so you can remove your `close_menu` calls from your click handlers!

The old tooltip and popup apis have been ported to the new api so there should be very little breaking changes. The old menu is still around but deprecated. `ui.menu_button` etc now open the new menu, if you can't update to the new one immediately you can use the old buttons from the deprecated `egui::menu` menu.

We also introduced `ui.close()` which closes the nearest container. So you can now conveniently close `Window`s, `Collapsible`s, `Modal`s and `Popup`s from within. To use this for your own containers, call `UiBuilder::closable` and then check for closing within that ui via `ui.should_close()`.

##### Details
* Add `Popup` and `Tooltip`, unifying the previous behaviours [#5713](https://github.com/emilk/egui/pull/5713) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `Ui::close` and `Response::should_close` [#5729](https://github.com/emilk/egui/pull/5729) by [@lucasmerlin](https://github.com/lucasmerlin)
* ⚠️ Improved menu based on `egui::Popup` [#5716](https://github.com/emilk/egui/pull/5716) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add a toggle for the compact menu style [#5777](https://github.com/emilk/egui/pull/5777) by [@s-nie](https://github.com/s-nie)
* Use the new `Popup` API for the color picker button [#7137](https://github.com/emilk/egui/pull/7137) by [@lucasmerlin](https://github.com/lucasmerlin)
* ⚠️ Close popup if `Memory::keep_popup_open` isn't called [#5814](https://github.com/emilk/egui/pull/5814) by [@juancampa](https://github.com/juancampa)
* Fix tooltips sometimes changing position each frame [#7304](https://github.com/emilk/egui/pull/7304) by [@emilk](https://github.com/emilk)
* Change popup memory to be per-viewport [#6753](https://github.com/emilk/egui/pull/6753) by [@mkalte666](https://github.com/mkalte666)
* Deprecate `Memory::popup` API in favor of new `Popup` API [#7317](https://github.com/emilk/egui/pull/7317) by [@emilk](https://github.com/emilk)


### ▲ Improved SVG support
You can render SVG in egui with

```rs
ui.add(egui::Image::new(egui::include_image!("icon.svg"));
```

(Requires the use of `egui_extras`, with the `svg` feature enabled and a call to [`install_image_loaders`](https://docs.rs/egui_extras/latest/egui_extras/fn.install_image_loaders.html)).

Previously this would sometimes result in a blurry SVG, epecially if the `Image` was set to be dynamically scale based on the size of the `Ui` that contained it. Now SVG:s are always pixel-perfect, for truly scalable graphics.

![svg-scaling](https://github.com/user-attachments/assets/faf63f0c-0ff7-47a0-a4cb-7210efeccb72)

##### Details
* Support text in SVGs [#5979](https://github.com/emilk/egui/pull/5979) by [@cernec1999](https://github.com/cernec1999)
* Fix sometimes blurry SVGs [#7071](https://github.com/emilk/egui/pull/7071) by [@emilk](https://github.com/emilk)
* Fix incorrect color fringe colors on SVG:s [#7069](https://github.com/emilk/egui/pull/7069) by [@emilk](https://github.com/emilk)
* Make `Image::paint_at` pixel-perfect crisp for SVG images [#7078](https://github.com/emilk/egui/pull/7078) by [@emilk](https://github.com/emilk)


### ✨ Crisper graphics
Non-SVG icons are also rendered better, and text sharpness has been improved, especially in light mode.

![image](https://github.com/user-attachments/assets/7f370aaf-886a-423c-8391-c378849b63ca)

##### Details
* Improve text sharpness [#5838](https://github.com/emilk/egui/pull/5838) by [@emilk](https://github.com/emilk)
* Improve text rendering in light mode [#7290](https://github.com/emilk/egui/pull/7290) by [@emilk](https://github.com/emilk)
* Improve texture filtering by doing it in gamma space [#7311](https://github.com/emilk/egui/pull/7311) by [@emilk](https://github.com/emilk)
* Make text underline and strikethrough pixel perfect crisp [#5857](https://github.com/emilk/egui/pull/5857) by [@emilk](https://github.com/emilk)

### Migration guide
We have some silently breaking changes (code compiles fine but behavior changed) that require special care:

#### Menus close on click by default
- previously menus would only close on click outside
- either
    - remove the `ui.close_menu()` calls from button click handlers since they are obsolete
    - if the menu should stay open on clicks, change the `PopupCloseBehavior`:
      ```rs
          // Change this
        ui.menu_button("Text", |ui| { /* Menu Content */ });
          // To this:
        MenuButton::new("Text").config(
            MenuConfig::default().close_behavior(PopupCloseBehavior::CloseOnClickOutside),
        ).ui(ui, |ui| { /* Menu Content */ });
        ```
      You can also change the behavior only for a single SubMenu by using `SubMenuButton`, but by default it should be passed to any submenus when using `MenuButton`.

#### `Memory::is_popup_open` api now requires calls to `Memory::keep_popup_open`
- The popup will immediately close if `keep_popup_open` is not called.
- It's recommended to use the new `Popup` api which handles this for you.
- If you can't switch to the new api for some reason, update the code to call `keep_popup_open`:
  ```rs
      if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        ui.memory_mut(|mem| mem.keep_popup_open(popup_id)); // <- add this line
        let area_response = Area::new(popup_id).show(...)
      }
  ```

### ⭐ Other improvements
* Add `Label::show_tooltip_when_elided` [#5710](https://github.com/emilk/egui/pull/5710) by [@bryceberger](https://github.com/bryceberger)
* Deprecate `Ui::allocate_new_ui` in favor of `Ui::scope_builder` [#5764](https://github.com/emilk/egui/pull/5764) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `expand_bg` to customize size of text background [#5365](https://github.com/emilk/egui/pull/5365) by [@MeGaGiGaGon](https://github.com/MeGaGiGaGon)
* Add assert messages and print bad argument values in asserts [#5216](https://github.com/emilk/egui/pull/5216) by [@bircni](https://github.com/bircni)
* Use `TextBuffer` for `layouter` in `TextEdit` instead of `&str` [#5712](https://github.com/emilk/egui/pull/5712) by [@kernelkind](https://github.com/kernelkind)
* Add a `Slider::update_while_editing(bool)` API [#5978](https://github.com/emilk/egui/pull/5978) by [@mbernat](https://github.com/mbernat)
* Add `Scene::drag_pan_buttons` option. Allows specifying which pointer buttons pan the scene by dragging [#5892](https://github.com/emilk/egui/pull/5892) by [@mitchmindtree](https://github.com/mitchmindtree)
* Add `Scene::sense` to customize how `Scene` responds to user input [#5893](https://github.com/emilk/egui/pull/5893) by [@mitchmindtree](https://github.com/mitchmindtree)
* Rework `TextEdit` arrow navigation to handle Unicode graphemes [#5812](https://github.com/emilk/egui/pull/5812) by [@MStarha](https://github.com/MStarha)
* `ScrollArea` improvements for user configurability [#5443](https://github.com/emilk/egui/pull/5443) by [@MStarha](https://github.com/MStarha)
* Add `Response::clicked_with_open_in_background` [#7093](https://github.com/emilk/egui/pull/7093) by [@emilk](https://github.com/emilk)
* Add `Modifiers::matches_any` [#7123](https://github.com/emilk/egui/pull/7123) by [@emilk](https://github.com/emilk)
* Add `Context::format_modifiers` [#7125](https://github.com/emilk/egui/pull/7125) by [@emilk](https://github.com/emilk)
* Add `OperatingSystem::is_mac` [#7122](https://github.com/emilk/egui/pull/7122) by [@emilk](https://github.com/emilk)
* Support vertical-only scrolling by holding down Alt [#7124](https://github.com/emilk/egui/pull/7124) by [@emilk](https://github.com/emilk)
* Support for back-button on Android [#7073](https://github.com/emilk/egui/pull/7073) by [@ardocrat](https://github.com/ardocrat)
* Select all text in DragValue when gaining focus via keyboard [#7107](https://github.com/emilk/egui/pull/7107) by [@Azkellas](https://github.com/Azkellas)
* Add `Context::current_pass_index` [#7276](https://github.com/emilk/egui/pull/7276) by [@emilk](https://github.com/emilk)
* Add `Context::cumulative_frame_nr` [#7278](https://github.com/emilk/egui/pull/7278) by [@emilk](https://github.com/emilk)
* Add `Visuals::text_edit_bg_color` [#7283](https://github.com/emilk/egui/pull/7283) by [@emilk](https://github.com/emilk)
* Add `Visuals::weak_text_alpha` and `weak_text_color` [#7285](https://github.com/emilk/egui/pull/7285) by [@emilk](https://github.com/emilk)
* Add support for scrolling via accesskit / kittest [#7286](https://github.com/emilk/egui/pull/7286) by [@lucasmerlin](https://github.com/lucasmerlin)
* Update area struct to allow force resizing [#7114](https://github.com/emilk/egui/pull/7114) by [@blackberryfloat](https://github.com/blackberryfloat)
* Add `egui::Sides` `shrink_left` / `shrink_right` [#7295](https://github.com/emilk/egui/pull/7295) by [@lucasmerlin](https://github.com/lucasmerlin)
* Set intrinsic size for Label [#7328](https://github.com/emilk/egui/pull/7328) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🔧 Changed
* Raise MSRV to 1.85 [#6848](https://github.com/emilk/egui/pull/6848) by [@torokati44](https://github.com/torokati44), [#7279](https://github.com/emilk/egui/pull/7279) by [@emilk](https://github.com/emilk)
* Set `hint_text` in `WidgetInfo` [#5724](https://github.com/emilk/egui/pull/5724) by [@bircni](https://github.com/bircni)
* Implement `Default` for `ThemePreference` [#5702](https://github.com/emilk/egui/pull/5702) by [@MichaelGrupp](https://github.com/MichaelGrupp)
* Align `available_rect` docs with the new reality after #4590 [#5701](https://github.com/emilk/egui/pull/5701) by [@podusowski](https://github.com/podusowski)
* Clarify platform-specific details for `Viewport` positioning [#5715](https://github.com/emilk/egui/pull/5715) by [@aspiringLich](https://github.com/aspiringLich)
* Simplify the text cursor API [#5785](https://github.com/emilk/egui/pull/5785) by [@valadaptive](https://github.com/valadaptive)
* Bump accesskit to 0.19 [#7040](https://github.com/emilk/egui/pull/7040) by [@valadaptive](https://github.com/valadaptive)
* Better define the meaning of `SizeHint` [#7079](https://github.com/emilk/egui/pull/7079) by [@emilk](https://github.com/emilk)
* Move all input-related options into `InputOptions` [#7121](https://github.com/emilk/egui/pull/7121) by [@emilk](https://github.com/emilk)
* `Button` inherits the `alt_text` of the `Image` in it, if any [#7136](https://github.com/emilk/egui/pull/7136) by [@emilk](https://github.com/emilk)
* Change API of `Tooltip` slightly [#7151](https://github.com/emilk/egui/pull/7151) by [@emilk](https://github.com/emilk)
* Use Rust edition 2024 [#7280](https://github.com/emilk/egui/pull/7280) by [@emilk](https://github.com/emilk)
* Change `ui.disable()` to modify opacity [#7282](https://github.com/emilk/egui/pull/7282) by [@emilk](https://github.com/emilk)
* Make the font atlas use a color image [#7298](https://github.com/emilk/egui/pull/7298) by [@valadaptive](https://github.com/valadaptive)
* Implement `BitOr` and `BitOrAssign` for `Rect` [#7319](https://github.com/emilk/egui/pull/7319) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🔥 Removed
* Remove things that have been deprecated for over a year [#7099](https://github.com/emilk/egui/pull/7099) by [@emilk](https://github.com/emilk)
* Remove `SelectableLabel` [#7277](https://github.com/emilk/egui/pull/7277) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🐛 Fixed
* `Scene`: make `scene_rect` full size on reset [#5801](https://github.com/emilk/egui/pull/5801) by [@graydenshand](https://github.com/graydenshand)
* `Scene`: `TextEdit` selection when placed in a `Scene` [#5791](https://github.com/emilk/egui/pull/5791) by [@karhu](https://github.com/karhu)
* `Scene`: Set transform layer before calling user content [#5884](https://github.com/emilk/egui/pull/5884) by [@mitchmindtree](https://github.com/mitchmindtree)
* Fix: transform `TextShape` underline width [#5865](https://github.com/emilk/egui/pull/5865) by [@emilk](https://github.com/emilk)
* Fix missing repaint after `consume_key` [#7134](https://github.com/emilk/egui/pull/7134) by [@lucasmerlin](https://github.com/lucasmerlin)
* Update `emoji-icon-font` with fix for fullwidth latin characters [#7067](https://github.com/emilk/egui/pull/7067) by [@emilk](https://github.com/emilk)
* Mark all keys as released if the app loses focus [#5743](https://github.com/emilk/egui/pull/5743) by [@emilk](https://github.com/emilk)
* Fix scroll handle extending outside of `ScrollArea` [#5286](https://github.com/emilk/egui/pull/5286) by [@gilbertoalexsantos](https://github.com/gilbertoalexsantos)
* Fix `Response::clicked_elsewhere` not returning `true` sometimes [#5798](https://github.com/emilk/egui/pull/5798) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix kinetic scrolling on touch devices [#5778](https://github.com/emilk/egui/pull/5778) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix `DragValue` expansion when editing [#5809](https://github.com/emilk/egui/pull/5809) by [@MStarha](https://github.com/MStarha)
* Fix disabled `DragValue` eating focus, causing focus to reset [#5826](https://github.com/emilk/egui/pull/5826) by [@KonaeAkira](https://github.com/KonaeAkira)
* Fix semi-transparent colors appearing too bright [#5824](https://github.com/emilk/egui/pull/5824) by [@emilk](https://github.com/emilk)
* Improve drag-to-select text (add margins) [#5797](https://github.com/emilk/egui/pull/5797) by [@hankjordan](https://github.com/hankjordan)
* Fix bug in pointer movement detection [#5329](https://github.com/emilk/egui/pull/5329) by [@rustbasic](https://github.com/rustbasic)
* Protect against NaN in hit-test code [#6851](https://github.com/emilk/egui/pull/6851) by [@Skgland](https://github.com/Skgland)
* Fix image button panicking with tiny `available_space` [#6900](https://github.com/emilk/egui/pull/6900) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix links and text selection in horizontal_wrapped layout [#6905](https://github.com/emilk/egui/pull/6905) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix `leading_space` sometimes being ignored during paragraph splitting [#7031](https://github.com/emilk/egui/pull/7031) by [@afishhh](https://github.com/afishhh)
* Fix typo in deprecation message for `ComboBox::from_id_source` [#7055](https://github.com/emilk/egui/pull/7055) by [@aelmizeb](https://github.com/aelmizeb)
* Bug fix: make sure `end_pass` is called for all loaders [#7072](https://github.com/emilk/egui/pull/7072) by [@emilk](https://github.com/emilk)
* Report image alt text as text if widget contains no other text [#7142](https://github.com/emilk/egui/pull/7142) by [@lucasmerlin](https://github.com/lucasmerlin)
* Slider: move by at least the next increment when using fixed_decimals [#7066](https://github.com/emilk/egui/pull/7066) by [@0x53A](https://github.com/0x53A)
* Fix crash when using infinite widgets [#7296](https://github.com/emilk/egui/pull/7296) by [@emilk](https://github.com/emilk)
* Fix `debug_assert` triggered by `menu`/`intersect_ray` [#7299](https://github.com/emilk/egui/pull/7299) by [@emilk](https://github.com/emilk)
* Change `Rect::area` to return zero for negative rectangles [#7305](https://github.com/emilk/egui/pull/7305) by [@emilk](https://github.com/emilk)

### 🚀 Performance
* Optimize editing long text by caching each paragraph [#5411](https://github.com/emilk/egui/pull/5411) by [@afishhh](https://github.com/afishhh)
* Make `WidgetText` smaller and faster [#6903](https://github.com/emilk/egui/pull/6903) by [@lucasmerlin](https://github.com/lucasmerlin)


## 0.31.1 - 2025-03-05
* Fix sizing bug in `TextEdit::singleline` [#5640](https://github.com/emilk/egui/pull/5640) by [@IaVashik](https://github.com/IaVashik)
* Fix panic when rendering thin textured rectangles [#5692](https://github.com/emilk/egui/pull/5692) by [@PPakalns](https://github.com/PPakalns)


## 0.31.0 - 2025-02-04 - Scene container, improved rendering quality

### Highlights ✨

#### Scene container
This release adds the `Scene` container to egui. It is a pannable, zoomable canvas that can contain `Widget`s and child `Ui`s.
This will make it easier to e.g. implement a graph editor.

![scene](https://github.com/user-attachments/assets/7dc5e395-a3cb-4bf3-83a3-51a76a48c409)

#### Clearer, pixel perfect rendering
The tessellator has been updated for improved rendering quality and better performance. It will produce fewer vertices
and shapes will have less overdraw. We've also defined what `CornerRadius` (previously `Rounding`) means.

We've also added a tessellator test to the [demo app](https://www.egui.rs/), where you can play around with different
values to see what's produced:


https://github.com/user-attachments/assets/adf55e3b-fb48-4df0-aaa2-150ee3163684


Check the [PR](https://github.com/emilk/egui/pull/5669) for more details.

#### `CornerRadius`, `Margin`, `Shadow` size reduction
In order to pave the path for more complex and customizable styling solutions, we've reduced the size of
`CornerRadius`, `Margin` and `Shadow` values to `i8` and `u8`.



### Migration guide
- Add a `StrokeKind` to all your `Painter::rect` calls [#5648](https://github.com/emilk/egui/pull/5648)
- `StrokeKind::default` was removed, since the 'normal' value depends on the context [#5658](https://github.com/emilk/egui/pull/5658)
  - You probably want to use `StrokeKind::Inside` when drawing rectangles
  - You probably want to use `StrokeKind::Middle` when drawing open paths
- Rename `Rounding` to `CornerRadius` [#5673](https://github.com/emilk/egui/pull/5673)
- `CornerRadius`, `Margin` and `Shadow` have been updated to use `i8` and `u8` [#5563](https://github.com/emilk/egui/pull/5563), [#5567](https://github.com/emilk/egui/pull/5567), [#5568](https://github.com/emilk/egui/pull/5568)
  - Remove the .0 from your values
  - Cast dynamic values with `as i8` / `as u8` or `as _` if you want Rust to infer the type
    - Rust will do a 'saturating' cast, so if your `f32` value is bigger than `127` it will be clamped to `127`
- `RectShape` parameters changed [#5565](https://github.com/emilk/egui/pull/5565)
  - Prefer to use the builder methods to create it instead of initializing it directly
- `Frame` now takes the `Stroke` width into account for its sizing, so check all views of your app to make sure they still look right.
  Read the [PR](https://github.com/emilk/egui/pull/5575) for more info.

### ⭐ Added
* Add `egui::Scene` for panning/zooming a `Ui` [#5505](https://github.com/emilk/egui/pull/5505) by [@grtlr](https://github.com/grtlr)
* Animated WebP support [#5470](https://github.com/emilk/egui/pull/5470) by [@Aely0](https://github.com/Aely0)
* Improve tessellation quality [#5669](https://github.com/emilk/egui/pull/5669) by [@emilk](https://github.com/emilk)
* Add `OutputCommand` for copying text and opening URL:s [#5532](https://github.com/emilk/egui/pull/5532) by [@emilk](https://github.com/emilk)
* Add `Context::copy_image` [#5533](https://github.com/emilk/egui/pull/5533) by [@emilk](https://github.com/emilk)
* Add `WidgetType::Image` and `Image::alt_text` [#5534](https://github.com/emilk/egui/pull/5534) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `epaint::Brush` for controlling `RectShape` texturing [#5565](https://github.com/emilk/egui/pull/5565) by [@emilk](https://github.com/emilk)
* Implement `nohash_hasher::IsEnabled` for `Id` [#5628](https://github.com/emilk/egui/pull/5628) by [@emilk](https://github.com/emilk)
* Add keys for `!`, `{`, `}` [#5548](https://github.com/emilk/egui/pull/5548) by [@Its-Just-Nans](https://github.com/Its-Just-Nans)
* Add `RectShape::stroke_kind ` to control if stroke is inside/outside/centered [#5647](https://github.com/emilk/egui/pull/5647) by [@emilk](https://github.com/emilk)

### 🔧 Changed
* ⚠️ `Frame` now includes stroke width as part of padding [#5575](https://github.com/emilk/egui/pull/5575) by [@emilk](https://github.com/emilk)
* Rename `Rounding` to `CornerRadius` [#5673](https://github.com/emilk/egui/pull/5673) by [@emilk](https://github.com/emilk)
* Require a `StrokeKind` when painting rectangles with strokes [#5648](https://github.com/emilk/egui/pull/5648) by [@emilk](https://github.com/emilk)
* Round widget coordinates to even multiple of 1/32 [#5517](https://github.com/emilk/egui/pull/5517) by [@emilk](https://github.com/emilk)
* Make all lines and rectangles crisp [#5518](https://github.com/emilk/egui/pull/5518) by [@emilk](https://github.com/emilk)
* Tweak window resize handles [#5524](https://github.com/emilk/egui/pull/5524) by [@emilk](https://github.com/emilk)

### 🔥 Removed
* Remove `egui::special_emojis::TWITTER` [#5622](https://github.com/emilk/egui/pull/5622) by [@emilk](https://github.com/emilk)
* Remove `StrokeKind::default` [#5658](https://github.com/emilk/egui/pull/5658) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Use correct minimum version of `profiling` crate [#5494](https://github.com/emilk/egui/pull/5494) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix interactive widgets sometimes being incorrectly marked as hovered [#5523](https://github.com/emilk/egui/pull/5523) by [@emilk](https://github.com/emilk)
* Fix panic due to non-total ordering in `Area::compare_order()` [#5569](https://github.com/emilk/egui/pull/5569) by [@HactarCE](https://github.com/HactarCE)
* Fix hovering through custom menu button [#5555](https://github.com/emilk/egui/pull/5555) by [@M4tthewDE](https://github.com/M4tthewDE)

### 🚀 Performance
* Use `u8` in `CornerRadius`, and introduce `CornerRadiusF32` [#5563](https://github.com/emilk/egui/pull/5563) by [@emilk](https://github.com/emilk)
* Store `Margin` using `i8` to reduce its size [#5567](https://github.com/emilk/egui/pull/5567) by [@emilk](https://github.com/emilk)
* Shrink size of `Shadow` by using `i8/u8` instead of `f32` [#5568](https://github.com/emilk/egui/pull/5568) by [@emilk](https://github.com/emilk)
* Avoid allocations for loader cache lookup [#5584](https://github.com/emilk/egui/pull/5584) by [@mineichen](https://github.com/mineichen)
* Use bitfield instead of bools in `Response` and `Sense` [#5556](https://github.com/emilk/egui/pull/5556) by [@polwel](https://github.com/polwel)


## 0.30.0 - 2024-12-16 - Modals and better layer support

### ✨ Highlights
* Add `Modal`, a popup that blocks input to the rest of the application ([#5358](https://github.com/emilk/egui/pull/5358) by [@lucasmerlin](https://github.com/lucasmerlin))
* Improved support for transform layers ([#5465](https://github.com/emilk/egui/pull/5465), [#5468](https://github.com/emilk/egui/pull/5468), [#5429](https://github.com/emilk/egui/pull/5429))

#### `egui_kittest`
This release welcomes a new crate to the family: [egui_kittest](https://github.com/emilk/egui/tree/main/crates/egui_kittest).
`egui_kittest` is a testing framework for egui, allowing you to test both automation (simulated clicks and other events),
and also do screenshot testing (useful for regression tests).
`egui_kittest` is built using [`kittest`](https://github.com/rerun-io/kittest), which is a general GUI testing framework that aims to work with any Rust GUI (not just egui!).
`kittest` uses the accessibility library [`AccessKit`](https://github.com/AccessKit/accesskit/) for automatation and to query the widget tree.

`kittest` and `egui_kittest` are written by [@lucasmerlin](https://github.com/lucasmerlin).

Here's a quick example of how to use `egui_kittest` to test a checkbox:

```rust
use egui::accesskit::Toggled;
use egui_kittest::{Harness, kittest::Queryable};

fn main() {
    let mut checked = false;
    let app = |ui: &mut egui::Ui| {
        ui.checkbox(&mut checked, "Check me!");
    };

    let mut harness = egui_kittest::Harness::new_ui(app);

    let checkbox = harness.get_by_label("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::False));
    checkbox.click();

    harness.run();

    let checkbox = harness.get_by_label("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::True));

    // You can even render the ui and do image snapshot tests
    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    harness.wgpu_snapshot("readme_example");
}
```

### ⭐ Added
* Add `Modal` and `Memory::set_modal_layer` [#5358](https://github.com/emilk/egui/pull/5358) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `UiBuilder::layer_id` and remove `layer_id` from `Ui::new` [#5195](https://github.com/emilk/egui/pull/5195) by [@emilk](https://github.com/emilk)
* Allow easier setting of background color for `TextEdit` [#5203](https://github.com/emilk/egui/pull/5203) by [@bircni](https://github.com/bircni)
* Set `Response::intrinsic_size` for `TextEdit` [#5266](https://github.com/emilk/egui/pull/5266) by [@lucasmerlin](https://github.com/lucasmerlin)
* Expose center position in `MultiTouchInfo` [#5247](https://github.com/emilk/egui/pull/5247) by [@lucasmerlin](https://github.com/lucasmerlin)
* `Context::add_font` [#5228](https://github.com/emilk/egui/pull/5228) by [@frederik-uni](https://github.com/frederik-uni)
* Impl from `Box<str>` for `WidgetText`, `RichText` [#5309](https://github.com/emilk/egui/pull/5309) by [@dimtpap](https://github.com/dimtpap)
* Add `Window::scroll_bar_visibility` [#5231](https://github.com/emilk/egui/pull/5231) by [@Zeenobit](https://github.com/Zeenobit)
* Add `ComboBox::close_behavior` [#5305](https://github.com/emilk/egui/pull/5305) by [@avalsch](https://github.com/avalsch)
* Add `painter.line()` [#5291](https://github.com/emilk/egui/pull/5291) by [@bircni](https://github.com/bircni)
* Allow attaching custom user data to a screenshot command [#5416](https://github.com/emilk/egui/pull/5416) by [@emilk](https://github.com/emilk)
* Add `Button::image_tint_follows_text_color` [#5430](https://github.com/emilk/egui/pull/5430) by [@emilk](https://github.com/emilk)
* Consume escape keystroke when bailing out from a drag operation [#5433](https://github.com/emilk/egui/pull/5433) by [@abey79](https://github.com/abey79)
* Add `Context::layer_transform_to_global` & `layer_transform_from_global` [#5465](https://github.com/emilk/egui/pull/5465) by [@emilk](https://github.com/emilk)

### 🔧 Changed
* Update MSRV to Rust 1.80 [#5421](https://github.com/emilk/egui/pull/5421), [#5457](https://github.com/emilk/egui/pull/5457) by [@emilk](https://github.com/emilk)
* Expand max font atlas size from 8k to 16k [#5257](https://github.com/emilk/egui/pull/5257) by [@rustbasic](https://github.com/rustbasic)
* Put font data into `Arc` to reduce memory consumption [#5276](https://github.com/emilk/egui/pull/5276) by [@StarStarJ](https://github.com/StarStarJ)
* Move `egui::util::cache` to `egui::cache`; add `FramePublisher` [#5426](https://github.com/emilk/egui/pull/5426) by [@emilk](https://github.com/emilk)
* Remove `Order::PanelResizeLine` [#5455](https://github.com/emilk/egui/pull/5455) by [@emilk](https://github.com/emilk)
* Drag-and-drop: keep cursor set by user, if any [#5467](https://github.com/emilk/egui/pull/5467) by [@abey79](https://github.com/abey79)
* Use `profiling` crate to support more profiler backends [#5150](https://github.com/emilk/egui/pull/5150) by [@teddemunnik](https://github.com/teddemunnik)
* Improve hit-test of thin widgets, and widgets across layers [#5468](https://github.com/emilk/egui/pull/5468) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Update `ScrollArea` drag velocity when drag stopped [#5175](https://github.com/emilk/egui/pull/5175) by [@valadaptive](https://github.com/valadaptive)
* Fix bug causing wrong-fire of `ViewportCommand::Visible` [#5244](https://github.com/emilk/egui/pull/5244) by [@rustbasic](https://github.com/rustbasic)
* Fix: `Ui::new_child` does not consider the `sizing_pass` field of `UiBuilder` [#5262](https://github.com/emilk/egui/pull/5262) by [@zhatuokun](https://github.com/zhatuokun)
* Fix Ctrl+Shift+Z redo shortcut [#5258](https://github.com/emilk/egui/pull/5258) by [@YgorSouza](https://github.com/YgorSouza)
* Fix: `Window::default_pos` does not work [#5315](https://github.com/emilk/egui/pull/5315) by [@rustbasic](https://github.com/rustbasic)
* Fix: `Sides` did not apply the layout position correctly [#5303](https://github.com/emilk/egui/pull/5303) by [@zhatuokun](https://github.com/zhatuokun)
* Respect `Style::override_font_id` in `RichText` [#5310](https://github.com/emilk/egui/pull/5310) by [@MStarha](https://github.com/MStarha)
* Fix disabled widgets "eating" focus [#5370](https://github.com/emilk/egui/pull/5370) by [@lucasmerlin](https://github.com/lucasmerlin)
* Fix cursor clipping in `TextEdit` inside a `ScrollArea` [#3660](https://github.com/emilk/egui/pull/3660) by [@juancampa](https://github.com/juancampa)
* Make text cursor always appear on click  [#5420](https://github.com/emilk/egui/pull/5420) by [@juancampa](https://github.com/juancampa)
* Fix `on_hover_text_at_pointer` for transformed layers [#5429](https://github.com/emilk/egui/pull/5429) by [@emilk](https://github.com/emilk)
* Fix: don't interact with `Area` outside its `constrain_rect` [#5459](https://github.com/emilk/egui/pull/5459) by [@MScottMcBee](https://github.com/MScottMcBee)
* Fix broken images on egui.rs (move from git lfs to normal git) [#5480](https://github.com/emilk/egui/pull/5480) by [@emilk](https://github.com/emilk)
* Fix: `ui.new_child` should now respect `disabled` [#5483](https://github.com/emilk/egui/pull/5483) by [@emilk](https://github.com/emilk)
* Fix zero-width strokes still affecting the feathering color of boxes [#5485](https://github.com/emilk/egui/pull/5485) by [@emilk](https://github.com/emilk)


## 0.29.1 - 2024-10-01 - Bug fixes
* Remove debug-assert triggered by `with_layer_id/dnd_drag_source` [#5191](https://github.com/emilk/egui/pull/5191) by [@emilk](https://github.com/emilk)
* Fix id clash in `Ui::response` [#5192](https://github.com/emilk/egui/pull/5192) by [@emilk](https://github.com/emilk)
* Do not round panel rectangles to pixel grid [#5196](https://github.com/emilk/egui/pull/5196) by [@emilk](https://github.com/emilk)


## 0.29.0 - 2024-09-26 - Multipass, `UiBuilder`, & visual improvements
### ✨ Highlights
This release adds initial support for multi-pass layout, which is a tool to circumvent [a common limitation of immediate mode](https://github.com/emilk/egui#layout).
You can use the new `UiBuilder::sizing_pass` ([#4969](https://github.com/emilk/egui/pull/4969)) to instruct the `Ui` and widgets to shrink to their minimum size, then store that size.
Then call the new `Context::request_discard` ([#5059](https://github.com/emilk/egui/pull/5059)) to discard the visual output and do another _pass_ immediately after the current finishes.
Together, this allows more advanced layouts that is normally not possible in immediate mode.
So far this is only used by `egui::Grid` to hide the "first-frame jitters" that would sometimes happen before, but 3rd party libraries can also use it to do much more advanced things.

There is also a new `UiBuilder` for more flexible construction of `Ui`s ([#4969](https://github.com/emilk/egui/pull/4969)).
By specifying a `sense` for the `Ui` you can make it respond to clicks and drags, reading the result with the new `Ui::response` ([#5054](https://github.com/emilk/egui/pull/5054)).
Among other things, you can use this to create buttons that contain arbitrary widgets.

0.29 also adds improve support for automatic switching between light and dark mode.
You can now set up a custom `Style` for both dark and light mode, and have egui follow the system preference ([#4744](https://github.com/emilk/egui/pull/4744) [#4860](https://github.com/emilk/egui/pull/4860)).

There also has been several small improvements to the look of egui:
* Fix vertical centering of text (e.g. in buttons) ([#5117](https://github.com/emilk/egui/pull/5117))
* Sharper rendering of lines and outlines ([#4943](https://github.com/emilk/egui/pull/4943))
* Nicer looking text selection, especially in light mode ([#5017](https://github.com/emilk/egui/pull/5017))

#### The new text selection
<img width="198" alt="New text selection in light mode" src="https://github.com/user-attachments/assets/bd342946-299c-44ab-bc2d-2aa8ddbca8eb">
<img width="187" alt="New text selection in dark mode" src="https://github.com/user-attachments/assets/352bed32-5150-49b9-a9f9-c7679a0d30b2">


#### What text selection used to look like
<img width="143" alt="Old text selection in light mode" src="https://github.com/user-attachments/assets/f3cbd798-cfed-4ad4-aa3a-d7480efcfa3c">
<img width="143" alt="Old text selection in dark mode" src="https://github.com/user-attachments/assets/9925d18d-da82-4a44-8a98-ea6857ecc14f">

### 🧳 Migration
* `id_source` is now called `id_salt` everywhere ([#5025](https://github.com/emilk/egui/pull/5025))
* `Ui::new` now takes a `UiBuilder` ([#4969](https://github.com/emilk/egui/pull/4969))
* Deprecated (replaced with `UiBuilder`):
	* `ui.add_visible_ui`
	* `ui.allocate_ui_at_rect`
	* `ui.child_ui`
	* `ui.child_ui_with_id_source`
	* `ui.push_stack_info`

### ⭐ Added
* Create a `UiBuilder` for building `Ui`s [#4969](https://github.com/emilk/egui/pull/4969) by [@emilk](https://github.com/emilk)
* Add `egui::Sides` for  adding UI on left and right sides [#5036](https://github.com/emilk/egui/pull/5036) by [@emilk](https://github.com/emilk)
* Make light & dark visuals customizable when following the system theme [#4744](https://github.com/emilk/egui/pull/4744) [#4860](https://github.com/emilk/egui/pull/4860) by [@bash](https://github.com/bash)
* Interactive `Ui`:s: add `UiBuilder::sense` and `Ui::response` [#5054](https://github.com/emilk/egui/pull/5054) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add a menu button with text and image  [#4748](https://github.com/emilk/egui/pull/4748) by [@NicolasBircksZR](https://github.com/NicolasBircksZR)
* Add `Ui::columns_const()` [#4764](https://github.com/emilk/egui/pull/4764) by [@v0x0g](https://github.com/v0x0g)
* Add `Slider::max_decimals_opt` [#4953](https://github.com/emilk/egui/pull/4953) by [@bircni](https://github.com/bircni)
* Add `Label::halign` [#4975](https://github.com/emilk/egui/pull/4975) by [@rustbasic](https://github.com/rustbasic)
* Add `ui.shrink_clip_rect` [#5068](https://github.com/emilk/egui/pull/5068) by [@emilk](https://github.com/emilk)
* Add `ScrollArea::scroll_bar_rect` [#5070](https://github.com/emilk/egui/pull/5070) by [@emilk](https://github.com/emilk)
* Add `Options::input_options` for click-delay etc [#4942](https://github.com/emilk/egui/pull/4942) by [@girtsf](https://github.com/girtsf)
* Add `WidgetType::RadioGroup` [#5081](https://github.com/emilk/egui/pull/5081) by [@bash](https://github.com/bash)
* Add return value to `with_accessibility_parent` [#5083](https://github.com/emilk/egui/pull/5083) by [@bash](https://github.com/bash)
* Add `Ui::with_visual_transform` [#5055](https://github.com/emilk/egui/pull/5055) by [@lucasmerlin](https://github.com/lucasmerlin)
* Make `Slider` and `DragValue` compatible with `NonZeroUsize` etc [#5105](https://github.com/emilk/egui/pull/5105) by [@emilk](https://github.com/emilk)
* Add `Context::request_discard` for multi-pass layouts [#5059](https://github.com/emilk/egui/pull/5059) by [@emilk](https://github.com/emilk)
* Add UI to modify `FontTweak` live [#5125](https://github.com/emilk/egui/pull/5125) by [@emilk](https://github.com/emilk)
* Add `Response::intrinsic_size` to enable better layout in 3rd party crates [#5082](https://github.com/emilk/egui/pull/5082) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add support for mipmap textures [#5146](https://github.com/emilk/egui/pull/5146) by [@nolanderc](https://github.com/nolanderc)
* Add `DebugOptions::show_unaligned` [#5165](https://github.com/emilk/egui/pull/5165) by [@emilk](https://github.com/emilk)
* Add `Slider::clamping` for precise clamp control [#5119](https://github.com/emilk/egui/pull/5119) by [@emilk](https://github.com/emilk)

### 🚀 Performance
* Optimize `Color32::from_rgba_unmultiplied` with LUT [#5088](https://github.com/emilk/egui/pull/5088) by [@YgorSouza](https://github.com/YgorSouza)

### 🔧 Changed
* Rename `id_source` to `id_salt` [#5025](https://github.com/emilk/egui/pull/5025) by [@bircni](https://github.com/bircni)
* Avoid some `Id` clashes by seeding auto-ids with child id [#4840](https://github.com/emilk/egui/pull/4840) by [@ironpeak](https://github.com/ironpeak)
* Nicer looking text selection, especially in light mode [#5017](https://github.com/emilk/egui/pull/5017) by [@emilk](https://github.com/emilk)
* Fix blurry lines by aligning to pixel grid [#4943](https://github.com/emilk/egui/pull/4943) by [@juancampa](https://github.com/juancampa)
* Center-align all text vertically [#5117](https://github.com/emilk/egui/pull/5117) by [@emilk](https://github.com/emilk)
* Clamp margin values in `Margin::ui` [#4873](https://github.com/emilk/egui/pull/4873) by [@rustbasic](https://github.com/rustbasic)
* Make `scroll_to_*` animations configurable [#4305](https://github.com/emilk/egui/pull/4305) by [@lucasmerlin](https://github.com/lucasmerlin)
* Update `Button` to correctly align contained image [#4891](https://github.com/emilk/egui/pull/4891) by [@PrimmR](https://github.com/PrimmR)
* Deprecate `ahash` re-exports [#4979](https://github.com/emilk/egui/pull/4979) by [@oscargus](https://github.com/oscargus)
* Fix: Ensures correct IME behavior when the text input area gains or loses focus [#4896](https://github.com/emilk/egui/pull/4896) by [@rustbasic](https://github.com/rustbasic)
* Enable rustdoc `generate-link-to-definition` feature on docs.rs [#5030](https://github.com/emilk/egui/pull/5030) by [@GuillaumeGomez](https://github.com/GuillaumeGomez)
* Make some `Memory` methods public [#5046](https://github.com/emilk/egui/pull/5046) by [@bircni](https://github.com/bircni)
* Deprecate `ui.set_sizing_pass` [#5074](https://github.com/emilk/egui/pull/5074) by [@emilk](https://github.com/emilk)
* Export module `egui::frame` [#5087](https://github.com/emilk/egui/pull/5087) by [@simgt](https://github.com/simgt)
* Use `log` crate instead of `eprintln` & remove some unwraps [#5010](https://github.com/emilk/egui/pull/5010) by [@bircni](https://github.com/bircni)
* Fix: `Event::Copy` and `Event::Cut` behave as if they select the entire text when there is no selection [#5115](https://github.com/emilk/egui/pull/5115) by [@rustbasic](https://github.com/rustbasic)

### 🐛 Fixed
* Prevent text shrinking in tooltips; round wrap-width to integer [#5161](https://github.com/emilk/egui/pull/5161) by [@emilk](https://github.com/emilk)
* Fix bug causing tooltips with dynamic content to shrink [#5168](https://github.com/emilk/egui/pull/5168) by [@emilk](https://github.com/emilk)
* Remove some debug asserts [#4826](https://github.com/emilk/egui/pull/4826) by [@emilk](https://github.com/emilk)
* Handle the IME event first in `TextEdit` to fix some bugs [#4794](https://github.com/emilk/egui/pull/4794) by [@rustbasic](https://github.com/rustbasic)
* Slider: round to decimals after applying `step_by` [#4822](https://github.com/emilk/egui/pull/4822) by [@AurevoirXavier](https://github.com/AurevoirXavier)
* Fix: hint text follows the alignment set on the `TextEdit` [#4889](https://github.com/emilk/egui/pull/4889) by [@PrimmR](https://github.com/PrimmR)
* Request focus on a `TextEdit` when clicked [#4991](https://github.com/emilk/egui/pull/4991) by [@Zoxc](https://github.com/Zoxc)
* Fix `Id` clash in `Frame` styling widget [#4967](https://github.com/emilk/egui/pull/4967) by [@YgorSouza](https://github.com/YgorSouza)
* Prevent `ScrollArea` contents from exceeding the container size [#5006](https://github.com/emilk/egui/pull/5006) by [@DouglasDwyer](https://github.com/DouglasDwyer)
* Fix bug in size calculation of truncated text [#5076](https://github.com/emilk/egui/pull/5076) by [@emilk](https://github.com/emilk)
* Fix: Make sure `RawInput::take` clears all events, like it says it does [#5104](https://github.com/emilk/egui/pull/5104) by [@emilk](https://github.com/emilk)
* Fix `DragValue` range clamping [#5118](https://github.com/emilk/egui/pull/5118) by [@emilk](https://github.com/emilk)
* Fix: panic when dragging window between monitors of different pixels_per_point [#4868](https://github.com/emilk/egui/pull/4868) by [@rustbasic](https://github.com/rustbasic)


## 0.28.1 - 2024-07-05 - Tooltip tweaks
### ⭐ Added
* Add `Image::uri()` [#4720](https://github.com/emilk/egui/pull/4720) by [@rustbasic](https://github.com/rustbasic)

### 🔧 Changed
* Better documentation for `Event::Zoom` [#4778](https://github.com/emilk/egui/pull/4778) by [@emilk](https://github.com/emilk)
* Hide tooltips when scrolling [#4784](https://github.com/emilk/egui/pull/4784) by [@emilk](https://github.com/emilk)
* Smoother animations [#4787](https://github.com/emilk/egui/pull/4787) by [@emilk](https://github.com/emilk)
* Hide tooltip on click [#4789](https://github.com/emilk/egui/pull/4789) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Fix default height of top/bottom panels [#4779](https://github.com/emilk/egui/pull/4779) by [@emilk](https://github.com/emilk)
* Show the innermost debug rectangle when pressing all modifier keys [#4782](https://github.com/emilk/egui/pull/4782) by [@emilk](https://github.com/emilk)
* Fix occasional flickering of pointer-tooltips [#4788](https://github.com/emilk/egui/pull/4788) by [@emilk](https://github.com/emilk)


## 0.28.0 - 2024-07-03 - Sizing pass, `UiStack` and GIF support
### ✨ Highlights
* Automatic sizing of menus/popups/tooltips with no jittering, using new _sizing pass_ [#4557](https://github.com/emilk/egui/pull/4557), [#4579](https://github.com/emilk/egui/pull/4579) by [@emilk](https://github.com/emilk)
* Support interactive widgets in tooltips [#4596](https://github.com/emilk/egui/pull/4596) by [@emilk](https://github.com/emilk)
* Add a `ui.stack()` with info about all ancestor `Ui`s, with optional tags [#4588](https://github.com/emilk/egui/pull/4588) by [@abey79](https://github.com/abey79), [#4617](https://github.com/emilk/egui/pull/4617) by [@emilk](https://github.com/emilk)
* GIF support [#4620](https://github.com/emilk/egui/pull/4620) by [@JustFrederik](https://github.com/JustFrederik)
* Blinking text cursor in `TextEdit` [#4279](https://github.com/emilk/egui/pull/4279) by [@emilk](https://github.com/emilk)

### 🧳 Migration
* Update MSRV to 1.76 ([#4411](https://github.com/emilk/egui/pull/4411))
* The `wrap/truncate` functions on `Label/Button/ComboBox` no longer take bools as arguments. Use `.wrap_mode(…)` instead for more fine control ([#4556](https://github.com/emilk/egui/pull/4556))
* `Style::wrap` has been deprecated in favor of `Style::wrap_mode` ([#4556](https://github.com/emilk/egui/pull/4556))
* `Ui::new` and `ui.child_ui` now takes a new parameter for the `UiStack` ([#4588](https://github.com/emilk/egui/pull/4588))
* The `extra_asserts` and `extra_debug_asserts` feature flags have been removed ([#4478](https://github.com/emilk/egui/pull/4478))
* Remove `Event::Scroll` and handle it in egui. Use `Event::MouseWheel` instead ([#4524](https://github.com/emilk/egui/pull/4524))
* `Event::Zoom` is no longer emitted on ctrl+scroll. Use `InputState::smooth_scroll_delta` instead ([#4524](https://github.com/emilk/egui/pull/4524))
* `ui.set_enabled` and `set_visible` have  been deprecated ([#4614](https://github.com/emilk/egui/pull/4614))
* `DragValue::clamp_range` renamed to `range` (([#4728](https://github.com/emilk/egui/pull/4728))

### ⭐ Added
* Overload operators for `Rect + Margin`, `Rect - Margin` etc [#4277](https://github.com/emilk/egui/pull/4277) by [@emilk](https://github.com/emilk)
* Add `Window::order` [#4301](https://github.com/emilk/egui/pull/4301) by [@alexparlett](https://github.com/alexparlett)
* Add a way to specify Undoer settings and construct Undoers more easily [#4357](https://github.com/emilk/egui/pull/4357) by [@valadaptive](https://github.com/valadaptive)
* Add xtask crate [#4293](https://github.com/emilk/egui/pull/4293) by [@YgorSouza](https://github.com/YgorSouza)
* Add `ViewportCommand::RequestCut`, `RequestCopy` and `RequestPaste` to trigger clipboard actions [#4035](https://github.com/emilk/egui/pull/4035) by [@bu5hm4nn](https://github.com/bu5hm4nn)
* Added ability to define colors at UV coordinates along a path [#4353](https://github.com/emilk/egui/pull/4353) by [@murl-digital](https://github.com/murl-digital)
* Add a `Display` impl for `Vec2`, `Pos2`, and `Rect` [#4428](https://github.com/emilk/egui/pull/4428) by [@tgross35](https://github.com/tgross35)
* Easing functions [#4630](https://github.com/emilk/egui/pull/4630) by [@emilk](https://github.com/emilk)
* Add `Options::line_scroll_speed` and `scroll_zoom_speed` [#4532](https://github.com/emilk/egui/pull/4532) by [@emilk](https://github.com/emilk)
* Add `TextEdit::hint_text_font` [#4517](https://github.com/emilk/egui/pull/4517) by [@zaaarf](https://github.com/zaaarf)
* Add `Options::reduce_texture_memory` to free up RAM [#4431](https://github.com/emilk/egui/pull/4431) by [@varphone](https://github.com/varphone)
* Add support for text truncation to `egui::Style` [#4556](https://github.com/emilk/egui/pull/4556) by [@abey79](https://github.com/abey79)
* Add `Response::show_tooltip_ui` and `show_tooltip_text` [#4580](https://github.com/emilk/egui/pull/4580) by [@emilk](https://github.com/emilk)
* Add `opacity` and `multiply_opacity` functions to `Ui` and `Painter` [#4586](https://github.com/emilk/egui/pull/4586) by [@emilk](https://github.com/emilk)
* Add `Key::Quote` [#4683](https://github.com/emilk/egui/pull/4683) by [@mkeeter](https://github.com/mkeeter)
* Improve backtraces when hovering widgets with modifiers pressed [#4696](https://github.com/emilk/egui/pull/4696) by [@emilk](https://github.com/emilk)
* Add `PopupCloseBehavior` [#4636](https://github.com/emilk/egui/pull/4636) by [@Umatriz](https://github.com/Umatriz)
* Add basic test for egui accesskit output [#4716](https://github.com/emilk/egui/pull/4716) by [@Wcubed](https://github.com/Wcubed)
* Add `clamp_to_range` option to DragValue, rename `clamp_range` to `range` (deprecating the former) [#4728](https://github.com/emilk/egui/pull/4728) by [@Wumpf](https://github.com/Wumpf)
* Add `Style::number_formatter` as the default used by `DragValue` [#4740](https://github.com/emilk/egui/pull/4740) by [@emilk](https://github.com/emilk)

### 🔧 Changed
* Improve the UI for changing the egui theme [#4257](https://github.com/emilk/egui/pull/4257) by [@emilk](https://github.com/emilk)
* Change the resize cursor when you reach the resize limit [#4275](https://github.com/emilk/egui/pull/4275) by [@emilk](https://github.com/emilk)
* Make `TextEdit` an atomic widget [#4276](https://github.com/emilk/egui/pull/4276) by [@emilk](https://github.com/emilk)
* Rename `fn scroll2` to `fn scroll` [#4282](https://github.com/emilk/egui/pull/4282) by [@emilk](https://github.com/emilk)
* Change `Frame::multiply_with_opacity` to multiply in gamma space [#4283](https://github.com/emilk/egui/pull/4283) by [@emilk](https://github.com/emilk)
* Use parent `Ui`s style for popups [#4325](https://github.com/emilk/egui/pull/4325) by [@alexparlett](https://github.com/alexparlett)
* Take `rounding` into account when using `Slider::trailing_fill` [#4308](https://github.com/emilk/egui/pull/4308) by [@rustbasic](https://github.com/rustbasic)
* Allow users to create viewports larger than monitor on Windows & macOS [#4337](https://github.com/emilk/egui/pull/4337) by [@lopo12123](https://github.com/lopo12123)
* Improve `ViewportBuilder::with_icon()` documentation [#4408](https://github.com/emilk/egui/pull/4408) by [@roccoblues](https://github.com/roccoblues)
* `include_image!` now accepts expressions [#4521](https://github.com/emilk/egui/pull/4521) by [@YgorSouza](https://github.com/YgorSouza)
* Remove scroll latency for smooth trackpads [#4526](https://github.com/emilk/egui/pull/4526) by [@emilk](https://github.com/emilk)
* Smooth out zooming with discreet scroll wheel [#4530](https://github.com/emilk/egui/pull/4530) by [@emilk](https://github.com/emilk)
* Make `TextEdit::return_key` optional [#4543](https://github.com/emilk/egui/pull/4543) by [@doonv](https://github.com/doonv)
* Better spacing and sizes for (menu) buttons [#4558](https://github.com/emilk/egui/pull/4558) by [@emilk](https://github.com/emilk)
* `ComboBox`: fix justified layout of popup if wider than parent button [#4570](https://github.com/emilk/egui/pull/4570) by [@emilk](https://github.com/emilk)
* Make `Area` state public [#4576](https://github.com/emilk/egui/pull/4576) by [@emilk](https://github.com/emilk)
* Don't persist `Area` size [#4749](https://github.com/emilk/egui/pull/4749) by [@emilk](https://github.com/emilk)
* Round text galley sizes to nearest UI point size [#4578](https://github.com/emilk/egui/pull/4578) by [@emilk](https://github.com/emilk)
* Once you have waited for a tooltip to show, show the next one right away [#4585](https://github.com/emilk/egui/pull/4585) by [@emilk](https://github.com/emilk)
* Fade in windows, tooltips, popups, etc [#4587](https://github.com/emilk/egui/pull/4587) by [@emilk](https://github.com/emilk)
* Make `egu::menu` types public [#4544](https://github.com/emilk/egui/pull/4544) by [@sor-ca](https://github.com/sor-ca)
* The default constrain rect for `Area/Window` is now `ctx.screen_rect` [#4590](https://github.com/emilk/egui/pull/4590) by [@emilk](https://github.com/emilk)
* Constrain `Area`s to screen by default [#4591](https://github.com/emilk/egui/pull/4591) by [@emilk](https://github.com/emilk)
* `Grid`: set the `sizing_pass` flag during the initial sizing pass [#4612](https://github.com/emilk/egui/pull/4612) by [@emilk](https://github.com/emilk)
* Remove special case for 0 in DragValue default formatter [#4639](https://github.com/emilk/egui/pull/4639) by [@YgorSouza](https://github.com/YgorSouza)
* Abort drags when pressing escape key [#4678](https://github.com/emilk/egui/pull/4678) by [@emilk](https://github.com/emilk)
* Allow setting a layer as a sublayer of another [#4690](https://github.com/emilk/egui/pull/4690) by [@YgorSouza](https://github.com/YgorSouza)
* Close context menus with Escape [#4711](https://github.com/emilk/egui/pull/4711) by [@emilk](https://github.com/emilk)
* Cancel DragValue edit if Escape is pressed [#4713](https://github.com/emilk/egui/pull/4713) by [@YgorSouza](https://github.com/YgorSouza)
* The default parser for `DragValue` and `Slider` now ignores whitespace [#4739](https://github.com/emilk/egui/pull/4739) by [@emilk](https://github.com/emilk)
* Disabled widgets are now also disabled in the accesskit output [#4750](https://github.com/emilk/egui/pull/4750) by [@Wcubed](https://github.com/Wcubed)
* Make it easier to grab the handle of a floating scroll bar [#4754](https://github.com/emilk/egui/pull/4754) by [@emilk](https://github.com/emilk)
* When debugging widget rects on hover, show width and height [#4762](https://github.com/emilk/egui/pull/4762) by [@emilk](https://github.com/emilk)
* Make sure all tooltips close if you open a menu in the same layer [#4766](https://github.com/emilk/egui/pull/4766) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Fix wrong replacement function in deprecation notice of `drag_released*` [#4314](https://github.com/emilk/egui/pull/4314) by [@sornas](https://github.com/sornas)
* Consider layer transform when positioning text agent [#4319](https://github.com/emilk/egui/pull/4319) by [@juancampa](https://github.com/juancampa)
* Fix incorrect line breaks [#4377](https://github.com/emilk/egui/pull/4377) by [@juancampa](https://github.com/juancampa)
* Fix `hex_color!` macro by re-exporting `color_hex` crate from `ecolor` [#4372](https://github.com/emilk/egui/pull/4372) by [@dataphract](https://github.com/dataphract)
* Change `Ui::allocate_painter` to inherit properties from `Ui` [#4343](https://github.com/emilk/egui/pull/4343) by [@varphone](https://github.com/varphone)
* Fix `Panel` incorrect size [#4351](https://github.com/emilk/egui/pull/4351) by [@zhatuokun](https://github.com/zhatuokun)
* Improve IME support with new `Event::Ime` [#4358](https://github.com/emilk/egui/pull/4358) by [@rustbasic](https://github.com/rustbasic)
* Disable interaction for `ScrollArea` and `Plot` when UI is disabled [#4457](https://github.com/emilk/egui/pull/4457) by [@varphone](https://github.com/varphone)
* Don't panic when replacement glyph is not found [#4542](https://github.com/emilk/egui/pull/4542) by [@RyanBluth](https://github.com/RyanBluth)
* Fix `Ui::scroll_with_delta` only scrolling if the `ScrollArea` is focused [#4303](https://github.com/emilk/egui/pull/4303) by [@lucasmerlin](https://github.com/lucasmerlin)
* Handle tooltips so large that they cover the widget [#4623](https://github.com/emilk/egui/pull/4623) by [@emilk](https://github.com/emilk)
* ScrollArea: Prevent drag interaction outside the area [#4611](https://github.com/emilk/egui/pull/4611) by [@s-nie](https://github.com/s-nie)
* Fix buggy interaction with widgets outside of clip rect [#4675](https://github.com/emilk/egui/pull/4675) by [@emilk](https://github.com/emilk)
* Make sure contents of a panel don't overflow [#4676](https://github.com/emilk/egui/pull/4676) by [@emilk](https://github.com/emilk)
* Fix: `Response::hover_pos` returns incorrect positions with layer transforms [#4679](https://github.com/emilk/egui/pull/4679) by [@Creative0708](https://github.com/Creative0708)
* Fix: Menu popups and tooltips don't respect layer transforms [#4708](https://github.com/emilk/egui/pull/4708) by [@Creative0708](https://github.com/Creative0708)
* Bug fix: report latest area size in `Area::show` response [#4710](https://github.com/emilk/egui/pull/4710) by [@emilk](https://github.com/emilk)
* Ensure `Window` scroll bars are at the window edges [#4733](https://github.com/emilk/egui/pull/4733) by [@emilk](https://github.com/emilk)
* Prevent `TextEdit` widgets from sending fake primary clicks [#4751](https://github.com/emilk/egui/pull/4751) by [@Aliremu](https://github.com/Aliremu)
* Fix text selection when there's multiple viewports [#4760](https://github.com/emilk/egui/pull/4760) by [@emilk](https://github.com/emilk)
* Use correct cursor icons when resizing panels too wide or narrow [#4769](https://github.com/emilk/egui/pull/4769) by [@emilk](https://github.com/emilk)


## 0.27.2 - 2024-04-02
### 🐛 Fixed
* Fix tooltips for non-interactive widgets [#4291](https://github.com/emilk/egui/pull/4291)
* Fix problem clicking the edge of a `TextEdit` [#4272](https://github.com/emilk/egui/pull/4272)
* Fix: `Response::clicked_elsewhere` takes clip rect into account [#4274](https://github.com/emilk/egui/pull/4274)
* Fix incorrect `Response::interact_rect` for `Area/Window` [#4273](https://github.com/emilk/egui/pull/4273)

### ⭐ Added
* Allow disabling animations on a `ScrollArea` [#4309](https://github.com/emilk/egui/pull/4309) (thanks [@lucasmerlin](https://github.com/lucasmerlin)!)


## 0.27.1 - 2024-03-29
### 🐛 Fixed
* Fix visual glitch on the right side of highly rounded rectangles [#4244](https://github.com/emilk/egui/pull/4244)
* Prevent visual glitch when shadow blur width is very high [#4245](https://github.com/emilk/egui/pull/4245)
* Fix `InputState::any_touches` and add `InputState::has_touch_screen` [#4247](https://github.com/emilk/egui/pull/4247)
* Fix `Context::repaint_causes` returning no causes [#4248](https://github.com/emilk/egui/pull/4248)
* Fix touch-and-hold to open context menu [#4249](https://github.com/emilk/egui/pull/4249)
* Hide shortcut text on zoom buttons if `zoom_with_keyboard` is false [#4262](https://github.com/emilk/egui/pull/4262)

### 🔧 Changed
* Don't apply a clip rect to the contents of an `Area` or `Window` [#4258](https://github.com/emilk/egui/pull/4258)


## 0.27.0 - 2024-03-26 - Nicer menus and new hit test logic
The hit test logic (what is the user clicking on?) has been completely rewritten, and should now be much more accurate and helpful.
The hit test and interaction logic is run at the start of the frame, using the widgets rects from the previous frame, but the latest mouse coordinates.
It enabled getting a `Response` for a widget _before_ creating it using `Context::read_response`.
This will in the future unlock more powerful widget styling options.
The new hit test also allows clicking slightly outside a button and still hit it, improving the support for touch screens.

The menus have also been improved so that they both act and feel better, with no change in API.
Included in this is much nicer looking shadows, supporting an offset.

<img width="580" alt="Screenshot 2024-03-26 at 17 00 23" src="https://github.com/emilk/egui/assets/1148717/f1eea39f-17a7-41ca-a983-ee142b04ef26">


### ⚠️ BREAKING
* `Response::clicked*` and `Response::dragged*` may lock the `Context`, so don't call it from a `Context`-locking closure.
* `Response::clicked_by` will no longer be true if clicked with keyboard. Use `Response::clicked` instead.
* `Memory::focus` has been renamed `Memory::focused`
* `Area::new` now takes an `Id` by argument [#4115](https://github.com/emilk/egui/pull/4115)
* Change the definition of `clicked_by` [#4192](https://github.com/emilk/egui/pull/4192)

### ☰ Menu related improvements
* Add some distance between parent menu and submenu [#4230](https://github.com/emilk/egui/pull/4230)
* Add `Area::sense` and improve hit-testing of buttons in menus [#4234](https://github.com/emilk/egui/pull/4234)
* Improve logic for when submenus are kept open [#4166](https://github.com/emilk/egui/pull/4166)
* Better align menus with the button that opened them [#4233](https://github.com/emilk/egui/pull/4233)
* Hide hover UI when showing the context menu [#4138](https://github.com/emilk/egui/pull/4138) (thanks [@abey79](https://github.com/abey79)!)
* CSS-like `Shadow` with offset, spread, and blur [#4232](https://github.com/emilk/egui/pull/4232)
* On touch screens, press-and-hold equals a secondary click [#4195](https://github.com/emilk/egui/pull/4195)

### ⭐ Added
* Add with_taskbar to viewport builder [#3958](https://github.com/emilk/egui/pull/3958) (thanks [@AnotherNathan](https://github.com/AnotherNathan)!)
* Add F21 to F35 key bindings [#4004](https://github.com/emilk/egui/pull/4004) (thanks [@oscargus](https://github.com/oscargus)!)
* Add `Options::debug_paint_interactive_widgets` [#4018](https://github.com/emilk/egui/pull/4018)
* Add `Ui::set_opacity` [#3965](https://github.com/emilk/egui/pull/3965) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Add `Response::paint_debug_info()` to make it easy to visualize a widget's id and state [#4056](https://github.com/emilk/egui/pull/4056) (thanks [@abey79](https://github.com/abey79)!)
* Add layer transforms, interaction in layer [#3906](https://github.com/emilk/egui/pull/3906) (thanks [@Tweoss](https://github.com/Tweoss)!)
* Add `ColorImage::from_gray_iter` [#3536](https://github.com/emilk/egui/pull/3536) (thanks [@wangxiaochuTHU](https://github.com/wangxiaochuTHU)!)
* Add API for raw mouse motion [#4063](https://github.com/emilk/egui/pull/4063) (thanks [@GiantBlargg](https://github.com/GiantBlargg)!)
* Add accessibility to `ProgressBar` and `Spinner` [#4139](https://github.com/emilk/egui/pull/4139) (thanks [@DataTriny](https://github.com/DataTriny)!)
* Add x11 window type settings to viewport builder [#4175](https://github.com/emilk/egui/pull/4175) (thanks [@psethwick](https://github.com/psethwick)!)
* Add an API for customizing the return key in TextEdit [#4085](https://github.com/emilk/egui/pull/4085) (thanks [@lemon-sh](https://github.com/lemon-sh)!)
* Convenience `const fn` for `Margin`, `Rounding` and `Shadow` [#4080](https://github.com/emilk/egui/pull/4080) (thanks [@0Qwel](https://github.com/0Qwel)!)
* Serde feature: add serde derives to input related structs [#4100](https://github.com/emilk/egui/pull/4100) (thanks [@gweisert](https://github.com/gweisert)!)
* Give each menu `Area` an id distinct from the id of what was clicked  [#4114](https://github.com/emilk/egui/pull/4114)
* `epaint`: Added `Shape::{scale,translate}` wrappers [#4090](https://github.com/emilk/egui/pull/4090) (thanks [@varphone](https://github.com/varphone)!)
* A `Window` can now be resizable in only one direction [#4155](https://github.com/emilk/egui/pull/4155)
* Add `EllipseShape` [#4122](https://github.com/emilk/egui/pull/4122) (thanks [@TheTacBanana](https://github.com/TheTacBanana)!)
* Adjustable Slider rail height [#4092](https://github.com/emilk/egui/pull/4092) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Expose state override for `HeaderResponse` [#4200](https://github.com/emilk/egui/pull/4200) (thanks [@Zeenobit](https://github.com/Zeenobit)!)

### 🔧 Changed
* `TextEdit`: Change `margin` property to `egui::Margin` type [#3993](https://github.com/emilk/egui/pull/3993) (thanks [@bu5hm4nn](https://github.com/bu5hm4nn)!)
* New widget interaction logic [#4026](https://github.com/emilk/egui/pull/4026)
* `ui.dnd_drop_zone()` now returns `InnerResponse`. [#4079](https://github.com/emilk/egui/pull/4079) (thanks [@sowbug](https://github.com/sowbug)!)
* Support interacting with the background of a `Ui` [#4074](https://github.com/emilk/egui/pull/4074)
* Quickly animate scroll when calling `ui.scroll_to_cursor` etc  [#4119](https://github.com/emilk/egui/pull/4119)
* Don't clear modifier state on focus change [#4157](https://github.com/emilk/egui/pull/4157) (thanks [@ming08108](https://github.com/ming08108)!)
* Prevent `egui::Window` from becoming larger than viewport [#4199](https://github.com/emilk/egui/pull/4199) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Don't show URLs when hovering hyperlinks [#4218](https://github.com/emilk/egui/pull/4218)

### 🐛 Fixed
* Fix incorrect handling of item spacing in `Window` title bar [#3995](https://github.com/emilk/egui/pull/3995) (thanks [@varphone](https://github.com/varphone)!)
* Make `on_disabled_hover_ui` respect `tooltip_delay` [#4012](https://github.com/emilk/egui/pull/4012) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Fix `TextEdit` being too short whenever there is horizontal margin [#4005](https://github.com/emilk/egui/pull/4005) (thanks [@gweisert](https://github.com/gweisert)!)
* Fix `Response::interact` and `Ui:interact_with_hovered` [#4013](https://github.com/emilk/egui/pull/4013)
* Fix: `Response.interact_pointer_pos` is `Some` on click and drag released [#4014](https://github.com/emilk/egui/pull/4014)
* Fix custom `Window` `Frame`s [#4009](https://github.com/emilk/egui/pull/4009) (thanks [@varphone](https://github.com/varphone)!)
* Fix: images with background color now respects rounding [#4029](https://github.com/emilk/egui/pull/4029) (thanks [@vincent-sparks](https://github.com/vincent-sparks)!)
* Fixed the incorrect display of the `Window` frame with a wide border or large rounding [#4032](https://github.com/emilk/egui/pull/4032) (thanks [@varphone](https://github.com/varphone)!)
* TextEdit: fix crash when hitting SHIFT + TAB around non-ASCII text [#3984](https://github.com/emilk/egui/pull/3984) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Fix two `ScrollArea` bugs: leaking scroll target and broken animation to target offset [#4174](https://github.com/emilk/egui/pull/4174) (thanks [@abey79](https://github.com/abey79)!)
* Fix bug in `Context::parent_viewport_id` [#4190](https://github.com/emilk/egui/pull/4190) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Remove unnecessary allocation in `RepaintCause::new` [#4146](https://github.com/emilk/egui/pull/4146) (thanks [@valsteen](https://github.com/valsteen)!)


## 0.26.2 - 2024-02-14
* Avoid interacting twice when not required [#4041](https://github.com/emilk/egui/pull/4041) (thanks [@abey79](https://github.com/abey79)!)


## 0.26.1 - 2024-02-11
* Fix `Window` title bar incorrect handling spacing [#3995](https://github.com/emilk/egui/pull/3995) (thanks [@varphone](https://github.com/varphone)!)
* Make `on_disabled_hover_ui` respect `tooltip_delay` [#4012](https://github.com/emilk/egui/pull/4012) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Fix `TextEdit` being too short whenever there is horizontal margin [#4005](https://github.com/emilk/egui/pull/4005) (thanks [@gweisert](https://github.com/gweisert)!)
* Fix `Response::interact` and `Ui:interact_with_hovered` [#4013](https://github.com/emilk/egui/pull/4013)
* Fix: `Response.interact_pointer_pos` is `Some` on click and drag released [#4014](https://github.com/emilk/egui/pull/4014)
* Fix custom `Window` `Frame`s [#4009](https://github.com/emilk/egui/pull/4009) (thanks [@varphone](https://github.com/varphone)!)


## 0.26.0 - 2024-02-05 - Text selection in labels

### ⚠️ BREAKING
* Always set `response.hovered` to false when dragging another widget [#3860](https://github.com/emilk/egui/pull/3860)
* `InputState::scroll_delta` has been replaced by `InputState::raw_scroll_delta` and `InputState::smooth_scroll_delta` [#3884](https://github.com/emilk/egui/pull/3884)
* Improve `Response.dragged`, `drag_started` and `clicked` [#3888](https://github.com/emilk/egui/pull/3888)

### ⭐ Added
* Selectable text in Labels [#3814](https://github.com/emilk/egui/pull/3814) [#3870](https://github.com/emilk/egui/pull/3870)
* Add some drag-and-drop-related APIs in `Response` and `Memory` [#3876](https://github.com/emilk/egui/pull/3876) (thanks [@abey79](https://github.com/abey79)!)
* Add drag-and-drop APIs with payloads storage [#3887](https://github.com/emilk/egui/pull/3887)
* `ComboBox`: add builder method for height [#3001](https://github.com/emilk/egui/pull/3001) (thanks [@hinto-janai](https://github.com/hinto-janai)!)
* Add keys `?`, `/`, `|` [#3820](https://github.com/emilk/egui/pull/3820)
* Add `Response::contains_pointer` [#3859](https://github.com/emilk/egui/pull/3859)
* Add `Align2::anchor_size` [#3863](https://github.com/emilk/egui/pull/3863)
* Add `Context::debug_text` [#3864](https://github.com/emilk/egui/pull/3864)
* Allow read access to shapes added to painter this frame [#3866](https://github.com/emilk/egui/pull/3866) (thanks [@brunizzl](https://github.com/brunizzl)!)
* Register callbacks with `Context::on_begin_frame` and `on_end_frame` [#3886](https://github.com/emilk/egui/pull/3886)
* Improve `Frame` API to allow picking color until after adding content [#3889](https://github.com/emilk/egui/pull/3889)
* Add opacity factor to `TextShape` [#3916](https://github.com/emilk/egui/pull/3916) (thanks [@StratusFearMe21](https://github.com/StratusFearMe21)!)
* `Context::repaint_causes`: `file:line` of what caused a repaint [#3949](https://github.com/emilk/egui/pull/3949)
* Add `TextureOptions::wrap_mode` [#3954](https://github.com/emilk/egui/pull/3954) (thanks [@CodedNil](https://github.com/CodedNil)!)
* Add `Spacing::menu_width` [#3973](https://github.com/emilk/egui/pull/3973)

### 🔧 Changed
* Move text selection logic to own module [#3843](https://github.com/emilk/egui/pull/3843)
* Smooth scrolling [#3884](https://github.com/emilk/egui/pull/3884)
* Turn off text wrapping by default in combo-box popups [#3912](https://github.com/emilk/egui/pull/3912)
* `Response.context_menu` now returns the response of the context menu, if open [#3904](https://github.com/emilk/egui/pull/3904) (thanks [@AufarZakiev](https://github.com/AufarZakiev)!)
* Update to puffin 0.19 [#3940](https://github.com/emilk/egui/pull/3940)
* Wait with showing tooltip until mouse has been still for 300ms [#3977](https://github.com/emilk/egui/pull/3977)

### 🐛 Fixed
* Fix: dragging to above/below a `TextEdit` or `Label` will select text to begin/end [#3858](https://github.com/emilk/egui/pull/3858)
* Fix clickable widgets blocking scrolling on touch screens [#3815](https://github.com/emilk/egui/pull/3815) (thanks [@lucasmerlin](https://github.com/lucasmerlin)!)
* Fix `stable_dt` [#3832](https://github.com/emilk/egui/pull/3832)
* Bug Fix : `Response::is_pointer_button_down_on` is now false the frame the button is released [#3833](https://github.com/emilk/egui/pull/3833) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Use runtime knowledge of OS for OS-specific text editing [#3840](https://github.com/emilk/egui/pull/3840)
* Fix calling `request_repaint_after` every frame causing immediate repaint [#3978](https://github.com/emilk/egui/pull/3978)

### 🚀 Performance
* Niche-optimize `Id` so that `Option<Id>` is the same size as `Id` [#3932](https://github.com/emilk/egui/pull/3932)
* Parallel tessellation with opt-in `rayon` feature [#3934](https://github.com/emilk/egui/pull/3934)



## 0.25.0 - 2024-01-08 - Better keyboard input

### ⚠️ BREAKING
* Ignore extra SHIFT and ALT when matching modifiers [#3769](https://github.com/emilk/egui/pull/3769)
* Replace `Key::PlusEquals` with `Key::Plus` and `Key::Equals` [#3769](https://github.com/emilk/egui/pull/3769)
* Removed `WidgetTextGalley`, `WidgetTextJob`, `RichText::into_text_job`, `WidgetText::into_text_job` [#3727](https://github.com/emilk/egui/pull/3727)
* Rename `TextBuffer::replace` to `replace_with` [#3751](https://github.com/emilk/egui/pull/3751)

### ⭐ Added
* Replace a special `Color32::PLACEHOLDER` with widget fallback color [#3727](https://github.com/emilk/egui/pull/3727)
* Add `Key`s for `Cut` `Copy` `Paste` `[` `]` `,` `\` `:` `.` `;` `+` `=`  [#3725](https://github.com/emilk/egui/pull/3725) [#3373](https://github.com/emilk/egui/pull/3373) [#3649](https://github.com/emilk/egui/pull/3649) [#3769](https://github.com/emilk/egui/pull/3769) (thanks [@MarijnS95](https://github.com/MarijnS95) and [@mkrueger](https://github.com/mkrueger)!)
* Add `Key::from_name`, `Key::ALL` [#3649](https://github.com/emilk/egui/pull/3649)
* Add `Event::Key::physical_key` [#3649](https://github.com/emilk/egui/pull/3649)
* Add indeterminate state to checkbox [#3605](https://github.com/emilk/egui/pull/3605) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Add `Color32::from_hex` and `Color32::to_hex` [#3570](https://github.com/emilk/egui/pull/3570) [#3777](https://github.com/emilk/egui/pull/3777) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Add `DragValue`s for RGB(A) in the color picker [#2734](https://github.com/emilk/egui/pull/2734) (thanks [@IVAN-MK7](https://github.com/IVAN-MK7)!)
* Add option to customize progress bar rounding [#2881](https://github.com/emilk/egui/pull/2881) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Add methods to load/store `TextEditState` undoer [#3479](https://github.com/emilk/egui/pull/3479) (thanks [@LoganDark](https://github.com/LoganDark)!)
* `ScrollArea`: Add option to always scroll the only enabled direction [#3710](https://github.com/emilk/egui/pull/3710) (thanks [@untbu](https://github.com/untbu)!)

### 🔧 Changed
* `Grid` now follows `style.visuals.striped` if not explicitly overwritten [#3723](https://github.com/emilk/egui/pull/3723) (thanks [@Wcubed](https://github.com/Wcubed)!)
* Allow arrow keys to move away focus from a Slider [#3641](https://github.com/emilk/egui/pull/3641) (thanks [@fornwall](https://github.com/fornwall)!)
* Keep submenus open until another one is hovered [#3055](https://github.com/emilk/egui/pull/3055) (thanks [@DannyStoll1](https://github.com/DannyStoll1)!)
* Highlight the header of the topmost `Window`, controlled by `Visuals.window_highlight_topmost` [#3515](https://github.com/emilk/egui/pull/3515) (thanks [@GuillaumeSchmid](https://github.com/GuillaumeSchmid)!)

### 🐛 Fixed
* Derive `serde` `Serialize` and `Deserialize` for `KeyboardShortcut` [#3694](https://github.com/emilk/egui/pull/3694) (thanks [@zeozeozeo](https://github.com/zeozeozeo)!)
* Fix `Window` positioning bug when bad `pivot` is stored in app data [#3721](https://github.com/emilk/egui/pull/3721) (thanks [@abey79](https://github.com/abey79)!)
* Impl `Clone` for `Fonts` [#3737](https://github.com/emilk/egui/pull/3737)
* Add missing `ResizeDirection::East` [#3749](https://github.com/emilk/egui/pull/3749) (thanks [@dbuch](https://github.com/dbuch)!)
* Fix: don't open context menu on drag [#3767](https://github.com/emilk/egui/pull/3767)
* Fix IME input of `CompositionEnd` without a `CompositionStart` [#3768](https://github.com/emilk/egui/pull/3768) (thanks [@FrankLeeC](https://github.com/FrankLeeC)!)
* Fix: allow using the full Private Use Area for custom fonts [#3509](https://github.com/emilk/egui/pull/3509) (thanks [@varphone](https://github.com/varphone)!)
* Fix: apply edited `DragValue` when it looses focus [#3776](https://github.com/emilk/egui/pull/3776)
* Fix: Non-resizable `Area`s now ignore mouse input outside their bounds [#3039](https://github.com/emilk/egui/pull/3039) (thanks [@fleabitdev](https://github.com/fleabitdev)!)
* Highlight submenu buttons when hovered and open [#3780](https://github.com/emilk/egui/pull/3780)
* Invalidate font atlas on any change to `pixels_per_point`, not matter how small [#3698](https://github.com/emilk/egui/pull/3698) (thanks [@StarStarJ](https://github.com/StarStarJ)!)
* Fix zoom-in shortcut (`Cmd +`) on non-English keyboards [#3769](https://github.com/emilk/egui/pull/3769)


## 0.24.1 - 2023-11-30 - Bug fixes
* Fix buggy text with multiple viewports on monitors with different scales [#3666](https://github.com/emilk/egui/pull/3666)


## 0.24.0 - 2023-11-23 - Multi-viewport

### ✨ Highlights
You can now spawn multiple native windows on supported backends (e.g. `eframe`), using [the new `viewport` API](https://docs.rs/egui/latest/egui/viewport/index.html) ([#3172](https://github.com/emilk/egui/pull/3172)).

You can easily zoom any egui app using Cmd+Plus, Cmd+Minus or Cmd+0, just like in a browser ([#3608](https://github.com/emilk/egui/pull/3608)).

Scrollbars are now hidden by default until you hover the `ScrollArea` ([#3539](https://github.com/emilk/egui/pull/3539)).

### ⭐ Added
* Multiple viewports/windows [#3172](https://github.com/emilk/egui/pull/3172) (thanks [@konkitoman](https://github.com/konkitoman)!)
* Introduce global `zoom_factor` [#3608](https://github.com/emilk/egui/pull/3608)
* Floating scroll bars [#3539](https://github.com/emilk/egui/pull/3539)
* Add redo support to `Undoer` [#3478](https://github.com/emilk/egui/pull/3478) (thanks [@LoganDark](https://github.com/LoganDark)!)
* Add `egui::Vec2b` [#3543](https://github.com/emilk/egui/pull/3543)
* Add max `Window` size & other size helpers [#3537](https://github.com/emilk/egui/pull/3537) (thanks [@arduano](https://github.com/arduano)!)
* Allow changing shape of slider handle [#3429](https://github.com/emilk/egui/pull/3429) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* `RawInput::viewports` contains a list of all viewports. Access the current one with `ctx.input(|i| i.viewport())`

### 🔧 Changed
* Replace `Id::null()` with `Id::NULL` [#3544](https://github.com/emilk/egui/pull/3544)
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)
* Update puffin to 0.18 [#3600](https://github.com/emilk/egui/pull/3600)

### 🐛 Fixed
* Fix upside down slider in the vertical orientation [#3424](https://github.com/emilk/egui/pull/3424) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Make slider step account for range start [#3488](https://github.com/emilk/egui/pull/3488) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Fix rounding of `ImageButton` [#3531](https://github.com/emilk/egui/pull/3531) (thanks [@chriscate](https://github.com/chriscate)!)
* Fix naming: `constraint_to` -> `constrain_to` [#3438](https://github.com/emilk/egui/pull/3438) (thanks [@rinde](https://github.com/rinde)!)
* Fix Shift+Tab behavior when no widget is focused [#3498](https://github.com/emilk/egui/pull/3498) (thanks [@DataTriny](https://github.com/DataTriny)!)
* Fix scroll not sticking when scrollbar is hidden [#3434](https://github.com/emilk/egui/pull/3434) (thanks [@LoganDark](https://github.com/LoganDark)!)
* Add `#[inline]` to all builder-pattern functions [#3557](https://github.com/emilk/egui/pull/3557)
* Properly reverse bool animation if value changes before it's finished [#3577](https://github.com/emilk/egui/pull/3577) (thanks [@YgorSouza](https://github.com/YgorSouza)!)


### ⚠️ BREAKING
* `egui::gui_zoom::zoom_with_keyboard_shortcuts` is gone, replaced with `Options::zoom_with_keyboard`, which is `true` by default
* `Spacing::scroll_bar_X` has been moved to `Spacing::scroll_bar.X`
* `Context::set_pixels_per_point` now calls `Context::set_zoom_level`, and it may make sense for you to call that directly instead
* If you are using `eframe`, check out the breaking changes in [the `eframe` changelog](crates/eframe/CHANGELOG.md)

#### For integrations
There are several changes relevant to integrations.

* Added `crate::RawInput::viewports` with information about all active viewports
* The repaint callback set by `Context::set_request_repaint_callback` now points to which viewport should be repainted
* `Context::run` now returns a list of `ViewportOutput` in `FullOutput` which should result in their own independent windows
* There is a new `Context::set_immediate_viewport_renderer` for setting up the immediate viewport integration
* If you support viewports, you need to call `Context::set_embed_viewports(false)`, or all new viewports will be embedded (the default behavior)


## 0.23.0 - 2023-09-27 - New image API
This release contains a simple and powerful image API:

```rs
// Load from web:
ui.image("https://www.example.com/some_image.png");

// Include image in the binary using `include_bytes`:
ui.image(egui::include_image!("../assets/ferris.svg"));

// With options:
ui.add(
    egui::Image::new("file://path/to/image.jpg")
        .max_width(200.0)
        .rounding(10.0),
);
```

The API is based on a plugin-system, where you can tell `egui` how to load the images, and from where.

`egui_extras` comes with loaders for you, so all you need to do is add the following to your `Cargo.toml`:

```toml
egui_extras = { version = "0.23", features = ["all_loaders"] }
image = { version = "0.24", features = ["jpeg", "png"] } # Add the types you want support for
```

And this to your code:

```rs
egui_extras::install_image_loaders(egui_ctx);
```

### ⚠️ BREAKING
* Update MSRV to Rust 1.70.0 [#3310](https://github.com/emilk/egui/pull/3310)
* Break out plotting to own crate `egui_plot` [#3282](https://github.com/emilk/egui/pull/3282)

### ⭐ Added
* A new image API [#3297](https://github.com/emilk/egui/pull/3297) [#3315](https://github.com/emilk/egui/pull/3315) [#3328](https://github.com/emilk/egui/pull/3328) [#3338](https://github.com/emilk/egui/pull/3338) [#3342](https://github.com/emilk/egui/pull/3342) [#3343](https://github.com/emilk/egui/pull/3343) [#3402](https://github.com/emilk/egui/pull/3402) (thanks [@jprochazk](https://github.com/jprochazk)!)
* Add option to truncate text at some width [#3244](https://github.com/emilk/egui/pull/3244)
* Add control of line height and letter spacing [#3302](https://github.com/emilk/egui/pull/3302)
* Support images with rounded corners [#3257](https://github.com/emilk/egui/pull/3257)
* Change focused widget with arrow keys [#3272](https://github.com/emilk/egui/pull/3272) (thanks [@TimonPost](https://github.com/TimonPost)!)
* Add opt-in `puffin` feature to egui [#3298](https://github.com/emilk/egui/pull/3298)
* Add debug-option to show a callstack to the widget under the mouse and removed the `trace!` macro as this is more useful [#3391](https://github.com/emilk/egui/pull/3391)
* Add `Context::open_url` and `Context::copy_text` [#3380](https://github.com/emilk/egui/pull/3380)
* Add  `Area::constrain_to` and `Window::constrain_to` [#3396](https://github.com/emilk/egui/pull/3396)
* Add `Memory::area_rect` [#3161](https://github.com/emilk/egui/pull/3161) (thanks [@tosti007](https://github.com/tosti007)!)
* Add `Margin::expand_rect` and `shrink_rect` [#3214](https://github.com/emilk/egui/pull/3214)
* Provide `into_inner()` for `egui::mutex::{Mutex, RwLock}` [#3110](https://github.com/emilk/egui/pull/3110) (thanks [@KmolYuan](https://github.com/KmolYuan)!)
* Support multi-threaded Wasm [#3236](https://github.com/emilk/egui/pull/3236)
* Change touch force to be `Option<f32>` instead of `f32` [#3240](https://github.com/emilk/egui/pull/3240) (thanks [@lucasmerlin](https://github.com/lucasmerlin)!)
* Add option to always open hyperlink in a new browser tab [#3242](https://github.com/emilk/egui/pull/3242) (thanks [@FreddyFunk](https://github.com/FreddyFunk)!)
* Add `Window::drag_to_scroll` [#3118](https://github.com/emilk/egui/pull/3118) (thanks [@KYovchevski](https://github.com/KYovchevski)!)
* Add `CollapsingState::remove` to clear stored state [#3252](https://github.com/emilk/egui/pull/3252) (thanks [@dmackdev](https://github.com/dmackdev)!)
* Add tooltip_delay option [#3245](https://github.com/emilk/egui/pull/3245) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Added `Context::is_context_menu_open()` [#3267](https://github.com/emilk/egui/pull/3267) (thanks [@dmlary](https://github.com/dmlary)!)
* Add `mime` field to `DroppedFile` [#3273](https://github.com/emilk/egui/pull/3273) (thanks [@abey79](https://github.com/abey79)!)
* Allow setting the progress bar height [#3183](https://github.com/emilk/egui/pull/3183) (thanks [@s-nie](https://github.com/s-nie)!)
* Add `scroll_area::State::velocity` [#3300](https://github.com/emilk/egui/pull/3300) (thanks [@Barugon](https://github.com/Barugon)!)
* Add `Visuals::interact_cursor` [#3312](https://github.com/emilk/egui/pull/3312) (thanks [@zkldi](https://github.com/zkldi)!)
* Add method to `RichText` making it easier to construct layout jobs [#3319](https://github.com/emilk/egui/pull/3319) (thanks [@OmegaJak](https://github.com/OmegaJak)!)
* Add `Context::style_mut` [#3359](https://github.com/emilk/egui/pull/3359)
* `std::borrow::Cow<'_, str>` now implements `TextBuffer` [#3164](https://github.com/emilk/egui/pull/3164) (thanks [@burtonageo](https://github.com/burtonageo)!)

### 🔧 Changed
* Separate text cursor from selection visuals [#3181](https://github.com/emilk/egui/pull/3181) (thanks [@lampsitter](https://github.com/lampsitter)!)
* `DragValue`: update value on each key press by default [#2880](https://github.com/emilk/egui/pull/2880) (thanks [@Barugon](https://github.com/Barugon)!)
* Replace uses of `RangeInclusive<f32>` with `emath::Rangef` [#3221](https://github.com/emilk/egui/pull/3221)
* Implement `Send + Sync` for `ColorPickerFn` and `Ui` (#3148) [#3233](https://github.com/emilk/egui/pull/3233) (thanks [@idanarye](https://github.com/idanarye)!)
* Use the minus character instead of "dash" [#3271](https://github.com/emilk/egui/pull/3271)
* Changing `menu_image_button` to use `ImageButton` builder [#3288](https://github.com/emilk/egui/pull/3288) (thanks [@v-kat](https://github.com/v-kat)!)
* Prune old egui memory data when reaching some limit [#3299](https://github.com/emilk/egui/pull/3299)

### 🐛 Fixed
* Fix TextEdit's character limit [#3173](https://github.com/emilk/egui/pull/3173) (thanks [@Serverator](https://github.com/Serverator)!)
* Set the correct unicode character for "ctrl" shortcuts [#3186](https://github.com/emilk/egui/pull/3186) (thanks [@abey79](https://github.com/abey79)!)
* Fix crash in `DragValue` when only setting `min_decimals` [#3231](https://github.com/emilk/egui/pull/3231)
* Fix clipping issued with `ScrollArea` [#2860](https://github.com/emilk/egui/pull/2860) (thanks [@Barugon](https://github.com/Barugon)!)
* Fix moving slider with arrow keys [#3354](https://github.com/emilk/egui/pull/3354)
* Fix problems with tabs in text [#3355](https://github.com/emilk/egui/pull/3355)
* Fix interaction with moved color-picker [#3395](https://github.com/emilk/egui/pull/3395)



## 0.22.0 - 2023-05-23 - A plethora of small improvements
### ⭐ Added
* Scroll bar visibility options [#2729](https://github.com/emilk/egui/pull/2729) (thanks [@IVAN-MK7](https://github.com/IVAN-MK7)!)
* Add `Grid::with_row_color` [#2519](https://github.com/emilk/egui/pull/2519) (thanks [@imgurbot12](https://github.com/imgurbot12)!)
* Add raw mouse wheel event [#2782](https://github.com/emilk/egui/pull/2782) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Improved plot groups and bounds handling [#2410](https://github.com/emilk/egui/pull/2410) (thanks [@s-nie](https://github.com/s-nie)!)
* Return plot transforms [#2935](https://github.com/emilk/egui/pull/2935)
* Add `Pointer::is_decidedly_dragging` and `could_any_button_be_click` [#2979](https://github.com/emilk/egui/pull/2979)
* Plot widget - allow disabling zoom and drag for x and y separately [#2901](https://github.com/emilk/egui/pull/2901) (thanks [@OmegaJak](https://github.com/OmegaJak)!)
* Add character limit to `TextEdit` [#2816](https://github.com/emilk/egui/pull/2816) (thanks [@wzid](https://github.com/wzid)!)
* Add `egui::Modifiers::contains` [#2989](https://github.com/emilk/egui/pull/2989) (thanks [@Wumpf](https://github.com/Wumpf)!)

### 🔧 Changed
* Improve vertical alignment of fonts [#2724](https://github.com/emilk/egui/pull/2724) (thanks [@lictex](https://github.com/lictex)!)
* Transpose the value/satuation panel of the color picker [#2727](https://github.com/emilk/egui/pull/2727) (thanks [@IVAN-MK7](https://github.com/IVAN-MK7)!)
* Replace `ComboBox::show_index` `String` with `Into<TextWidget>` [#2790](https://github.com/emilk/egui/pull/2790) (thanks [@tosti007](https://github.com/tosti007)!)
* Replace `tracing` with `log` [#2928](https://github.com/emilk/egui/pull/2928)
* Only show id clash warnings in debug builds by default [#2930](https://github.com/emilk/egui/pull/2930)
* ⚠️ BREAKING: `Plot::link_axis` and `Plot::link_cursor` now take the name of the group [#2410](https://github.com/emilk/egui/pull/2410)

### 🐛 Fixed
* Clear all keys and modifies on focus change, fixing "stuck keys" [#2933](https://github.com/emilk/egui/pull/2933)
* Fix deadlock when using `show_blocking_widget` [#2753](https://github.com/emilk/egui/pull/2753) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Fix the OS check for windows [#2832](https://github.com/emilk/egui/pull/2832) (thanks [@jleibs](https://github.com/jleibs)!)
* Fix scroll bars not appearing (#2826) [#2827](https://github.com/emilk/egui/pull/2827) (thanks [@lunixbochs](https://github.com/lunixbochs)!)
* Fix UI `data()` read mutability [#2742](https://github.com/emilk/egui/pull/2742) (thanks [@IS2511](https://github.com/IS2511)!)
* Menu State rect now uses menu frame rect instead of contents rect [#2886](https://github.com/emilk/egui/pull/2886) (thanks [@hats-np](https://github.com/hats-np)!)
* Hide `Response::triple_clicked` in docs [#2867](https://github.com/emilk/egui/pull/2867) (thanks [@ccaven](https://github.com/ccaven)!)
* `request_repaint_after` works even when called from background thread [#2939](https://github.com/emilk/egui/pull/2939)
* Show alt key on Mac as `"Option"`, not `"Alt"` [#2981](https://github.com/emilk/egui/pull/2981) (thanks [@Wumpf](https://github.com/Wumpf)!)
* Mention `store` in `TextEditState` doc comment [#2988](https://github.com/emilk/egui/pull/2988) (thanks [@fxdave](https://github.com/fxdave)!)
* Fix typos [#2866](https://github.com/emilk/egui/pull/2866) (thanks [@fezjo](https://github.com/fezjo)!)


### ✨ Examples
* Fix resizable columns option in the table demo [#2780](https://github.com/emilk/egui/pull/2780) (thanks [@Bobo1239](https://github.com/Bobo1239)!)
* Update serial window example [#2756](https://github.com/emilk/egui/pull/2756) (thanks [@c-git](https://github.com/c-git)!)
* Demo app: use `enum` instead of strings for demo-selector anchor [#2781](https://github.com/emilk/egui/pull/2781) (thanks [@XyLyXyRR](https://github.com/XyLyXyRR)!)
* Use `env_logger` in all examples [#2934](https://github.com/emilk/egui/pull/2934)
* Rename `examples/user_attention/README.mg` to `README.md` [#2948](https://github.com/emilk/egui/pull/2948) (thanks [@MAlba124](https://github.com/MAlba124)!)
* egui_demo_app: add some native window info [b5c24d6](https://github.com/emilk/egui/commit/b5c24d6ec83112440f1a807d5ec79241ea8b40fe)



## 0.21.0 - 2023-02-08 - Deadlock fix and style customizability
* ⚠️ BREAKING: `egui::Context` now use closures for locking ([#2625](https://github.com/emilk/egui/pull/2625)):
  * `ctx.input().key_pressed(Key::A)` -> `ctx.input(|i| i.key_pressed(Key::A))`
  * `ui.memory().toggle_popup(popup_id)` -> `ui.memory_mut(|mem| mem.toggle_popup(popup_id))`

### ⭐ Added
* Add `Response::drag_started_by` and `Response::drag_released_by` for convenience, similar to `dragged` and `dragged_by` ([#2507](https://github.com/emilk/egui/pull/2507)).
* Add `PointerState::*_pressed` to check if the given button was pressed in this frame ([#2507](https://github.com/emilk/egui/pull/2507)).
* `Event::Key` now has a `repeat` field that is set to `true` if the event was the result of a key-repeat ([#2435](https://github.com/emilk/egui/pull/2435)).
* Add `Slider::drag_value_speed`, which lets you ask for finer precision when dragging the slider value rather than the actual slider.
* Add `Memory::any_popup_open`, which returns true if any popup is currently open ([#2464](https://github.com/emilk/egui/pull/2464)).
* Add `Plot::clamp_grid` to only show grid where there is data ([#2480](https://github.com/emilk/egui/pull/2480)).
* Add `ScrollArea::drag_to_scroll` if you want to turn off that feature.
* Add `Response::on_hover_and_drag_cursor`.
* Add `Window::default_open` ([#2539](https://github.com/emilk/egui/pull/2539)).
* Add `ProgressBar::fill` if you want to set the fill color manually. ([#2618](https://github.com/emilk/egui/pull/2618)).
* Add `Button::rounding` to enable round buttons ([#2616](https://github.com/emilk/egui/pull/2616)).
* Add `WidgetVisuals::optional_bg_color` - set it to `Color32::TRANSPARENT` to hide button backgrounds ([#2621](https://github.com/emilk/egui/pull/2621)).
* Add `Context::screen_rect` and `Context::set_cursor_icon` ([#2625](https://github.com/emilk/egui/pull/2625)).
* You can turn off the vertical line left of indented regions with `Visuals::indent_has_left_vline` ([#2636](https://github.com/emilk/egui/pull/2636)).
* Add `Response.highlight` to highlight a widget ([#2632](https://github.com/emilk/egui/pull/2632)).
* Add `Separator::grow` and `Separator::shrink` ([#2665](https://github.com/emilk/egui/pull/2665)).
* Add `Slider::trailing_fill` for trailing color behind the circle like a `ProgressBar` ([#2660](https://github.com/emilk/egui/pull/2660)).

### 🔧 Changed
* Improved plot grid appearance ([#2412](https://github.com/emilk/egui/pull/2412)).
* Improved the algorithm for picking the number of decimals to show when hovering values in the `Plot`.
* Default `ComboBox` is now controlled with `Spacing::combo_width` ([#2621](https://github.com/emilk/egui/pull/2621)).
* `DragValue` and `Slider` now use the proportional font ([#2638](https://github.com/emilk/egui/pull/2638)).
* `ScrollArea` is less aggressive about clipping its contents ([#2665](https://github.com/emilk/egui/pull/2665)).
* Updated to be compatible with a major breaking change in AccessKit that drastically reduces memory usage when accessibility is enabled ([#2678](https://github.com/emilk/egui/pull/2678)).
* Improve `DragValue` behavior ([#2649](https://github.com/emilk/egui/pull/2649), [#2650](https://github.com/emilk/egui/pull/2650), [#2688](https://github.com/emilk/egui/pull/2688), [#2638](https://github.com/emilk/egui/pull/2638)).

### 🐛 Fixed
* Trigger `PointerEvent::Released` for drags ([#2507](https://github.com/emilk/egui/pull/2507)).
* Expose `TextEdit`'s multiline flag to AccessKit ([#2448](https://github.com/emilk/egui/pull/2448)).
* Don't render `\r` (Carriage Return) ([#2452](https://github.com/emilk/egui/pull/2452)).
* The `button_padding` style option works closer as expected with image+text buttons now ([#2510](https://github.com/emilk/egui/pull/2510)).
* Menus are now moved to fit on the screen.
* Fix `Window::pivot` causing windows to move around ([#2694](https://github.com/emilk/egui/pull/2694)).


## 0.20.1 - 2022-12-11 - Fix key-repeat
### 🔧 Changed
* `InputState`: all press functions again include key repeats (like in egui 0.19) ([#2429](https://github.com/emilk/egui/pull/2429)).
* Improve the look of thin white lines ([#2437](https://github.com/emilk/egui/pull/2437)).

### 🐛 Fixed
* Fix key-repeats for `TextEdit`, `Slider`s, etc ([#2429](https://github.com/emilk/egui/pull/2429)).


## 0.20.0 - 2022-12-08 - AccessKit, prettier text, overlapping widgets
* MSRV (Minimum Supported Rust Version) is now `1.65.0` ([#2314](https://github.com/emilk/egui/pull/2314)).
* ⚠️ BREAKING: egui now expects integrations to do all color blending in gamma space ([#2071](https://github.com/emilk/egui/pull/2071)).
* ⚠️ BREAKING: if you have overlapping interactive widgets, only the top widget (last added) will be interactive ([#2244](https://github.com/emilk/egui/pull/2244)).

### ⭐ Added
* Added helper functions for animating panels that collapse/expand ([#2190](https://github.com/emilk/egui/pull/2190)).
* Added `Context::os/Context::set_os` to query/set what operating system egui believes it is running on ([#2202](https://github.com/emilk/egui/pull/2202)).
* Added `Button::shortcut_text` for showing keyboard shortcuts in menu buttons ([#2202](https://github.com/emilk/egui/pull/2202)).
* Added `egui::KeyboardShortcut` for showing keyboard shortcuts in menu buttons ([#2202](https://github.com/emilk/egui/pull/2202)).
* Texture loading now takes a `TextureOptions` with minification and magnification filters ([#2224](https://github.com/emilk/egui/pull/2224)).
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

### 🔧 Changed
* Panels always have a separator line, but no stroke on other sides. Their spacing has also changed slightly ([#2261](https://github.com/emilk/egui/pull/2261)).
* Tooltips are only shown when mouse pointer is still ([#2263](https://github.com/emilk/egui/pull/2263)).
* Make it slightly easier to click buttons ([#2304](https://github.com/emilk/egui/pull/2304)).
* `egui::color` has been renamed `egui::ecolor` ([#2399](https://github.com/emilk/egui/pull/2399)).

### 🐛 Fixed
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
### ⭐ Added
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

### 🔧 Changed
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* `PaintCallback` shapes now require the whole callback to be put in an `Arc<dyn Any>` with the value being a backend-specific callback type ([#1684](https://github.com/emilk/egui/pull/1684)).
* Replaced `needs_repaint` in `FullOutput` with `repaint_after`. Used to force repaint after the set duration in reactive mode ([#1694](https://github.com/emilk/egui/pull/1694)).
* `Layout::left_to_right` and `Layout::right_to_left` now takes the vertical align as an argument. Previous default was `Align::Center`.
* Improved ergonomics of adding plot items. All plot items that take a series of 2D coordinates can now be created directly from `Vec<[f64; 2]>`. The `Value` and `Values` types were removed in favor of `PlotPoint` and `PlotPoints` respectively ([#1816](https://github.com/emilk/egui/pull/1816)).
* `TextBuffer` no longer needs to implement `AsRef<str>` ([#1824](https://github.com/emilk/egui/pull/1824)).

### 🐛 Fixed
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

### ⭐ Added
* Added `Shape::Callback` for backend-specific painting, [with an example](https://github.com/emilk/egui/tree/main/examples/custom_3d_glow) ([#1351](https://github.com/emilk/egui/pull/1351)).
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

### 🔧 Changed
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

### 🐛 Fixed
* Fixed `ComboBox`es always being rendered left-aligned ([#1304](https://github.com/emilk/egui/pull/1304)).
* Fixed ui code that could lead to a deadlock ([#1380](https://github.com/emilk/egui/pull/1380)).
* Text is darker and more readable in bright mode ([#1412](https://github.com/emilk/egui/pull/1412)).
* Fixed a lot of broken/missing doclinks  ([#1419](https://github.com/emilk/egui/pull/1419)).
* Fixed `Ui::add_visible` sometimes leaving the `Ui` in a disabled state ([#1436](https://github.com/emilk/egui/issues/1436)).
* Added line breaking rules for Japanese text ([#1498](https://github.com/emilk/egui/pull/1498)).

### ☢️ Deprecated
* Deprecated `CollapsingHeader::selectable` ([#1538](https://github.com/emilk/egui/pull/1538)).

### 🔥 Removed
* Removed the `single_threaded/multi_threaded` flags - egui is now always thread-safe ([#1390](https://github.com/emilk/egui/pull/1390)).

### Contributors 🙏
* [4JX](https://github.com/4JX)
* [a-liashenko](https://github.com/a-liashenko)
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

### ⭐ Added
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
* Added ability to scroll a UI into view without specifying an alignment ([1247](https://github.com/emilk/egui/pull/1247)).
* Added `Ui::scroll_to_rect` ([1252](https://github.com/emilk/egui/pull/1252)).

### 🔧 Changed
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

### 🐛 Fixed
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
* [a-liashenko](https://github.com/a-liashenko)
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

### ⭐ Added
* Added back `CtxRef::begin_frame,end_frame` as an alternative to `CtxRef::run`.


## 0.16.0 - 2021-12-29 - Context menus and rich text

### ⭐ Added
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

### 🔧 Changed
* MSRV (Minimum Supported Rust Version) is now `1.56.0`.
* `ui.add(Button::new("…").text_color(…))` is now `ui.button(RichText::new("…").color(…))` (same for `Label` )([#855](https://github.com/emilk/egui/pull/855)).
* Plots now provide a `show` method that has to be used to add items to and show the plot ([#766](https://github.com/emilk/egui/pull/766)).
* `menu::menu(ui, ...)` is now `ui.menu_button(...)` ([#543](https://github.com/emilk/egui/pull/543))
* Replaced `CtxRef::begin_frame` and `end_frame` with `CtxRef::run` ([#872](https://github.com/emilk/egui/pull/872)).
* Replaced `scroll_delta` and `zoom_delta` in `RawInput` with `Event::Scroll` and `Event::Zoom`.
* Unified the four `Memory` data buckets (`data`, `data_temp`, `id_data` and `id_data_temp`) into a single `Memory::data`, with a new interface ([#836](https://github.com/emilk/egui/pull/836)).
* Replaced `Ui::__test` with `egui::__run_test_ui` ([#872](https://github.com/emilk/egui/pull/872)).

### 🐛 Fixed
* Fixed `ComboBox` and other popups getting clipped to parent window ([#885](https://github.com/emilk/egui/pull/885)).
* The color picker is now better at keeping the same hue even when saturation goes to zero ([#886](https://github.com/emilk/egui/pull/886)).

### 🔥 Removed
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

### ⭐ Added
* Added horizontal scrolling support to `ScrollArea` and `Window` (opt-in).
* `TextEdit::layouter`: Add custom text layout for e.g. syntax highlighting or WYSIWYG.
* `Fonts::layout_job`: New text layout engine allowing mixing fonts, colors and styles, with underlining and strikethrough.
* Added `ui.add_enabled(bool, widget)` to easily add a possibly disabled widget.
* Added `ui.add_enabled_ui(bool, |ui| …)` to create a possibly disabled UI section.
* Added feature `"serialize"` separately from `"persistence"`.
* Added `egui::widgets::global_dark_light_mode_buttons` to easily add buttons for switching the egui theme.
* `TextEdit` can now be used to show text which can be selected and copied, but not edited.
* Added `Memory::caches` for caching things from one frame to the next.

### 🔧 Changed
* Change the default monospace font to [Hack](https://github.com/source-foundry/Hack).
* Label text will now be centered, right-aligned and/or justified based on the layout of the `Ui` it is in.
* `Hyperlink` will now word-wrap just like a `Label`.
* All `Ui`s must now have a finite `max_rect`.
  * Deprecated: `max_rect_finite`, `available_size_before_wrap_finite` and `available_rect_before_wrap_finite`.
* `Painter`/`Fonts`: text layout now expect a color when creating a `Galley`. You may override that color with `Painter::galley_with_color`.
* MSRV (Minimum Supported Rust Version) is now `1.54.0`.
* By default, `DragValue`s no longer show a tooltip when hovered. Change with `Style::explanation_tooltips`.
* Smaller and nicer color picker.
* `ScrollArea` will auto-shrink to content size unless told otherwise using `ScrollArea::auto_shrink`.
* By default, `Slider`'s `clamp_to_range` is set to true.
* Renamed `TextEdit::enabled` to `TextEdit::interactive`.
* `ui.label` (and friends) now take `impl ToString` as argument instead of `impl Into<Label>`.

### 🐛 Fixed
* Fixed wrongly sized multiline `TextEdit` in justified layouts.
* Fixed clip rectangle of windows that don't fit the central area.
* Show tooltips above widgets on touch screens.
* Fixed popups sometimes getting clipped by panels.

### 🔥 Removed
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

### 🐛 Fixed
* Fixed window resize bug introduced in `0.14.1`.


## 0.14.1 - 2021-08-28 - Layout bug fixes

### ⭐ Added
* Added `Ui::horizontal_top`.

### 🐛 Fixed
* Fixed `set_width/set_min_width/set_height/set_min_height/expand_to_include_x/expand_to_include_y`.
* Make minimum grid column width propagate properly.
* Make sure `TextEdit` contents expand to fill width if applicable.
* `ProgressBar`: add a minimum width and fix for having it in an infinite layout.
* Fixed sometimes not being able to click inside a combo box or popup menu.


## 0.14.0 - 2021-08-24 - Ui panels and bug fixes

### ⭐ Added
* Panels can now be added to any `Ui`.
* Plot:
  * [Line styles](https://github.com/emilk/egui/pull/482).
  * Added `show_background` and `show_axes` methods to `Plot`.
* [Progress bar](https://github.com/emilk/egui/pull/519).
* `Grid::num_columns`: allow the last column to take up the rest of the space of the parent `Ui`.
* Added an API for dropping files into egui (see `RawInput`).
* `CollapsingHeader` can now optionally be selectable.

### 🔧 Changed
* A single-line `TextEdit` will now clip text that doesn't fit in it, and scroll.
* Return closure return value from `Area::show`, `ComboBox::show_ui`, `ComboBox::combo_box_with_label`, `Window::show`, `popup::*`, `menu::menu`.
* Only move/resize windows with primary mouse button.
* Tooltips are now moved to not cover the widget they are attached to.

### 🐛 Fixed
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

### ⭐ Added
* Plot: you can now set the stroke of a `HLine/VLine`.

### 🔧 Changed
* `Plot::new` now takes an `id_source: impl Hash` instead of a `name: impl ToString`. Functionally it is the same.


## 0.13.0 - 2021-06-24 - Better panels, plots and new visual style

### ⭐ Added
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

### 🔧 Changed
* Minimum Rust version is now 1.51 (used to be 1.52)
* [Tweaked the default visuals style](https://github.com/emilk/egui/pull/450).
* Plot: Renamed `Curve` to `Line`.
* `TopPanel::top` is now `TopBottomPanel::top`.
* `SidePanel::left` no longer takes the default width by argument, but by a builder call.
* `SidePanel::left` is resizable by default.

### 🐛 Fixed
* Fixed uneven lettering on non-integral device scales ("extortion lettering").
* Fixed invisible scroll bar when native window is too narrow for egui.


## 0.12.0 - 2021-05-10 - Multitouch, user memory, window pivots, and improved plots

### ⭐ Added
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

### 🔧 Changed
* Make `Memory::has_focus` public (again).
* `Plot` must now be given a name that is unique within its scope.
* Tab only selects labels if the `screen_reader` option is turned on.
* Renamed `ui.wrap` to `ui.scope`.

### 🐛 Fixed
* Fixed [defocus-bug on touch screens](https://github.com/emilk/egui/issues/288).
* Fixed bug with the layout of wide `DragValue`s.

### 🔥 Removed
* Moved experimental markup language to `egui_demo_lib`


## 0.11.0 - 2021-04-05 - Optimization, screen reader & new layout logic

### ⭐ Added
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

### 🔧 Changed
* `ui.advance_cursor` is now called `ui.add_space`.
* `kb_focus` is now just called `focus`.

### 🐛 Fixed
* Fixed some bugs related to centered layouts.
* Fixed secondary-click to open a menu.
* [Fix panic for zero-range sliders and zero-speed drag values](https://github.com/emilk/egui/pull/216).
* Fixed false id clash error for wrapping text.
* Fixed bug that would close a popup (e.g. the color picker) when clicking inside of it.

### ☢️ Deprecated
* Deprecated `combo_box_with_label` in favor of new `ComboBox`.
* Deprecated type-specific constructors for `Slider` and `DragValue` (`Slider::f32`, `DragValue::usize` etc).


## 0.10.0 - 2021-02-28 - Plot and polish

<img src="media/egui-0.10-plot.gif" width="50%">

### ⭐ Added
* Added `egui::plot::Plot` to plot some 2D data.
* Added `Ui::hyperlink_to(label, url)`.
* Sliders can now have a value prefix and suffix (e.g. the suffix `"°"` works like a unit).
* `Context::set_pixels_per_point` to control the scale of the UI.
* Added `Response::changed()` to query if e.g. a slider was dragged, text was entered or a checkbox was clicked.
* Added support for all integers in `DragValue` and `Slider` (except 128-bit).

### 🔧 Changed
* Improve the positioning of tooltips.
* Only show tooltips if mouse is still.
* `Slider` will now show the value display by default, unless turned off with `.show_value(false)`.
* The `Slider` value is now a `DragValue` which when dragged can pick values outside of the slider range (unless `clamp_to_range` is set).


## 0.9.0 - 2021-02-07 - Light Mode and much more

<img src="media/0.9.0-disabled.gif" width="50%">

### ⭐ Added
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

### 🔧 Changed
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

### 🐛 Fixed
* It is now possible to click widgets even when FPS is very low.
* Tessellator: handle sharp path corners better (switch to bevel instead of miter joints for > 90°).


## 0.8.0 - 2021-01-17 - Grid layout & new visual style

<img src="media/widget_gallery_0.8.0.gif" width="50%">

### ⭐ Added
* Added a simple grid layout (`Grid`).
* Added `ui.allocate_at_least` and `ui.allocate_exact_size`.
* Added function `InputState::key_down`.
* Added `Window::current_pos` to position a window.

### 🔧 Changed
* New simpler and sleeker look!
* Renamed `PaintCmd` to `Shape`.
* Replace tuple `(Rect, Shape)` with tuple-struct `ClippedShape`.
* Renamed feature `"serde"` to `"persistence"`.
* Break out the modules `math` and `paint` into separate crates `emath` and `epaint`.

### 🐛 Fixed
* Fixed a bug that would sometimes trigger a "Mismatching panels" panic in debug builds.
* `Image` and `ImageButton` will no longer stretch to fill a justified layout.


## 0.7.0 - 2021-01-04

### ⭐ Added
* Added `ui.scroll_to_cursor` and `response.scroll_to_me` ([#81](https://github.com/emilk/egui/pull/81) by [lucaspoffo](https://github.com/lucaspoffo)).
* Added `window.id(…)` and `area.id(…)` for overriding the default `Id`.

### 🔧 Changed
* Renamed `Srgba` to `Color32`.
* All color constructors now starts with `from_`, e.g. `Color32::from_rgb`.
* Renamed `FontFamily::VariableWidth` to `FontFamily::Proportional`.
* Removed `pixels_per_point` from `FontDefinitions`.

### 🐛 Fixed
* `RepaintSignal` now implements `Sync` so it can be sent to a background thread.
* `TextEdit` widgets are now slightly larger to accommodate their frames.

### ☢️ Deprecated
* Deprecated `color::srgba`.


## 0.6.0 - 2020-12-26

### ⭐ Added
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

### 🔧 Changed
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

### 🐛 Fixed
* The background for `CentralPanel` will now cover unused space too.
* `ui.columns`: Improve allocated size estimation.

### ☢️ Deprecated
* `RawInput::screen_size` - use `RawInput::screen_rect` instead.
* left/centered/right column functions on `Ui`.
* `ui.interact_hover` and `ui.hovered`.


## 0.5.0 - 2020-12-13

### ⭐ Added
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

### 🔧 Changed
* New default font: [Ubuntu-Light](https://fonts.google.com/specimen/Ubuntu).
* Make it simpler to override fonts in `FontDefinitions`.
* Remove minimum button width.
* Refactor `egui::Layout` substantially, changing its interface.
* Calling `on_hover_text`/`on_hover_ui` multiple times will stack tooltips underneath the previous ones.
* Text wrapping on labels, buttons, checkboxes and radio buttons is now based on the layout.

### 🔥 Removed

* Removed the `label!` macro.


## 0.4.0 - 2020-11-28

### ⭐ Added
* `TextEdit` improvements:
  * Much improved text editing, with better navigation and selection.
  * Move focus between `TextEdit` widgets with tab and shift-tab.
  * Undo edtis in a `TextEdit`.
  * You can now check if a `TextEdit` lost keyboard focus with `response.lost_focus`.
  * Added `ui.text_edit_singleline` and `ui.text_edit_multiline`.
* You can now debug why your `Ui` is unexpectedly wide with `ui.style_mut().debug.show_expand_width = true;`

### 🔧 Changed
* Pressing enter in a single-line `TextEdit` will now surrender keyboard focus for it.
* You must now be explicit when creating a `TextEdit` if you want it to be singeline or multiline.
* Improved automatic `Id` generation, making `Id` clashes less likely.
* egui now requires modifier key state from the integration
* Added, renamed and removed some keys in the `Key` enum.
* Fixed incorrect text wrapping width on radio buttons

### 🐛 Fixed
* Fixed bug where a lost widget could still retain keyboard focus.


## 0.3.0 - 2020-11-07

### ⭐ Added
* Panels: you can now create panels using `SidePanel`, `TopPanel` and `CentralPanel`.
* You can now override the default egui fonts.
* Added ability to override text color with `visuals.override_text_color`.
* The demo now includes a simple drag-and-drop example.
* The demo app now has a slider to scale all of egui.

### 🔧 Changed
* `ui.horizontal(…)` etc returns `Response`.
* Refactored the interface for `egui::app::App`.
* Windows are now constrained to the screen.
* `Context::begin_frame()` no longer returns a `Ui`. Instead put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
* `Context::end_frame()` now returns shapes that need to be converted to triangles with `Context::tessellate()`.
* Anti-aliasing is now off by default in debug builds.

### 🔥 Removed
* You can no longer throw windows.

### 🐛 Fixed
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
