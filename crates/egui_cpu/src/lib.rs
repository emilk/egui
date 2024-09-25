mod pipeline;

use egui::ahash::HashMap;
use egui::epaint::Primitive;
use egui::{ClippedPrimitive, ColorImage, ImageData, TextureId, TexturesDelta, Vec2};
use euc::buffer::Buffer2d;
use euc::{IndexedVertices, Pipeline, Sampler, Texture};
use image::{DynamicImage, ImageBuffer, Pixel, RgbaImage};
use std::ops::Deref;
use vek::Rgba;

#[derive(Debug, Default)]
pub struct Renderer {
    textures: HashMap<TextureId, Buffer2d<image::Rgba<u8>>>,
}

impl Renderer {
    pub fn update_textures(&mut self, delta: TexturesDelta) {
        for (id, delta) in delta.set {
            dbg!(delta.image.size());
            let image = match delta.image {
                ImageData::Color(color) => RgbaImage::from_raw(
                    color.width() as u32,
                    color.height() as u32,
                    Vec::from(color.deref().as_raw()),
                )
                .unwrap(),
                ImageData::Font(font) => {
                    let color_image = ColorImage {
                        size: font.size,
                        pixels: font.srgba_pixels(None).collect(),
                    };

                    RgbaImage::from_raw(
                        font.width() as u32,
                        font.height() as u32,
                        Vec::from(color_image.as_raw()),
                    )
                    .unwrap()
                }
            };

            let buffer = Buffer2d::from_texture(&DynamicImage::from(image).to_rgba8());
            self.textures.insert(id, buffer);

            if delta.pos.is_some() {
                unimplemented!()
            }
        }
    }

    pub fn render(
        &self,
        primitives: &[ClippedPrimitive],
        resolution: Vec2,
        dpi: f32,
    ) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let width = (resolution.x * dpi) as usize;
        let height = (resolution.y * dpi) as usize;

        dbg!(width, height);

        let mut output = Buffer2d::fill([width, height], 0x000000);
        let mut depth = Buffer2d::fill([width, height], 1.0);

        for ClippedPrimitive {
            primitive,
            clip_rect,
        } in primitives
        {
            match primitive {
                Primitive::Mesh(mesh) => {
                    let texture = self.textures.get(&mesh.texture_id).unwrap();

                    let sampler = texture
                        .map(|pixel| Rgba::from(pixel.0).map(|e: u8| e as f32))
                        .linear();
                    // let sampler = DebugSampler {
                    //     texture: Buffer2d::fill([1, 1], 0.0),
                    // };

                    // let sampler = |pos| Rgba::new(0, 0, 0, 0);

                    let mut pipeline = pipeline::EguiPipeline {
                        screen_size: vek::Vec2::new(width as f32, height as f32) / dpi,
                        scissor_rect: Default::default(),
                        sampler: &sampler,
                    };

                    pipeline.scissor_rect = vek::Rect::new(
                        clip_rect.min.x,
                        clip_rect.min.y,
                        clip_rect.width(),
                        clip_rect.height(),
                    );

                    pipeline.render(
                        mesh.indices.iter().map(|&i| mesh.vertices[i as usize]),
                        &mut output,
                        &mut depth,
                    );
                }
                Primitive::Callback(_) => {
                    println!("Callback not implemented");
                }
            }
        }

        let raw = output.raw();
        let raw = Vec::from(bytemuck::cast_slice(&raw));
        let image = RgbaImage::from_raw(width as u32, height as u32, raw).unwrap();
        image
    }
}

struct DebugSampler {
    texture: Buffer2d<f32>,
}
impl Sampler<2> for DebugSampler {
    type Index = f32;
    type Sample = Rgba<f32>;
    type Texture = Buffer2d<f32>;

    fn raw_texture(&self) -> &Self::Texture {
        &self.texture
    }

    fn sample(&self, index: [Self::Index; 2]) -> Self::Sample {
        if index[0] != 0.0 || index[1] != 0.0 {
            dbg!(index);
        }
        Rgba::new(0.0, 0.0, 0.0, 0.0)
    }
}
