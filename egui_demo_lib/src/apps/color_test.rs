use egui::{color::*, widgets::color_picker::show_color, *};
use std::collections::HashMap;

const GRADIENT_SIZE: Vec2 = vec2(256.0, 24.0);

const BLACK: Color32 = Color32::BLACK;
const GREEN: Color32 = Color32::GREEN;
const RED: Color32 = Color32::RED;
const TRANSPARENT: Color32 = Color32::TRANSPARENT;
const WHITE: Color32 = Color32::WHITE;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct ColorTest {
    #[cfg_attr(feature = "persistence", serde(skip))]
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

impl epi::App for ColorTest {
    fn name(&self) -> &str {
        "🎨 Color test"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if frame.is_web() {
                ui.colored_label(
                    RED,
                    "NOTE: The WebGL backend does NOT pass the color test."
                );
                ui.small("This is because WebGL does not support a linear framebuffer blending (not even WebGL2!).\nMaybe when WebGL3 becomes mainstream in 2030 the web can finally get colors right?");
                ui.separator();
            }
            ScrollArea::auto_sized().show(ui, |ui| {
                self.ui(ui, frame.tex_allocator());
            });
        });
    }
}

impl ColorTest {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        mut tex_allocator: &mut Option<&mut dyn epi::TextureAllocator>,
    ) {
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });

        ui.label("This is made to test that the egui painter backend is set up correctly, so that all colors are interpolated and blended in linear space with premultiplied alpha.");
        ui.label("If everything is set up correctly, all groups of gradients will look uniform");

        ui.checkbox(&mut self.vertex_gradients, "Vertex gradients");
        ui.checkbox(&mut self.texture_gradients, "Texture gradients");
        ui.checkbox(&mut self.srgb, "Show naive sRGBA horror");

        ui.heading("sRGB color test");
        ui.label("Use a color picker to ensure this color is (255, 165, 0) / #ffa500");
        ui.wrap(|ui| {
            ui.style_mut().spacing.item_spacing.y = 0.0; // No spacing between gradients
            let g = Gradient::one_color(Color32::from_rgb(255, 165, 0));
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

            let tex_color = Rgba::from_rgb(1.0, 0.25, 0.25);
            let vertex_color = Rgba::from_rgb(0.5, 0.75, 0.75);

            ui.horizontal(|ui| {
                let color_size = ui.style().spacing.interact_size;
                ui.label("texture");
                show_color(ui, tex_color, color_size);
                ui.label(" * ");
                show_color(ui, vertex_color, color_size);
                ui.label(" vertex color =");
            });
            {
                let g = Gradient::one_color(Color32::from(tex_color * vertex_color));
                self.vertex_gradient(ui, "Ground truth (vertices)", WHITE, &g);
                self.tex_gradient(ui, tex_allocator, "Ground truth (texture)", WHITE, &g);
            }
            if let Some(tex_allocator) = &mut tex_allocator {
                ui.horizontal(|ui| {
                    let g = Gradient::one_color(Color32::from(tex_color));
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
            (TRANSPARENT, Color32::from_rgba_premultiplied(0, 0, 255, 0)),
        );

        ui.separator();

        pixel_test(ui);
    }

    fn show_gradients(
        &mut self,
        ui: &mut Ui,
        tex_allocator: &mut Option<&mut dyn epi::TextureAllocator>,
        bg_fill: Color32,
        (left, right): (Color32, Color32),
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
        tex_allocator: &mut Option<&mut dyn epi::TextureAllocator>,
        label: &str,
        bg_fill: Color32,
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

    fn vertex_gradient(&mut self, ui: &mut Ui, label: &str, bg_fill: Color32, gradient: &Gradient) {
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

fn vertex_gradient(ui: &mut Ui, bg_fill: Color32, gradient: &Gradient) -> Response {
    use egui::paint::*;
    let (rect, response) = ui.allocate_at_least(GRADIENT_SIZE, Sense::hover());
    if bg_fill != Default::default() {
        let mut triangles = Triangles::default();
        triangles.add_colored_rect(rect, bg_fill);
        ui.painter().add(Shape::triangles(triangles));
    }
    {
        let n = gradient.0.len();
        assert!(n >= 2);
        let mut triangles = Triangles::default();
        for (i, &color) in gradient.0.iter().enumerate() {
            let t = i as f32 / (n as f32 - 1.0);
            let x = lerp(rect.x_range(), t);
            triangles.colored_vertex(pos2(x, rect.top()), color);
            triangles.colored_vertex(pos2(x, rect.bottom()), color);
            if i < n - 1 {
                let i = i as u32;
                triangles.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
                triangles.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
            }
        }
        ui.painter().add(Shape::triangles(triangles));
    }
    response
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct Gradient(pub Vec<Color32>);

impl Gradient {
    pub fn one_color(srgba: Color32) -> Self {
        Self(vec![srgba, srgba])
    }
    pub fn texture_gradient(left: Color32, right: Color32) -> Self {
        Self(vec![left, right])
    }
    pub fn ground_truth_linear_gradient(left: Color32, right: Color32) -> Self {
        let left = Rgba::from(left);
        let right = Rgba::from(right);

        let n = 255;
        Self(
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    Color32::from(lerp(left..=right, t))
                })
                .collect(),
        )
    }
    /// This is how a bad person blends `sRGBA`
    pub fn ground_truth_bad_srgba_gradient(left: Color32, right: Color32) -> Self {
        let n = 255;
        Self(
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    Color32::from_rgba_premultiplied(
                        lerp((left[0] as f32)..=(right[0] as f32), t).round() as u8, // Don't ever do this please!
                        lerp((left[1] as f32)..=(right[1] as f32), t).round() as u8, // Don't ever do this please!
                        lerp((left[2] as f32)..=(right[2] as f32), t).round() as u8, // Don't ever do this please!
                        lerp((left[3] as f32)..=(right[3] as f32), t).round() as u8, // Don't ever do this please!
                    )
                })
                .collect(),
        )
    }

    /// Do premultiplied alpha-aware blending of the gradient on top of the fill color
    pub fn with_bg_fill(self, bg: Color32) -> Self {
        let bg = Rgba::from(bg);
        Self(
            self.0
                .into_iter()
                .map(|fg| {
                    let fg = Rgba::from(fg);
                    Color32::from(bg * (1.0 - fg.a()) + fg)
                })
                .collect(),
        )
    }

    pub fn to_pixel_row(&self) -> Vec<Color32> {
        self.0.clone()
    }
}

#[derive(Default)]
struct TextureManager(HashMap<Gradient, TextureId>);

impl TextureManager {
    fn get(
        &mut self,
        tex_allocator: &mut dyn epi::TextureAllocator,
        gradient: &Gradient,
    ) -> TextureId {
        *self.0.entry(gradient.clone()).or_insert_with(|| {
            let pixels = gradient.to_pixel_row();
            let width = pixels.len();
            let height = 1;
            tex_allocator.alloc_srgba_premultiplied((width, height), &pixels)
        })
    }
}

fn pixel_test(ui: &mut Ui) {
    ui.label("Each subsequent square should be one physical pixel larger than the previous. They should be exactly one physical pixel apart. They should be perfectly aligned to the pixel grid.");

    let pixels_per_point = ui.ctx().pixels_per_point();
    let num_squares: u32 = 8;
    let size_pixels = Vec2::new(
        ((num_squares + 1) * (num_squares + 2) / 2) as f32,
        num_squares as f32,
    );
    let size_points = size_pixels / pixels_per_point + Vec2::splat(2.0);
    let (response, painter) = ui.allocate_painter(size_points, Sense::hover());

    let mut cursor_pixel = Pos2::new(
        response.rect.min.x * pixels_per_point,
        response.rect.min.y * pixels_per_point,
    )
    .ceil();
    for size in 1..=num_squares {
        let rect_points = Rect::from_min_size(
            Pos2::new(
                cursor_pixel.x / pixels_per_point,
                cursor_pixel.y / pixels_per_point,
            ),
            Vec2::splat(size as f32) / pixels_per_point,
        );
        painter.rect_filled(rect_points, 0.0, egui::Color32::WHITE);
        cursor_pixel.x += (1 + size) as f32;
    }
}
