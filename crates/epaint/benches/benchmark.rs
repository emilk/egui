use criterion::{black_box, criterion_group, criterion_main, Criterion};

use epaint::{tessellator::Path, *};

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
                clipped_shapes.push(ClippedShape { clip_rect, shape });
            }
        }
        assert_eq!(clipped_shapes.len(), 100_000);

        let pixels_per_point = 2.0;
        let options = TessellationOptions::default();

        let atlas = TextureAtlas::new([4096, 256]);
        let font_tex_size = atlas.size();
        let prepared_discs = atlas.prepared_discs();

        b.iter(|| {
            let mut tessellator = Tessellator::new(
                pixels_per_point,
                options,
                font_tex_size,
                prepared_discs.clone(),
            );
            let clipped_primitives = tessellator.tessellate_shapes(clipped_shapes.clone());
            black_box(clipped_primitives);
        });
    });
}

fn thick_line_solid(c: &mut Criterion) {
    c.bench_function("thick_solid_line", move |b| {
        let line = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(100.0, 1.0)];
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(1.5, &Stroke::new(2.0, Color32::RED).into(), &mut mesh);

            black_box(mesh);
        });
    });
}

fn thick_large_line_solid(c: &mut Criterion) {
    c.bench_function("thick_large_solid_line", move |b| {
        let line = (0..1000).map(|i| pos2(i as f32, 10.0)).collect::<Vec<_>>();
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(1.5, &Stroke::new(2.0, Color32::RED).into(), &mut mesh);

            black_box(mesh);
        });
    });
}

fn thin_line_solid(c: &mut Criterion) {
    c.bench_function("thin_solid_line", move |b| {
        let line = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(100.0, 1.0)];
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(1.5, &Stroke::new(0.5, Color32::RED).into(), &mut mesh);

            black_box(mesh);
        });
    });
}

fn thin_large_line_solid(c: &mut Criterion) {
    c.bench_function("thin_large_solid_line", move |b| {
        let line = (0..1000).map(|i| pos2(i as f32, 10.0)).collect::<Vec<_>>();
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(1.5, &Stroke::new(0.5, Color32::RED).into(), &mut mesh);

            black_box(mesh);
        });
    });
}

fn thick_line_uv(c: &mut Criterion) {
    c.bench_function("thick_uv_line", move |b| {
        let line = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(100.0, 1.0)];
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(
                1.5,
                &PathStroke::new_uv(2.0, |_, p| {
                    black_box(p * 2.0);
                    Color32::RED
                }),
                &mut mesh,
            );

            black_box(mesh);
        });
    });
}

fn thick_large_line_uv(c: &mut Criterion) {
    c.bench_function("thick_large_uv_line", move |b| {
        let line = (0..1000).map(|i| pos2(i as f32, 10.0)).collect::<Vec<_>>();
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(
                1.5,
                &PathStroke::new_uv(2.0, |_, p| {
                    black_box(p * 2.0);
                    Color32::RED
                }),
                &mut mesh,
            );

            black_box(mesh);
        });
    });
}

fn thin_line_uv(c: &mut Criterion) {
    c.bench_function("thin_uv_line", move |b| {
        let line = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(100.0, 1.0)];
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(
                1.5,
                &PathStroke::new_uv(2.0, |_, p| {
                    black_box(p * 2.0);
                    Color32::RED
                }),
                &mut mesh,
            );

            black_box(mesh);
        });
    });
}

fn thin_large_line_uv(c: &mut Criterion) {
    c.bench_function("thin_large_uv_line", move |b| {
        let line = (0..1000).map(|i| pos2(i as f32, 10.0)).collect::<Vec<_>>();
        let mut path = Path::default();
        path.add_open_points(&line);

        b.iter(|| {
            let mut mesh = Mesh::default();
            path.stroke_closed(
                1.5,
                &PathStroke::new_uv(2.0, |_, p| {
                    black_box(p * 2.0);
                    Color32::RED
                }),
                &mut mesh,
            );

            black_box(mesh);
        });
    });
}

criterion_group!(
    benches,
    single_dashed_lines,
    many_dashed_lines,
    tessellate_circles,
    thick_line_solid,
    thick_large_line_solid,
    thin_line_solid,
    thin_large_line_solid,
    thick_line_uv,
    thick_large_line_uv,
    thin_line_uv,
    thin_large_line_uv
);
criterion_main!(benches);
