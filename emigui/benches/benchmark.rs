use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut example_app = emigui::examples::ExampleApp::default();
    let mut ctx = emigui::Context::new(1.0);

    let raw_input = emigui::RawInput {
        screen_size: emigui::vec2(1280.0, 1024.0),
        ..Default::default()
    };

    c.bench_function("example_app", |b| {
        b.iter(|| {
            ctx.begin_frame(raw_input.clone());
            let mut ui = ctx.fullscreen_ui();
            example_app.ui(&mut ui, "");
            ctx.end_frame()
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
