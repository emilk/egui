use criterion::{criterion_group, criterion_main, Criterion};

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
                    ui.label(egui_demo_lib::LOREM_IPSUM_LONG);
                })
            });
        });
        let _ = ctx.end_frame();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
