use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    let raw_input = egui::RawInput::default();

    {
        let mut ctx = egui::CtxRef::default();
        let mut demo_windows = egui::demos::DemoWindows::default();

        c.bench_function("demo_windows_minimal", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx, &Default::default(), &mut None, |_ui| {});
                ctx.end_frame()
            })
        });
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.memory().all_collpasing_are_open = true; // expand the demo window with everything
        let mut demo_windows = egui::demos::DemoWindows::default();

        c.bench_function("demo_windows_full", |b| {
            b.iter(|| {
                ctx.begin_frame(raw_input.clone());
                demo_windows.ui(&ctx, &Default::default(), &mut None, |_ui| {});
                ctx.end_frame()
            })
        });
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.memory().all_collpasing_are_open = true; // expand the demo window with everything
        let mut demo_windows = egui::demos::DemoWindows::default();
        ctx.begin_frame(raw_input.clone());
        demo_windows.ui(&ctx, &Default::default(), &mut None, |_ui| {});
        let (_, paint_commands) = ctx.end_frame();

        c.bench_function("tesselate", |b| {
            b.iter(|| ctx.tesselate(paint_commands.clone()))
        });
    }

    {
        let mut ctx = egui::CtxRef::default();
        ctx.begin_frame(raw_input);
        egui::CentralPanel::default().show(&ctx, |ui| {
            c.bench_function("label", |b| {
                b.iter(|| {
                    ui.label(egui::demos::LOREM_IPSUM_LONG);
                })
            });
        });
        let _ = ctx.end_frame();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
