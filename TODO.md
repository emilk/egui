# TODO
TODO-list for the Egui project. If you looking for something to do, look here.

* Widgets:
    * [ ] Text input
        * [x] Input
        * [x] Text focus
        * [x] Cursor movement
        * [ ] Text selection
        * [ ] Clipboard copy/paste
        * [ ] Move focus with tab
    * [ ] Horizontal slider
    * [/] Color picker
        * [x] linear rgb <-> sRGB
        * [x] HSV
        * [x] Color edit button with popup color picker
        * [ ] Easily edit users own (s)RGBA quadruplets (`&mut [u8;4]`/`[f32;4]`)
* Containers
    * [ ] Scroll areas
        * [x] Vertical scrolling
        * [x] Scroll-wheel input
        * [x] Drag background to scroll
        * [ ] Horizontal scrolling
        * [X] Kinetic scrolling
* [ ] Text
    * [ ] Unicode
        * [ ] Shared mutable expanding texture map?
    * [ ] Change text style/color and continue in same layout
* [ ] Menu bar (File, Edit, etc)
    * [ ] Sub-menus
    * [ ] Keyboard shortcuts
* [ ] Layout
    * [x] Generalize Layout (separate from Ui)
    * [ ] Table with resizable columns
    * [ ] Grid layout
    * [ ] Point list
* Windows
    * [ ] Positioning preference: `window.preference(Top, Right)`
        * [ ] Keeping right/bottom on expand. Maybe cover jitteryness with quick animation?
* [ ] Image support
    * [ ] user-chosen texture ids (so people can show thing with mipmaps and whatnot)
        * [ ] `enum TextureId { Egui, User(u64) }` added to `Triangles`
    * [ ] API for creating a texture managed by Egui
        * Backend-agnostic. Good for people doing Egui-apps (games etc).
        * [ ] Convert font texture to RGBA, or communicate format in initialization?
        * [ ] Generalized font atlas
* Visuals
    * [x] Pixel-perfect painting (round positions to nearest pixel).
    * [x] Fix `aa_size`: should be 1, currently fudged at 1.5
    * [x] Fix thin rounded corners rendering bug (too bright)
    * [x] Smoother animation (e.g. ease-out)? NO: animation are too brief for subtelty
    * [ ] Veriy alpha and sRGB correctness
        * [x] sRGBA decode in fragment shader
    * [ ] Thin circles look bad
    * [ ] Color picker
* Math
    * [ ] Change `width.min(max_width)` to `width.at_most(max_width)`

## egui_web
    * [x] Scroll input
    * [x] Change to resize cursor on hover
    * [x] Port most code to Rust
    * [x] Read url fragment and redirect to a subpage (e.g. different examples apps)
    * [ ] Embeddability
        * [ ] Support canvas that does NOT cover entire screen.
        * [ ] Support multiple eguis in one web page.
        * [ ] Filtering events to avoid too frequent repaints
        * [ ] Multiple canvases from the same rust code
            * Different Egui instances, same app
            * Allows very nice web integration

## Modularity
* [x] `trait Widget` (`Label`, `Slider`, `Checkbox`, ...)
* [ ] `trait Container` (`Frame`, `Resize`, `ScrollArea`, ...)
* [ ] `widget::TextButton` implemented as a `container::Button` which contains a `widget::Label`.
* [ ] Easily chain `Container`s without nested closures.
    * e.g. `ui.containers((Frame::new(), Resize::new(), ScrollArea::new()), |ui| ...)`
* [ ] Attach labels to checkboxes, radio buttons and sliders with a separate wrapper-widget ?

## Input
* [x] Distinguish between clicks and drags
* [x] Double-click
* [x] Text
* [ ] Support all mouse buttons
* [ ] Distinguish between touch input and mouse input

## Other
* [x] Persist UI state in external storage
* [x] Persist Example App state
* [ ] Create an Egui icon (or use an emoji)
* [ ] Build in a profiler which tracks which `Ui` in which window takes up CPU.
    * [ ] Draw as flame graph
    * [ ] Draw as hotmap
* [ ] Windows should open from `UI`s and be boxed by parent ui.
    * Then we could open the example app inside a window in the example app, recursively.
* [ ] Implement a minimal markdown viewer

## Names and structure
* [x] Combine Egui and Context?
* [x] Solve which parts of Context are behind a mutex
* [x] Rename Region to Ui
* [x] Maybe find a shorter name for the library like `egui`?
* [ ] Rename things to be more consistent with Dear ImGui ?

## Global widget search
Ability to do a search for any widget. The search works even for collapsed regions and closed windows and menus. This is implemented like this: while searching, all region are layed out and their add_content functions are run. If none of the contents matches the search, the layout is reverted and nothing is shown. So windows will get temporarily opened and run, but if the search is not a match in the window it is closed again. This means then when searching your whole GUI is being run, which may be a bit slower, but it would be a really awesome feature.

# Done:
* Widgets
    * [x] Label
    * [x] Button
    * [x] Checkbox
    * [x] Radiobutton
    * [x] Collapsing header region
    * [x] Tooltip
    * [x] Movable/resizable windows
        * [x] Kinetic windows
    * [x] Add support for clicking hyperlinks
* Containers
    * [x] Vertical slider
        * [x] Resize any side and corner on windows
        * [x] Fix autoshrink
        * [x] Automatic positioning of new windows
* Simple animations
* Clip rects
    * [x] Separate Ui::clip_rect from Ui::rect
    * [x] Use clip rectangles when painting
    * [x] Use clip rectangles when interacting
    * [x] Adjust clip rects so edges of child widgets aren't clipped
    * [x] Use HW clip rects
