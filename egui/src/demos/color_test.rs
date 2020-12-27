use crate::widgets::color_picker::show_color;
use crate::*;
use color::*;
use std::collections::HashMap;

const GRADIENT_SIZE: Vec2 = vec2(256.0, 24.0);

pub struct ColorTest {
    tex_mngr: TextureManager,
    vertex_gradients: bool,
    texture_gradients: bool,
    srgb: bool,
}

impl Default for ColorTest {
    fn default() -> Self {
        Self {
            tex_mngr: Default::default(),
            vertex_gradients: true,
            texture_gradients: true,
            srgb: false,
        }
    }
}

impl ColorTest {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        mut tex_allocator: &mut Option<&mut dyn app::TextureAllocator>,
    ) {
        ui.label("This is made to test if your Egui painter backend is set up correctly");
        ui.label("It is meant to ensure you do proper sRGBA decoding of both texture and vertex colors, and blend using premultiplied alpha.");
        ui.label("If everything is set up correctly, all groups of gradients will look uniform");

        ui.checkbox(&mut self.vertex_gradients, "Vertex gradients");
        ui.checkbox(&mut self.texture_gradients, "Texture gradients");
        ui.checkbox(&mut self.srgb, "Show naive sRGBA horror");

        ui.heading("sRGB color test");
        ui.label("Use a color picker to ensure this color is (255, 165, 0) / #ffa500");
        ui.wrap(|ui| {
            ui.style_mut().spacing.item_spacing.y = 0.0; // No spacing between gradients
            let g = Gradient::one_color(Srgba::from_rgb(255, 165, 0));
            self.vertex_gradient(ui, "orange rgb(255, 165, 0) - vertex", WHITE, &g);
            self.tex_gradient(
                ui,
                tex_allocator,
                "orange rgb(255, 165, 0) - texture",
                WHITE,
                &g,
            );
        });

        ui.separator();

        ui.label("Test that vertex color times texture color is done in linear space:");
        ui.wrap(|ui| {
            ui.style_mut().spacing.item_spacing.y = 0.0; // No spacing between gradients

            let tex_color = Rgba::new(1.0, 0.25, 0.25, 1.0);
            let vertex_color = Rgba::new(0.5, 0.75, 0.75, 1.0);

            ui.horizontal(|ui| {
                let color_size = ui.style().spacing.interact_size;
                ui.label("texture");
                show_color(ui, tex_color, color_size);
                ui.label(" * ");
                show_color(ui, vertex_color, color_size);
                ui.label(" vertex color =");
            });
            {
                let g = Gradient::one_color(Srgba::from(tex_color * vertex_color));
                self.vertex_gradient(ui, "Ground truth (vertices)", WHITE, &g);
                self.tex_gradient(ui, tex_allocator, "Ground truth (texture)", WHITE, &g);
            }
            if let Some(tex_allocator) = &mut tex_allocator {
                ui.horizontal(|ui| {
                    let g = Gradient::one_color(Srgba::from(tex_color));
                    let tex = self.tex_mngr.get(*tex_allocator, &g);
                    let texel_offset = 0.5 / (g.0.len() as f32);
                    let uv =
                        Rect::from_min_max(pos2(texel_offset, 0.0), pos2(1.0 - texel_offset, 1.0));
                    ui.add(Image::new(tex, GRADIENT_SIZE).tint(vertex_color).uv(uv))
                        .on_hover_text(format!("A texture that is {} texels wide", g.0.len()));
                    ui.label("GPU result");
                });
            }
        });

        ui.separator();

        // TODO: test color multiplication (image tint),
        // to make sure vertex and texture color multiplication is done in linear space.

        self.show_gradients(ui, tex_allocator, WHITE, (RED, GREEN));
        if self.srgb {
            ui.label("Notice the darkening in the center of the naive sRGB interpolation.");
        }

        ui.separator();

        self.show_gradients(ui, tex_allocator, RED, (TRANSPARENT, GREEN));

        ui.separator();

        self.show_gradients(ui, tex_allocator, WHITE, (TRANSPARENT, GREEN));
        if self.srgb {
            ui.label(
            "Notice how the linear blend stays green while the naive sRGBA interpolation looks gray in the middle.",
        );
        }

        ui.separator();

        // TODO: another ground truth where we do the alpha-blending against the background also.
        // TODO: exactly the same thing, but with vertex colors (no textures)
        self.show_gradients(ui, tex_allocator, WHITE, (TRANSPARENT, BLACK));
        ui.separator();
        self.show_gradients(ui, tex_allocator, BLACK, (TRANSPARENT, WHITE));
        ui.separator();

        ui.label("Additive blending: add more and more blue to the red background:");
        self.show_gradients(
            ui,
            tex_allocator,
            RED,
            (TRANSPARENT, Srgba::from_rgba_premultiplied(0, 0, 255, 0)),
        );

        ui.separator();
    }

    fn show_gradients(
        &mut self,
        ui: &mut Ui,
        tex_allocator: &mut Option<&mut dyn app::TextureAllocator>,
        bg_fill: Srgba,
        (left, right): (Srgba, Srgba),
    ) {
        let is_opaque = left.is_opaque() && right.is_opaque();

        ui.horizontal(|ui| {
            let color_size = ui.style().spacing.interact_size;
            if !is_opaque {
                ui.label("Background:");
                show_color(ui, bg_fill, color_size);
            }
            ui.label("gradient");
            show_color(ui, left, color_size);
            ui.label("-");
            show_color(ui, right, color_size);
        });

        ui.wrap(|ui| {
            ui.style_mut().spacing.item_spacing.y = 0.0; // No spacing between gradients
            if is_opaque {
                let g = Gradient::ground_truth_linear_gradient(left, right);
                self.vertex_gradient(ui, "Ground Truth (CPU gradient) - vertices", bg_fill, &g);
                self.tex_gradient(
                    ui,
                    tex_allocator,
                    "Ground Truth (CPU gradient) - texture",
                    bg_fill,
                    &g,
                );
            } else {
                let g = Gradient::ground_truth_linear_gradient(left, right).with_bg_fill(bg_fill);
                self.vertex_gradient(
                    ui,
                    "Ground Truth (CPU gradient, CPU blending) - vertices",
                    bg_fill,
                    &g,
                );
                self.tex_gradient(
                    ui,
                    tex_allocator,
                    "Ground Truth (CPU gradient, CPU blending) - texture",
                    bg_fill,
                    &g,
                );
                let g = Gradient::ground_truth_linear_gradient(left, right);
                self.vertex_gradient(ui, "CPU gradient, GPU blending - vertices", bg_fill, &g);
                self.tex_gradient(
                    ui,
                    tex_allocator,
                    "CPU gradient, GPU blending - texture",
                    bg_fill,
                    &g,
                );
            }

            let g = Gradient::texture_gradient(left, right);
            self.vertex_gradient(
                ui,
                "Triangle mesh of width 2 (test vertex decode and interpolation)",
                bg_fill,
                &g,
            );
            self.tex_gradient(
                ui,
                tex_allocator,
                "Texture of width 2 (test texture sampler)",
                bg_fill,
                &g,
            );

            if self.srgb {
                let g =
                    Gradient::ground_truth_bad_srgba_gradient(left, right).with_bg_fill(bg_fill);
                self.vertex_gradient(
                    ui,
                    "Triangle mesh with naive sRGBA interpolation (WRONG)",
                    bg_fill,
                    &g,
                );
                self.tex_gradient(
                    ui,
                    tex_allocator,
                    "Naive sRGBA interpolation (WRONG)",
                    bg_fill,
                    &g,
                );
            }
        });
    }

    fn tex_gradient(
        &mut self,
        ui: &mut Ui,
        tex_allocator: &mut Option<&mut dyn app::TextureAllocator>,
        label: &str,
        bg_fill: Srgba,
        gradient: &Gradient,
    ) {
        if !self.texture_gradients {
            return;
        }
        if let Some(tex_allocator) = tex_allocator {
            ui.horizontal(|ui| {
                let tex = self.tex_mngr.get(*tex_allocator, gradient);
                let texel_offset = 0.5 / (gradient.0.len() as f32);
                let uv = Rect::from_min_max(pos2(texel_offset, 0.0), pos2(1.0 - texel_offset, 1.0));
                ui.add(Image::new(tex, GRADIENT_SIZE).bg_fill(bg_fill).uv(uv))
                    .on_hover_text(format!(
                        "A texture that is {} texels wide",
                        gradient.0.len()
                    ));
                ui.label(label);
            });
        }
    }

    fn vertex_gradient(&mut self, ui: &mut Ui, label: &str, bg_fill: Srgba, gradient: &Gradient) {
        if !self.vertex_gradients {
            return;
        }
        ui.horizontal(|ui| {
            vertex_gradient(ui, bg_fill, gradient).on_hover_text(format!(
                "A triangle mesh that is {} vertices wide",
                gradient.0.len()
            ));
            ui.label(label);
        });
    }
}

fn vertex_gradient(ui: &mut Ui, bg_fill: Srgba, gradient: &Gradient) -> Response {
    use crate::paint::*;
    let response = ui.allocate_response(GRADIENT_SIZE, Sense::hover());
    if bg_fill != Default::default() {
        let mut triangles = Triangles::default();
        triangles.add_colored_rect(response.rect, bg_fill);
        ui.painter().add(PaintCmd::triangles(triangles));
    }
    {
        let n = gradient.0.len();
        assert!(n >= 2);
        let mut triangles = Triangles::default();
        for (i, &color) in gradient.0.iter().enumerate() {
            let t = i as f32 / (n as f32 - 1.0);
            let x = lerp(response.rect.x_range(), t);
            triangles.colored_vertex(pos2(x, response.rect.top()), color);
            triangles.colored_vertex(pos2(x, response.rect.bottom()), color);
            if i < n - 1 {
                let i = i as u32;
                triangles.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
                triangles.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
            }
        }
        ui.painter().add(PaintCmd::triangles(triangles));
    }
    response
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct Gradient(pub Vec<Srgba>);

impl Gradient {
    pub fn one_color(srgba: Srgba) -> Self {
        Self(vec![srgba, srgba])
    }
    pub fn texture_gradient(left: Srgba, right: Srgba) -> Self {
        Self(vec![left, right])
    }
    pub fn ground_truth_linear_gradient(left: Srgba, right: Srgba) -> Self {
        let left = Rgba::from(left);
        let right = Rgba::from(right);

        let n = 255;
        Self(
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    Srgba::from(lerp(left..=right, t))
                })
                .collect(),
        )
    }
    /// This is how a bad person blends `sRGBA`
    pub fn ground_truth_bad_srgba_gradient(left: Srgba, right: Srgba) -> Self {
        let n = 255;
        Self(
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    Srgba([
                        lerp((left[0] as f32)..=(right[0] as f32), t).round() as u8, // Don't ever do this please!
                        lerp((left[1] as f32)..=(right[1] as f32), t).round() as u8, // Don't ever do this please!
                        lerp((left[2] as f32)..=(right[2] as f32), t).round() as u8, // Don't ever do this please!
                        lerp((left[3] as f32)..=(right[3] as f32), t).round() as u8, // Don't ever do this please!
                    ])
                })
                .collect(),
        )
    }

    /// Do premultiplied alpha-aware blending of the gradient on top of the fill color
    pub fn with_bg_fill(self, bg: Srgba) -> Self {
        let bg = Rgba::from(bg);
        Self(
            self.0
                .into_iter()
                .map(|fg| {
                    let fg = Rgba::from(fg);
                    Srgba::from(bg * (1.0 - fg.a()) + fg)
                })
                .collect(),
        )
    }

    pub fn to_pixel_row(&self) -> Vec<Srgba> {
        self.0.clone()
    }
}

#[derive(Default)]
struct TextureManager(HashMap<Gradient, TextureId>);

impl TextureManager {
    fn get(
        &mut self,
        tex_allocator: &mut dyn app::TextureAllocator,
        gradient: &Gradient,
    ) -> TextureId {
        *self.0.entry(gradient.clone()).or_insert_with(|| {
            let pixels = gradient.to_pixel_row();
            let width = pixels.len();
            let height = 1;
            let id = tex_allocator.alloc();
            tex_allocator.set_srgba_premultiplied(id, (width, height), &pixels);
            id
        })
    }
}
