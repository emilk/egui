use crate::layers::ShapeIdx;
use crate::{ClippedPrimitive, Context, Mesh, Painter, Pos2, Shape};
use emath::Rot2;
use epaint::{ClippedShape, Primitive};

pub(crate) trait PainterExt {
    fn add_rotated(&self, shape: impl Into<Shape>, rot: Rot2, origin: Pos2) -> ShapeIdx;
}

impl PainterExt for Painter {
    fn add_rotated(&self, shape: impl Into<Shape>, rot: Rot2, origin: Pos2) -> ShapeIdx {
        let clip_rect = self.clip_rect();
        let shape = shape.into();
        let mut mesh = tesselate(self.ctx(), ClippedShape { clip_rect, shape });
        mesh.rotate(rot, origin);
        self.add(mesh)
    }
}

pub(crate) fn tesselate(ctx: &Context, shape: ClippedShape) -> Mesh {
    let primitives = ctx.tessellate(vec![shape], ctx.pixels_per_point());
    match primitives.into_iter().next() {
        Some(ClippedPrimitive {
            primitive: Primitive::Mesh(mesh),
            ..
        }) => mesh,
        Some(_) => unreachable!(),
        None => Mesh::default(),
    }
}
