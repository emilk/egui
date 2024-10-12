# egui_kittest 

Ui testing library for egui, based on [kittest](https://github.com/rerun-io/kittest) (an [AccessKit](https://github.com/AccessKit/accesskit) based testing library).

```rust
use egui::accesskit::{Role, Toggled};
use egui::{CentralPanel, Context, TextEdit, Vec2};
use egui_kittest::Harness;
use kittest::Queryable;
use std::cell::RefCell;

fn main() {
    let mut checked = false;
    let app = |ctx: &Context| {
        CentralPanel::default().show(ctx, |ui| {
            ui.checkbox(&mut checked, "Check me!");
        });
    };

    let mut harness = Harness::builder().with_size(egui::Vec2::new(200.0, 100.0)).build(app);
    
    let checkbox = harness.get_by_name("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::False));
    checkbox.click();
    
    harness.run();

    let checkbox = harness.get_by_name("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::True));

    // You can even render the ui and do image snapshot tests
    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    egui_kittest::image_snapshot(&egui_kittest::wgpu::TestRenderer::new().render(&harness), "readme_example");
}
```
