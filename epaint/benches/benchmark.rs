use criterion::{black_box, criterion_group, criterion_main, Criterion};

use epaint::{pos2, Color32, Shape, Stroke};

fn single_dashed_lines(c: &mut Criterion) {
    c.bench_function("single_dashed_lines", move |b| {
        b.iter(|| {
            let mut v = Vec::new();

            let line = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(100.0, 1.0)];

            for _ in 0..100 {
                v.extend(Shape::dashed_line(
                    &line,
                    Stroke::new(1.5, Color32::RED),
                    10.0,
                    2.5,
                ));
            }

            black_box(v);
        });
    });
}

fn many_dashed_lines(c: &mut Criterion) {
    c.bench_function("many_dashed_lines", move |b| {
        b.iter(|| {
            let mut v = Vec::new();

            let line = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(100.0, 1.0)];

            for _ in 0..100 {
                Shape::dashed_line_many(&line, Stroke::new(1.5, Color32::RED), 10.0, 2.5, &mut v);
            }

            black_box(v);
        });
    });
}

criterion_group!(benches, single_dashed_lines, many_dashed_lines);
criterion_main!(benches);
