use criterion::{black_box, criterion_group, criterion_main, Criterion};

use epaint::*;

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

fn tessellate_circles(c: &mut Criterion) {
    c.bench_function("tessellate_circles_100k", move |b| {
        let radii: [f32; 10] = [1.0, 2.0, 3.6, 4.0, 5.7, 8.0, 10.0, 13.0, 15.0, 17.0];
        let mut clipped_shapes = vec![];
        for r in radii {
            for _ in 0..10_000 {
                let clip_rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1024.0));
                let shape = Shape::circle_filled(Pos2::new(10.0, 10.0), r, Color32::WHITE);
                clipped_shapes.push(ClippedShape(clip_rect, shape));
            }
        }
        assert_eq!(clipped_shapes.len(), 100_000);

        let pixels_per_point = 2.0;
        let options = TessellationOptions::default();

        let atlas = TextureAtlas::new([4096, 256]);
        let font_tex_size = atlas.size();
        let prepared_discs = atlas.prepared_discs();

        b.iter(|| {
            let clipped_primitive = tessellate_shapes(
                pixels_per_point,
                options,
                font_tex_size,
                prepared_discs.clone(),
                clipped_shapes.clone(),
            );
            black_box(clipped_primitive);
        });
    });
}

criterion_group!(
    benches,
    single_dashed_lines,
    many_dashed_lines,
    tessellate_circles
);
criterion_main!(benches);
