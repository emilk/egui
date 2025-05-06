use std::fmt::Write as _;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use egui::epaint::TextShape;
use egui::load::SizedTexture;
use egui::{Button, Id, RichText, TextureId, Ui, UiBuilder, Vec2};
use egui_demo_lib::LOREM_IPSUM_LONG;
use rand::Rng as _;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc; // Much faster allocator

/// Each iteration should be called in their own `Ui` with an intentional id clash,
/// to prevent the Context from building a massive map of `WidgetRects` (which would slow the test,
/// causing unreliable results).
fn create_benchmark_ui(ctx: &egui::Context) -> Ui {
    Ui::new(ctx.clone(), Id::new("clashing_id"), UiBuilder::new())
}

pub fn criterion_benchmark(c: &mut Criterion) {
    use egui::RawInput;

    {
        let ctx = egui::Context::default();
        let mut demo_windows = egui_demo_lib::DemoWindows::default();

        // The most end-to-end benchmark.
        c.bench_function("demo_with_tessellate__realistic", |b| {
            b.iter(|| {
                let full_output = ctx.run(RawInput::default(), |ctx| {
                    demo_windows.ui(ctx);
                });
                ctx.tessellate(full_output.shapes, full_output.pixels_per_point)
            });
        });

        c.bench_function("demo_no_tessellate", |b| {
            b.iter(|| {
                ctx.run(RawInput::default(), |ctx| {
                    demo_windows.ui(ctx);
                })
            });
        });

        let full_output = ctx.run(RawInput::default(), |ctx| {
            demo_windows.ui(ctx);
        });
        c.bench_function("demo_only_tessellate", |b| {
            b.iter(|| ctx.tessellate(full_output.shapes.clone(), full_output.pixels_per_point));
        });
    }

    if false {
        let ctx = egui::Context::default();
        ctx.memory_mut(|m| m.set_everything_is_visible(true)); // give us everything
        let mut demo_windows = egui_demo_lib::DemoWindows::default();
        c.bench_function("demo_full_no_tessellate", |b| {
            b.iter(|| {
                ctx.run(RawInput::default(), |ctx| {
                    demo_windows.ui(ctx);
                })
            });
        });
    }

    {
        let ctx = egui::Context::default();
        let _ = ctx.run(RawInput::default(), |ctx| {
            c.bench_function("label &str", |b| {
                b.iter_batched_ref(
                    || create_benchmark_ui(ctx),
                    |ui| {
                        ui.label("the quick brown fox jumps over the lazy dog");
                    },
                    BatchSize::LargeInput,
                );
            });
            c.bench_function("label format!", |b| {
                b.iter_batched_ref(
                    || create_benchmark_ui(ctx),
                    |ui| {
                        ui.label("the quick brown fox jumps over the lazy dog".to_owned());
                    },
                    BatchSize::LargeInput,
                );
            });
        });
    }

    {
        let ctx = egui::Context::default();
        let _ = ctx.run(RawInput::default(), |ctx| {
            let mut group = c.benchmark_group("button");

            // To ensure we have a valid image, let's use the font texture. The size
            // shouldn't be important for this benchmark.
            let image = SizedTexture::new(TextureId::default(), Vec2::splat(16.0));

            group.bench_function("1_button_text", |b| {
                b.iter_batched_ref(
                    || create_benchmark_ui(ctx),
                    |ui| {
                        ui.add(Button::new("Hello World"));
                    },
                    BatchSize::LargeInput,
                );
            });
            group.bench_function("2_button_text_image", |b| {
                b.iter_batched_ref(
                    || create_benchmark_ui(ctx),
                    |ui| {
                        ui.add(Button::image_and_text(image, "Hello World"));
                    },
                    BatchSize::LargeInput,
                );
            });
            group.bench_function("3_button_text_image_right_text", |b| {
                b.iter_batched_ref(
                    || create_benchmark_ui(ctx),
                    |ui| {
                        ui.add(Button::image_and_text(image, "Hello World").right_text("⏵"));
                    },
                    BatchSize::LargeInput,
                );
            });
            group.bench_function("4_button_italic", |b| {
                b.iter_batched_ref(
                    || create_benchmark_ui(ctx),
                    |ui| {
                        ui.add(Button::new(RichText::new("Hello World").italics()));
                    },
                    BatchSize::LargeInput,
                );
            });
        });
    }

    {
        let ctx = egui::Context::default();
        ctx.begin_pass(RawInput::default());

        egui::CentralPanel::default().show(&ctx, |ui| {
            c.bench_function("Painter::rect", |b| {
                let painter = ui.painter();
                let rect = ui.max_rect();
                b.iter(|| {
                    painter.rect(
                        rect,
                        2.0,
                        egui::Color32::RED,
                        (1.0, egui::Color32::WHITE),
                        egui::StrokeKind::Inside,
                    );
                });
            });
        });

        // Don't call `end_pass` to not have to drain the huge paint list
    }

    {
        let pixels_per_point = 1.0;
        let max_texture_side = 8 * 1024;
        let wrap_width = 512.0;
        let font_id = egui::FontId::default();
        let text_color = egui::Color32::WHITE;
        let fonts = egui::epaint::text::Fonts::new(
            pixels_per_point,
            max_texture_side,
            egui::FontDefinitions::default(),
        );
        {
            let mut locked_fonts = fonts.lock();
            c.bench_function("text_layout_uncached", |b| {
                b.iter(|| {
                    use egui::epaint::text::{layout, LayoutJob};

                    let job = LayoutJob::simple(
                        LOREM_IPSUM_LONG.to_owned(),
                        font_id.clone(),
                        text_color,
                        wrap_width,
                    );
                    layout(&mut locked_fonts.fonts, job.into())
                });
            });
        }
        c.bench_function("text_layout_cached", |b| {
            b.iter(|| {
                fonts.layout(
                    LOREM_IPSUM_LONG.to_owned(),
                    font_id.clone(),
                    text_color,
                    wrap_width,
                )
            });
        });

        c.bench_function("text_layout_cached_many_lines_modified", |b| {
            const NUM_LINES: usize = 2_000;

            let mut string = String::new();
            for _ in 0..NUM_LINES {
                for i in 0..30_u8 {
                    write!(string, "{i:02X} ").unwrap();
                }
                string.push('\n');
            }

            let mut rng = rand::rng();
            b.iter(|| {
                fonts.begin_pass(pixels_per_point, max_texture_side);

                // Delete a random character, simulating a user making an edit in a long file:
                let mut new_string = string.clone();
                let idx = rng.random_range(0..string.len());
                new_string.remove(idx);

                fonts.layout(new_string, font_id.clone(), text_color, wrap_width);
            });
        });

        let galley = fonts.layout(LOREM_IPSUM_LONG.to_owned(), font_id, text_color, wrap_width);
        let font_image_size = fonts.font_image_size();
        let prepared_discs = fonts.texture_atlas().lock().prepared_discs();
        let mut tessellator = egui::epaint::Tessellator::new(
            1.0,
            Default::default(),
            font_image_size,
            prepared_discs,
        );
        let mut mesh = egui::epaint::Mesh::default();
        let text_shape = TextShape::new(egui::Pos2::ZERO, galley, text_color);
        c.bench_function("tessellate_text", |b| {
            b.iter(|| {
                tessellator.tessellate_text(&text_shape, &mut mesh);
                mesh.clear();
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
