use derive_more::{Add, Mul};
use euc::primitives::PrimitiveKind;
use euc::rasterizer::{Rasterizer, Triangles};
use euc::{
    AaMode, CoordinateMode, CullMode, DepthMode, Pipeline, PixelMode, Sampler, Texture,
    TriangleList,
};
use vek::{Rgba, Vec2, Vec4};

pub(crate) struct EguiPipeline<S> {
    pub screen_size: Vec2<f32>,
    pub scissor_rect: vek::Rect<f32, f32>,
    pub sampler: S,
}

#[derive(Debug, Clone, Add, Mul)]
pub(crate) struct VertexData {
    text_coord: Vec2<f32>,
    color: Rgba<f32>,
}

impl<'r, S: Sampler<2, Index = f32, Sample = Rgba<f32>>> Pipeline<'r> for EguiPipeline<S> {
    type Vertex = egui::epaint::Vertex;
    type VertexData = VertexData;
    type Primitives = TriangleList;
    type Fragment = Rgba<f32>;
    type Pixel = u32;

    fn coordinate_mode(&self) -> CoordinateMode {
        CoordinateMode::VULKAN
    }

    fn pixel_mode(&self) -> PixelMode {
        PixelMode::WRITE
    }

    fn vertex(&self, vertex: &Self::Vertex) -> ([f32; 4], Self::VertexData) {
        let position =
            position_from_screen(Vec2::new(vertex.pos.x, vertex.pos.y), self.screen_size);
        let text_coord = Vec2::new(vertex.uv.x, vertex.uv.y);
        let color = Rgba::new(
            vertex.color.r() as f32 / 255.0,
            vertex.color.g() as f32 / 255.0,
            vertex.color.b() as f32 / 255.0,
            vertex.color.a() as f32 / 255.0,
        );
        (position.into_array(), VertexData { text_coord, color })
    }

    fn fragment(&self, vs_out: Self::VertexData) -> Self::Fragment {
        vs_out.color * self.sampler.sample(vs_out.text_coord.into_array())
    }

    fn blend(&self, old: Self::Pixel, new: Self::Fragment) -> Self::Pixel {
        //Source over
        let source = new;
        let dest = Rgba::from(old.to_le_bytes()).map(|c: u8| c as f32);

        let source_alpha = source.a / 255.0;
        let inv_source_alpha = 1.0 - source_alpha;

        let r = source.r + dest.r * inv_source_alpha;
        let g = source.g + dest.g * inv_source_alpha;
        let b = source.b + dest.b * inv_source_alpha;
        let a = source.a + dest.a * inv_source_alpha;
        u32::from_le_bytes(Rgba::new(r, g, b, a).map(|c| (c) as u8).into_array())

        // u32::from_le_bytes(new.map(|c| c as u8).into_array())
    }

    fn rasterizer_config(
        &self,
    ) -> <<Self::Primitives as PrimitiveKind<Self::VertexData>>::Rasterizer as Rasterizer>::Config
    {
        CullMode::None
    }

    fn aa_mode(&self) -> AaMode {
        AaMode::None
    }
}

// From wgsl shader:
// fn unpack_color(color: u32) -> vec4<f32> {
//     return vec4<f32>(
//         f32(color & 255u),
//         f32((color >> 8u) & 255u),
//         f32((color >> 16u) & 255u),
//         f32((color >> 24u) & 255u),
//     ) / 255.0;
// }
// fn unpack_color(color: [u8; 4]) -> Vec4<f32> {
//     Vec4::new(
//         (color & 255u32) as f32,
//         ((color >> 8u32) & 255u32) as f32,
//         ((color >> 16u32) & 255u32) as f32,
//         ((color >> 24u32) & 255u32) as f32,
//     ) / 255.0
// }

// From wgsl shader:
// fn position_from_screen(screen_pos: vec2<f32>) -> vec4<f32> {
//     return vec4<f32>(
//         2.0 * screen_pos.x / r_locals.screen_size.x - 1.0,
//         1.0 - 2.0 * screen_pos.y / r_locals.screen_size.y,
//         0.0,
//         1.0,
//     );
// }
fn position_from_screen(screen_pos: Vec2<f32>, screen_size: Vec2<f32>) -> Vec4<f32> {
    Vec4::new(
        2.0 * screen_pos.x / screen_size.x - 1.0,
        1.0 - 2.0 * screen_pos.y / screen_size.y,
        0.0,
        1.0,
    )
}
