use egui::accesskit::{Role, Toggled};
use egui::{CentralPanel, Context, TextEdit, Vec2};
use egui_kittest::Harness;
use kittest::Queryable;
use std::cell::RefCell;

fn main() {
    let checked = RefCell::new(false);
    let text = RefCell::new(String::new());
    let app = |ctx: &Context| {
        CentralPanel::default().show(ctx, |ui| {
            ui.checkbox(&mut checked.borrow_mut(), "Check me!");
            TextEdit::singleline(&mut *text.borrow_mut())
                .hint_text("Type here")
                .show(ui);
        });
    };

    let mut harness = Harness::new(app).with_size(Vec2::new(200.0, 100.0));

    harness.run();

    harness.get_by_name("Check me!").click();

    harness.run();

    assert!(*checked.borrow());
    let checkbox = harness.get_by_name("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::True));

    harness
        .get_by_role(Role::TextInput)
        .type_text("Hello, World!");

    harness.run();

    assert_eq!(&*text.borrow_mut(), "Hello, World!");
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("Hello, World!")
    );

    #[cfg(feature = "wgpu")]
    {
        let mut renderer = egui_kittest::wgpu::TestRenderer::new();
        let image = renderer.render(&harness);

        image.save("../kittest.png").unwrap();
    }
}
