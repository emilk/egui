use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    let raw_input = egui::RawInput {
        screen_size: egui::vec2(1280.0, 1024.0),
        ..Default::default()
    };

    {
        let mut ctx = egui::Context::new();
        let mut demo_app = egui::demos::DemoApp::default();

        c.bench_function("demo_app", |b| {
            b.iter(|| {
                let mut ui = ctx.begin_frame(raw_input.clone());
                demo_app.ui(&mut ui, "");
                ctx.end_frame()
            })
        });
    }

    {
        let mut ctx = egui::Context::new();
        let mut ui = ctx.begin_frame(raw_input.clone());
        c.bench_function("label", |b| {
            b.iter(|| {
                ui.label(egui::demos::LOREM_IPSUM_LONG);
            })
        });
        // let _ = ctx.end_frame(); // skip, because tessellating all that text is slow
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
