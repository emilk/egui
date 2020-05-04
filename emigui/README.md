# GUI implementation
This is the core library crate Emigui. It is fully platform independent without any backend. You give the Emigui library input each frame (mouse pos etc), and it outputs a triangle mesh for you to paint.

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
    * [ ] BUG FIX: Don't catch clicks on closed windows
* [ ] Scroll areas
    * [x] Vertical scrolling
    * [ ] Horizontal scrolling
    * [x] Scroll-wheel input
    * [x] Drag background to scroll
    * [ ] Kinetic scrolling
* [x] Add support for clicking links
* [ ] Menu bar (File, Edit, etc)
* [ ] Text input
    * [x] Input events (key presses)
    * [x] Text focus
    * [ ] Cursor movement
    * [ ] Text selection
    * [ ] Clipboard copy/paste
    * [ ] Move focus with tab
    * [ ] Handle leading/trailing space
* [ ] Color picker
* [ ] Style editor
* [ ] Table with resizable columns
* [ ] Layout
    * [ ] Generalize Layout (separate from Region)
    * [ ] Cascading layout: same lite if it fits, else next line. Like text.
    * [ ] Grid layout

### Web version:
* [x] Scroll input
* [x] Change to resize cursor on hover
* [ ] Make it a JS library for easily creating your own stuff

### Animations
Add extremely quick animations for some things, maybe 2-3 frames. For instance:
* [x] Animate collapsing headers with clip_rect

### Clip rects
* [x] Separate Region::clip_rect from Region::rect
* [x] Use clip rectangles when painting
* [x] Use clip rectangles when interacting
* [x] Adjust clip rects so edges of child widgets aren't clipped
* [ ] Use HW clip rects

### Modularity
* [x] `trait Widget` (`Label`, `Slider`, `Checkbox`, ...)
* [ ] `trait Container` (`Frame`, `Resize`, `ScrollArea`, ...)
* [ ] `widget::TextButton` implemented as a `container::Button` which contains a `widget::Label`.
* [ ] Easily chain `Container`s without nested closures.
    * e.g. `region.containers((Frame::new(), Resize::new(), ScrollArea::new()), |ui| ...)`

### Input
* [ ] Distinguish between clicks and drags
* [ ] Double-click
* [x] Text

### Debugability / Inspection
* [x] Widget debug rectangles
* [ ] Easily debug why something keeps expanding


### Other
* [x] Persist UI state in external storage
* [ ] Pixel-perfect rendering (round positions to nearest pixel).
* [ ] Build in a profiler which tracks which region in which window takes up CPU.
    * [ ] Draw as flame graph
    * [ ] Draw as hotmap

### Names and structure
* [ ] Rename things to be more consistent with Dear ImGui
* [ ] Combine Emigui and Context?
* [ ] Solve which parts of Context are behind a mutex
    * [ ] All of Context behind one mutex?
    * [ } Break up Context into Input, State, Output ?
* [ ] Rename Region to something shorter?
    * `region: &Region` `region.add(...)` :/
    * `gui: &Gui` `gui.add(...)` :)
    * `ui: &Ui` `ui.add(...)` :)
* [ ] Maybe find a shorter name for the library like `egui`?

### Global widget search
Ability to do a search for any widget. The search works even for collapsed regions and closed windows and menus. This is implemented like this: while searching, all region are layed out and their add_content functions are run. If none of the contents matches the search, the layout is reverted and nothing is shown. So windows will get temporarily opened and run, but if the search is not a match in the window it is closed again. This means then when searching your whole GUI is being run, which may be a bit slower, but it would be a really awesome feature.
