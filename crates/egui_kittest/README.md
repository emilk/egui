# egui_kittest 

Ui testing library for egui, based on [kittest](https://github.com/rerun-io/kittest) (an [AccessKit](https://github.com/AccessKit/accesskit) based testing library).

## Example usage
```rust
use egui::accesskit::Toggled;
use egui_kittest::{Harness, kittest::Queryable};

fn main() {
    let mut checked = false;
    let app = |ui: &mut egui::Ui| {
        ui.checkbox(&mut checked, "Check me!");
    };

    let mut harness = Harness::new_ui(app);
    
    let checkbox = harness.get_by_label("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::False));
    checkbox.click();
    
    harness.run();

    let checkbox = harness.get_by_label("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::True));
    
    // Shrink the window size to the smallest size possible
    harness.fit_contents();

    // You can even render the ui and do image snapshot tests
    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    harness.wgpu_snapshot("readme_example");
}
```

## Snapshot testing
There is a snapshot testing feature. To create snapshot tests, enable the `snapshot` and `wgpu` features.
Once enabled, you can call `Harness::wgpu_snapshot` to render the ui and save the image to the `tests/snapshots` directory.

To update the snapshots, run your tests with `UPDATE_SNAPSHOTS=true`, so e.g. `UPDATE_SNAPSHOTS=true cargo test`.
Running with `UPDATE_SNAPSHOTS=true` will still cause the tests to fail, but on the next run, the tests should pass.

If you want to have multiple snapshots in the same test, it makes sense to collect the results in a `Vec` 
([look here](https://github.com/emilk/egui/blob/70a01138b77f9c5724a35a6ef750b9ae1ab9f2dc/crates/egui_demo_lib/src/demo/demo_app_windows.rs#L388-L427) for an example).
This way they can all be updated at the same time.

You should add the following to your `.gitignore`:
```gitignore
**/tests/snapshots/**/*.diff.png
**/tests/snapshots/**/*.new.png
```
