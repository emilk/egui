# GUI implementation
This is the core library crate Emigui. It is fully platform independent without any backend. You give the Emigui library input each frame (mouse pos etc), and it outputs a triangle mesh for you to paint.

## TODO:
* Widgets:
    * Movable/resizable windows
    * Scroll areas (requires scissor tests / clip rects in paint backend)
    * Menu bar (File, Edit, etc)
    * One-line text input
    * Color picker
    * Style editor
    * Persist UI state in external storage
* Rename Region to something shorter?
    * `region: &Region` `region.add(...)` :/
    * `gui: &Gui` `gui.add(...)` :)
    * `ui: &Ui` `ui.add(...)` :)
