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
    * [ ] Kinetic windows
* [ ] Scroll areas
    * [x] Vertical scrolling
    * [ ] Horizontal scrolling
    * [x] Scroll-wheel input
        * [ ] Scroll-wheel input in web verions
    * [x] Drag background to scroll
    * [ ] Kinetic scrolling
* [ ] Menu bar (File, Edit, etc)
* [ ] One-line TextField
    * [ ] Clipboard copy/paste
* [ ] Color picker
* [ ] Style editor

### Animations
Add extremely quick animations for some things, maybe 2-3 frames. For instance:
* [x] Animate collapsing headers with clip_rect

### Clip rects
* [x] Separate Region::clip_rect from Region::rect
* [x] Use clip rectangles when painting
* [ ] Use clip rectangles when interacting
* [x] Adjust clip rects so edges of child widgets aren't clipped

### Other
* [ ] Generalize Layout so we can create grid layouts etc
* [ ] Persist UI state in external storage
* [ ] Build in a profiler which tracks which region in which window takes up CPU.
    * [ ] Draw as flame graph
    * [ ] Draw as hotmap

### Names and structure
* [ ] Rename things to be more consistent with Dear ImGui
* [ ] Combine Emigui and Context
* [ ] Rename Region to something shorter?
    * `region: &Region` `region.add(...)` :/
    * `gui: &Gui` `gui.add(...)` :)
    * `ui: &Ui` `ui.add(...)` :)

### Global widget search
Ability to do a search for any widget. The search works even for collapsed regions and closed windows and menus. This is implemented like this: while searching, all region are layed out and their add_content functions are run. If none of the contents matches the search, the layout is reverted and nothing is shown. So windows will get temporarily opened and run, but if the search is not a match in the window it is closed again. This means then when searching your whole GUI is being run, which may be a bit slower, but it would be a really awesome feature.
