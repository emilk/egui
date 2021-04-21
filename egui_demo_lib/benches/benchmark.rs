use criterion::{criterion_group, criterion_main, Criterion};

use egui_demo_lib::LOREM_IPSUM_LONG;

pub fn criterion_benchmark(c: &mut Criterion) {
    let raw_input = egui::RawInput::default();

    {
        let mut ctx = egui::CtxRef::default();
        let mut demo_windows = egui_demo_lib::DemoWindows::default();

        // The most end-to-end benchmark.
        c.bench_function("demo_with_tesselate__realistic", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx);
                let (_, shapes) = ctx.end_frame();
                ctx.tessellate(shapes)
            })
        });

        c.bench_function("demo_no_tesselate", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx);
                ctx.end_frame()
            })
        });

        ctx.begin_frame(raw_input.clone());
        demo_windows.ui(&ctx);
        let (_, shapes) = ctx.end_frame();
        c.bench_function("demo_only_tessellate", |b| {
            b.iter(|| ctx.tessellate(shapes.clone()))
        });
    }

    if false {
        let mut ctx = egui::CtxRef::default();
        ctx.memory().set_everything_is_visible(true); // give us everything
        let mut demo_windows = egui_demo_lib::DemoWindows::default();
        c.bench_function("demo_full_no_tesselate", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx);
                ctx.end_frame()
            })
        });
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.begin_frame(raw_input);
        let mut ui = egui::Ui::__test();
        c.bench_function("label &str", |b| {
            b.iter(|| {
                ui.label("the quick brown fox jumps over the lazy dog");
            })
        });
        c.bench_function("label format!", |b| {
            b.iter(|| {
                ui.label("the quick brown fox jumps over the lazy dog".to_owned());
            })
        });
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
        c.bench_function("text_layout_uncached", |b| {
            b.iter(|| font.layout_multiline(LOREM_IPSUM_LONG.to_owned(), wrap_width))
        });
        c.bench_function("text_layout_cached", |b| {
            b.iter(|| fonts.layout_multiline(text_style, LOREM_IPSUM_LONG.to_owned(), wrap_width))
        });

        let galley = font.layout_multiline(LOREM_IPSUM_LONG.to_owned(), wrap_width);
        let mut tessellator = egui::epaint::Tessellator::from_options(Default::default());
        let mut mesh = egui::epaint::Mesh::default();
        c.bench_function("tessellate_text", |b| {
            b.iter(|| {
                let fake_italics = false;
                tessellator.tessellate_text(
                    fonts.texture().size(),
                    egui::Pos2::ZERO,
                    &galley,
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
