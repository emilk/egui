#![allow(unsafe_code)]

use std::{borrow::Cow, collections::HashMap, num::NonZeroU32};

use egui::epaint::Primitive;
use wgpu;
use wgpu::util::DeviceExt as _;

/// Enum for selecting the right buffer type.
#[derive(Debug)]
enum BufferType {
    Uniform,
    Index,
    Vertex,
}

/// Information about the screen used for rendering.
pub struct ScreenDescriptor {
    /// Size of the window in physical pixels.
    pub size_in_pixels: [u32; 2],

    /// HiDPI scale factor (pixels per point).
    pub pixels_per_point: f32,
}

impl ScreenDescriptor {
    /// size in "logical" points
    fn screen_size_in_points(&self) -> [f32; 2] {
        [
            self.size_in_pixels[0] as f32 / self.pixels_per_point,
            self.size_in_pixels[1] as f32 / self.pixels_per_point,
        ]
    }
}

/// Uniform buffer used when rendering.
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct UniformBuffer {
    screen_size_in_points: [f32; 2],
}

/// Wraps the buffers and includes additional information.
#[derive(Debug)]
struct SizedBuffer {
    buffer: wgpu::Buffer,
    /// number of bytes
    size: usize,
}

/// Render pass to render a egui based GUI.
pub struct RenderPass {
    render_pipeline: wgpu::RenderPipeline,
    index_buffers: Vec<SizedBuffer>,
    vertex_buffers: Vec<SizedBuffer>,
    uniform_buffer: SizedBuffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    textures: HashMap<egui::TextureId, (wgpu::Texture, wgpu::BindGroup)>,
}

impl RenderPass {
    /// Creates a new render pass to render a egui UI.
    ///
    /// If the format passed is not a *Srgb format, the shader will automatically convert to `sRGB` colors in the shader.
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        msaa_samples: u32,
    ) -> Self {
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("egui_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("egui.wgsl"))),
        };
        let module = device.create_shader_module(&shader);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("egui_uniform_buffer"),
            contents: bytemuck::cast_slice(&[UniformBuffer {
                screen_size_in_points: [0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_buffer = SizedBuffer {
            buffer: uniform_buffer,
            size: std::mem::size_of::<UniformBuffer>(),
        };

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui_uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: None,
                        ty: wgpu::BufferBindingType::Uniform,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer.buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui_texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("egui_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("egui_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                entry_point: if output_format.describe().srgb {
                    "vs_main"
                } else {
                    "vs_conv_main"
                },
                module: &module,
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 5 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    // 0: vec2 position
                    // 1: vec2 texture coordinates
                    // 2: uint color
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                unclipped_depth: false,
                conservative: false,
                cull_mode: None,
                front_face: wgpu::FrontFace::default(),
                polygon_mode: wgpu::PolygonMode::default(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                count: msaa_samples,
                mask: !0,
            },

            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            multiview: None,
        });

        Self {
            render_pipeline,
            vertex_buffers: Vec::with_capacity(64),
            index_buffers: Vec::with_capacity(64),
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            textures: HashMap::new(),
        }
    }

    /// Executes the egui render pass.
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_attachment: &wgpu::TextureView,
        paint_jobs: &[egui::epaint::ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
        clear_color: Option<wgpu::Color>,
    ) {
        let load_operation = if let Some(color) = clear_color {
            wgpu::LoadOp::Clear(color)
        } else {
            wgpu::LoadOp::Load
        };

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: color_attachment,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_operation,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            label: Some("egui main render pass"),
        });
        rpass.push_debug_group("egui_pass");

        self.execute_with_renderpass(&mut rpass, paint_jobs, screen_descriptor);

        rpass.pop_debug_group();
    }

    /// Executes the egui render pass onto an existing wgpu renderpass.
    pub fn execute_with_renderpass<'rpass>(
        &'rpass self,
        rpass: &mut wgpu::RenderPass<'rpass>,
        paint_jobs: &[egui::epaint::ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
    ) {
        rpass.set_pipeline(&self.render_pipeline);

        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);

        let pixels_per_point = screen_descriptor.pixels_per_point;
        let size_in_pixels = screen_descriptor.size_in_pixels;

        for (
            (
                egui::ClippedPrimitive {
                    clip_rect,
                    primitive,
                },
                vertex_buffer,
            ),
            index_buffer,
        ) in paint_jobs
            .iter()
            .zip(&self.vertex_buffers)
            .zip(&self.index_buffers)
        {
            // Transform clip rect to physical pixels.
            let clip_min_x = pixels_per_point * clip_rect.min.x;
            let clip_min_y = pixels_per_point * clip_rect.min.y;
            let clip_max_x = pixels_per_point * clip_rect.max.x;
            let clip_max_y = pixels_per_point * clip_rect.max.y;

            // Make sure clip rect can fit within an `u32`.
            let clip_min_x = clip_min_x.clamp(0.0, size_in_pixels[0] as f32);
            let clip_min_y = clip_min_y.clamp(0.0, size_in_pixels[1] as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, size_in_pixels[0] as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, size_in_pixels[1] as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            let width = (clip_max_x - clip_min_x).max(1);
            let height = (clip_max_y - clip_min_y).max(1);

            {
                // Clip scissor rectangle to target size.
                let x = clip_min_x.min(size_in_pixels[0]);
                let y = clip_min_y.min(size_in_pixels[1]);
                let width = width.min(size_in_pixels[0] - x);
                let height = height.min(size_in_pixels[1] - y);

                // Skip rendering with zero-sized clip areas.
                if width == 0 || height == 0 {
                    continue;
                }

                rpass.set_scissor_rect(x, y, width, height);
            }

            match primitive {
                Primitive::Mesh(mesh) => {
                    if let Some((_texture, bind_group)) = self.textures.get(&mesh.texture_id) {
                        rpass.set_bind_group(1, bind_group, &[]);
                        rpass.set_index_buffer(
                            index_buffer.buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        rpass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
                        rpass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                    } else {
                        tracing::warn!("Missing texture: {:?}", mesh.texture_id);
                    }
                }
                Primitive::Callback(_) => {
                    // already warned about earlier
                }
            }
        }
    }

    /// Should be called before `execute()`.
    pub fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: egui::TextureId,
        image_delta: &egui::epaint::ImageDelta,
    ) {
        let width = image_delta.image.width() as u32;
        let height = image_delta.image.height() as u32;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let data_color32 = match &image_delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    width as usize * height as usize,
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                Cow::Borrowed(&image.pixels)
            }
            egui::ImageData::Font(image) => {
                assert_eq!(
                    width as usize * height as usize,
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                Cow::Owned(image.srgba_pixels(1.0).collect::<Vec<_>>())
            }
        };
        let data_bytes: &[u8] = bytemuck::cast_slice(data_color32.as_slice());

        let queue_write_data_to_texture = |texture, origin| {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin,
                    aspect: wgpu::TextureAspect::All,
                },
                data_bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(4 * width),
                    rows_per_image: NonZeroU32::new(height),
                },
                size,
            );
        };

        if let Some(pos) = image_delta.pos {
            // update the existing texture
            let (texture, _bind_group) = self
                .textures
                .get(&id)
                .expect("Tried to update a texture that has not been allocated yet.");
            let origin = wgpu::Origin3d {
                x: pos[0] as u32,
                y: pos[1] as u32,
                z: 0,
            };
            queue_write_data_to_texture(texture, origin);
        } else {
            // allocate a new texture
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            });
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let origin = wgpu::Origin3d::ZERO;
            queue_write_data_to_texture(&texture, origin);
            self.textures.insert(id, (texture, bind_group));
        };
    }

    /// Should be called before `execute()`.
    pub fn free_texture(&mut self, id: &egui::TextureId) {
        self.textures.remove(id);
    }

    /// Uploads the uniform, vertex and index data used by the render pass.
    /// Should be called before `execute()`.
    pub fn update_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        paint_jobs: &[egui::epaint::ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let screen_size_in_points = screen_descriptor.screen_size_in_points();

        self.update_buffer(
            device,
            queue,
            &BufferType::Uniform,
            0,
            bytemuck::cast_slice(&[UniformBuffer {
                screen_size_in_points,
            }]),
        );

        for (i, egui::ClippedPrimitive { primitive, .. }) in paint_jobs.iter().enumerate() {
            match primitive {
                Primitive::Mesh(mesh) => {
                    let data: &[u8] = bytemuck::cast_slice(&mesh.indices);
                    if i < self.index_buffers.len() {
                        self.update_buffer(device, queue, &BufferType::Index, i, data);
                    } else {
                        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("egui_index_buffer"),
                            contents: data,
                            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        });
                        self.index_buffers.push(SizedBuffer {
                            buffer,
                            size: data.len(),
                        });
                    }

                    let data: &[u8] = bytemuck::cast_slice(&mesh.vertices);
                    if i < self.vertex_buffers.len() {
                        self.update_buffer(device, queue, &BufferType::Vertex, i, data);
                    } else {
                        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("egui_vertex_buffer"),
                            contents: data,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });

                        self.vertex_buffers.push(SizedBuffer {
                            buffer,
                            size: data.len(),
                        });
                    }
                }
                Primitive::Callback(_) => {
                    tracing::warn!("Painting callbacks not supported by egui-wgpu (yet)");
                }
            }
        }
    }

    /// Updates the buffers used by egui. Will properly re-size the buffers if needed.
    fn update_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer_type: &BufferType,
        index: usize,
        data: &[u8],
    ) {
        let (buffer, storage, label) = match buffer_type {
            BufferType::Index => (
                &mut self.index_buffers[index],
                wgpu::BufferUsages::INDEX,
                "egui_index_buffer",
            ),
            BufferType::Vertex => (
                &mut self.vertex_buffers[index],
                wgpu::BufferUsages::VERTEX,
                "egui_vertex_buffer",
            ),
            BufferType::Uniform => (
                &mut self.uniform_buffer,
                wgpu::BufferUsages::UNIFORM,
                "egui_uniform_buffer",
            ),
        };

        if data.len() > buffer.size {
            buffer.size = data.len();
            buffer.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: storage | wgpu::BufferUsages::COPY_DST,
            });
        } else {
            queue.write_buffer(&buffer.buffer, 0, data);
        }
    }
}
