# GUI implementation
This is the core library crate Egui. It is fully platform independent without any backend. You give the Egui library input each frame (mouse pos etc), and it outputs a triangle mesh for you to paint.

## TODO:
### Widgets
* [x] Label
* [x] Button
* [x] Checkbox
* [x] Radiobutton
* [x] Horizontal slider
* [ ] Vertical slider
* [x] Collapsing header region
* [x] Tooltip
* [x] Movable/resizable windows
    * [x] Kinetic windows
    * [ ] Windows should open from `UI`s and be boxed by parent ui.
        * Then we could open the example app inside a window in the example app, recursively.
    * [x] Resize any side and corner on windows
    * [x] Fix autoshrink
    * [x] Automatic positioning of new windows
* [ ] Scroll areas
    * [x] Vertical scrolling
    * [ ] Horizontal scrolling
    * [x] Scroll-wheel input
    * [x] Drag background to scroll
    * [ ] Kinetic scrolling
* [x] Add support for clicking hyperlinks
* [x] Menu bar (File, Edit, etc)
    * [ ] Sub-menus
    * [ ] Keyboard shortcuts
* [ ] Text input
    * [x] Input events (key presses)
    * [x] Text focus
    * [x] Cursor movement
    * [ ] Text selection
    * [ ] Clipboard copy/paste
    * [ ] Move focus with tab
    * [x] Handle leading/trailing space
* [ ] Color picker
* [ ] Style editor
* [ ] Table with resizable columns
* [ ] Layout
    * [ ] Generalize Layout (separate from Ui)
    * [ ] Cascading layout: same lite if it fits, else next line. Like text.
    * [ ] Grid layout
    * [ ] Point list
* [ ] Image support

### Web version:
* [x] Scroll input
* [x] Change to resize cursor on hover
* [ ] Make it a JS library for easily creating your own stuff
* [x] Read url fragment and redirect to a subpage (e.g. different examples apps)

### Visuals
* [x] Simplify button style to make for nicer collapsible headers. Maybe weak outline? Or just subtle different text color?
* [/] Pixel-perfect painting (round positions to nearest pixel).
* [ ] Make sure alpha blending is correct (different between web and glium)
* [ ] Color picker widgets
* [ ] Fix thin rounded corners rendering bug (too bright)

### Animations
Add extremely quick animations for some things, maybe 2-3 frames. For instance:
* [x] Animate collapsing headers with clip_rect

### Clip rects
* [x] Separate Ui::clip_rect from Ui::rect
* [x] Use clip rectangles when painting
* [x] Use clip rectangles when interacting
* [x] Adjust clip rects so edges of child widgets aren't clipped
* [x] Use HW clip rects

### Modularity
* [x] `trait Widget` (`Label`, `Slider`, `Checkbox`, ...)
* [ ] `trait Container` (`Frame`, `Resize`, `ScrollArea`, ...)
* [ ] `widget::TextButton` implemented as a `container::Button` which contains a `widget::Label`.
* [ ] Easily chain `Container`s without nested closures.
    * e.g. `ui.containers((Frame::new(), Resize::new(), ScrollArea::new()), |ui| ...)`

### Input
* [x] Distinguish between clicks and drags
* [x] Double-click
* [x] Text
* [ ] Support all mouse buttons

### Debugability / Inspection
* [x] Widget debug rectangles
* [x] Easily debug why something keeps expanding

### Other
* [x] Persist UI state in external storage
* [ ] Persist Example App state
* [ ] Build in a profiler which tracks which `Ui` in which window takes up CPU.
    * [ ] Draw as flame graph
    * [ ] Draw as hotmap
* [ ] Change `width.min(max_width)` to `width.at_most(max_width)`

### Names and structure
* [ ] Rename things to be more consistent with Dear ImGui
* [x] Combine Egui and Context?
* [x] Solve which parts of Context are behind a mutex
* [x] Rename Region to Ui
* [ ] Move Path and Triangles to own crate
* [ ] Maybe find a shorter name for the library like `egui`?

### Global widget search
Ability to do a search for any widget. The search works even for collapsed regions and closed windows and menus. This is implemented like this: while searching, all region are layed out and their add_content functions are run. If none of the contents matches the search, the layout is reverted and nothing is shown. So windows will get temporarily opened and run, but if the search is not a match in the window it is closed again. This means then when searching your whole GUI is being run, which may be a bit slower, but it would be a really awesome feature.
