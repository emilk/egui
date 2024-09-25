use egui::load::SizedTexture;
use egui::{include_image, ColorImage, ImageSource, Pos2, RawInput, Stroke, TextureId, Vec2};

fn main() {
    let ctx = egui::Context::default();

    let mut input = RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Default::default(),
            Vec2::new(400.0, 200.0),
        )),
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&input.viewport_id)
        .unwrap()
        .native_pixels_per_point = Some(2.0);
    let output = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.group(|ui| {
                ui.label("Hello World!");
                ui.button("Click me!");
                ui.checkbox(&mut true, "Check me!");
                ui.heading("Heading");
            });
            // ui.image(SizedTexture::new(
            //     TextureId::default(),
            //     Vec2::new(2048.0, 128.0),
            // ));
            ui.image(include_image!("../../../media/rerun_io_logo.png"));
        });

        // ctx.debug_painter().rect_filled(
        //     egui::Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(100.0, 100.0)),
        //     10.0,
        //     egui::Color32::from_rgba_premultiplied(255, 0, 0, 255),
        // );
        // ctx.debug_painter().rect_filled(
        //     egui::Rect::from_min_size(Pos2::new(100.0, 0.0), Vec2::new(100.0, 100.0)),
        //     10.0,
        //     egui::Color32::from_rgba_premultiplied(0, 255, 0, 255),
        // );
        //
        // ctx.debug_painter().rect_stroke(
        //     egui::Rect::from_min_size(Pos2::new(200.0, 0.0), Vec2::new(100.0, 100.0)),
        //     10.0,
        //     Stroke::new(10.0, egui::Color32::from_rgba_premultiplied(0, 0, 255, 255)),
        // );
    });

    let primitives = ctx.tessellate(output.shapes, ctx.pixels_per_point());

    let mut cpu_renderer = egui_cpu::Renderer::default();

    cpu_renderer.update_textures(output.textures_delta);

    dbg!(ctx.screen_rect());

    let image = cpu_renderer.render(
        &primitives,
        ctx.screen_rect().size(),
        ctx.pixels_per_point(),
    );

    image.save("output.png").unwrap();
}
