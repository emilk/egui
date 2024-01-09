use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration,
};

fn local_data(n: u64) {
    let ctx = egui_master::Context::default();
    egui_master::Area::new("test").show(&ctx, |_ui| {
        let mut data = None;
        for i in 0..n {
            let _ = data
                .get_or_insert_with(egui_master::util::IdTypeMap::default)
                .get_temp::<u64>(egui_master::Id::new(i));
        }
    });
}

fn global_data(n: u64) {
    let ctx = egui_master::Context::default();
    egui_master::Area::new("test").show(&ctx, |ui| {
        for i in 0..n {
            let _ = ui.data_mut(|data| data.get_temp::<u64>(egui_master::Id::new(i)));
        }
    });
}

fn new_local_data(n: u64) {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        for i in 0..n {
            let _ = ui.local_data_mut().get_temp::<u64>(egui::Id::new(i));
        }
    });
}

fn new_global_data(n: u64) {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        for i in 0..n {
            let _ = ui.data_mut(|data| data.get_temp::<u64>(egui::Id::new(i)));
        }
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("data");
    // 1, 10, 100, 1000, 10000
    for i in (0..5).map(|i| 10u64.pow(i)) {
        group.bench_with_input(BenchmarkId::new("local", i), &i, |b, i| {
            b.iter(|| local_data(black_box(*i)));
        });
        group.bench_with_input(BenchmarkId::new("global", i), &i, |b, i| {
            b.iter(|| global_data(black_box(*i)));
        });
        group.bench_with_input(BenchmarkId::new("new_local", i), &i, |b, i| {
            b.iter(|| new_local_data(black_box(*i)));
        });
        group.bench_with_input(BenchmarkId::new("new_global", i), &i, |b, i| {
            b.iter(|| new_global_data(black_box(*i)));
        });
    }
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
