Example of a dialog that is a native window (an `egui` viewport) which is owned by the main window and modal:
on Windows, while the dialog is open, the main window does not accept input.

```sh
cargo run -p modal_dialog
```

The dialog opens centered on the main window wherever the platform lets apps place windows.
On Windows it also stays above the main window, has no taskbar button, and appears without a flash.
On other platforms it is an ordinary separate window and the main window stays interactive.
When viewports are embedded (the backend has no native windows) the dialog is shown inside the main window with `egui::Modal`.
