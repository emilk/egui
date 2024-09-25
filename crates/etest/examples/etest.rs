use accesskit_query::Queryable;
use egui::accesskit::{Role, Toggled};
use egui::{CentralPanel, Context, TextEdit, Vec2};
use etest::Harness;
use std::cell::RefCell;

fn main() {
    let checked = RefCell::new(false);
    let text = RefCell::new(String::new());
    let mut app = |ctx: &Context| {
        CentralPanel::default().show(ctx, |ui| {
            ui.checkbox(&mut checked.borrow_mut(), "Check me!");
            TextEdit::singleline(&mut *text.borrow_mut())
                .hint_text("Type here")
                .show(ui);
        });
    };

    let mut harness = Harness::new().with_size(Vec2::new(200.0, 100.0));

    harness.run(&mut app);

    harness.get_by_name("Check me!").click();

    harness.run(&mut app);

    assert!(*checked.borrow());
    let checkbox = harness.get_by_name("Check me!");
    assert_eq!(checkbox.toggled(), Some(Toggled::True));

    harness
        .get_by_role(Role::TextInput)
        .type_text("Hello, World!");

    harness.run(&mut app);

    assert_eq!(&*text.borrow_mut(), "Hello, World!");
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("Hello, World!")
    );

    #[cfg(feature = "wgpu")]
    {
        let mut renderer = etest::wgpu::TestRenderer::new();
        let image = renderer.render(&harness);

        image.save("crates/etest/etest.png").unwrap();
    }
}
