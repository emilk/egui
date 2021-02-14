use criterion::{criterion_group, criterion_main, Criterion};

use egui_demo_lib::LOREM_IPSUM_LONG;

pub fn criterion_benchmark(c: &mut Criterion) {
    let raw_input = egui::RawInput::default();

    {
        let mut ctx = egui::CtxRef::default();
        let mut demo_windows = egui_demo_lib::DemoWindows::default();

        c.bench_function("demo_windows_minimal", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx);
                ctx.end_frame()
            })
        });
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.memory().set_everything_is_visible(true); // give us everything
        let mut demo_windows = egui_demo_lib::DemoWindows::default();

        c.bench_function("demo_windows_full", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx);
                ctx.end_frame()
            })
        });
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.memory().set_everything_is_visible(true); // give us everything
        let mut demo_windows = egui_demo_lib::DemoWindows::default();
        ctx.begin_frame(raw_input.clone());
        demo_windows.ui(&ctx);
        let (_, shapes) = ctx.end_frame();

        c.bench_function("tessellate", |b| b.iter(|| ctx.tessellate(shapes.clone())));
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.begin_frame(raw_input);
        egui::CentralPanel::default().show(&ctx, |ui| {
            c.bench_function("label", |b| {
                b.iter(|| {
                    ui.label(LOREM_IPSUM_LONG);
                })
            });
        });
        let _ = ctx.end_frame();
    }

    {
        let pixels_per_point = 1.0;
        let wrap_width = 512.0;
        let text_style = egui::TextStyle::Body;
        let fonts = egui::epaint::text::Fonts::from_definitions(
            pixels_per_point,
            egui::FontDefinitions::default(),
        );
        let font = &fonts[text_style];
        c.bench_function("text layout", |b| {
            b.iter(|| font.layout_multiline(LOREM_IPSUM_LONG.to_owned(), wrap_width))
        });

        let galley = font.layout_multiline(LOREM_IPSUM_LONG.to_owned(), wrap_width);
        let mut tesselator = egui::epaint::Tessellator::from_options(Default::default());
        let mut mesh = egui::epaint::Mesh::default();
        c.bench_function("tesselate text", |b| {
            b.iter(|| {
                let fake_italics = false;
                tesselator.tessellate_text(
                    &fonts,
                    egui::Pos2::ZERO,
                    &galley,
                    text_style,
                    egui::Color32::WHITE,
                    fake_italics,
                    &mut mesh,
                );
                mesh.clear();
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
