#![allow(unsafe_code)]

use std::{borrow::Cow, collections::HashMap, num::NonZeroU32};

use egui::{epaint::Primitive, PaintCallbackInfo};
use type_map::TypeMap;
use wgpu;
use wgpu::util::DeviceExt as _;

/// A callback function that can be used to compose an [`egui::PaintCallback`] for custom WGPU
/// rendering.
///
/// The callback is composed of two functions: `prepare` and `paint`.
///
/// `prepare` is called every frame before `paint`, and can use the passed-in [`wgpu::Device`] and
/// [`wgpu::Buffer`] to allocate or modify GPU resources such as buffers.
///
/// `paint` is called after `prepare` and is given access to the the [`wgpu::RenderPass`] so that it
/// can issue draw commands.
///
/// The final argument of both the `prepare` and `paint` callbacks is a the
/// [`paint_callback_resources`][crate::renderer::RenderPass::paint_callback_resources].
/// `paint_callback_resources` has the same lifetime as the Egui render pass, so it can be used to
/// store buffers, pipelines, and other information that needs to be accessed during the render
/// pass.
///
/// # Example
///
/// See the [`custom3d_glow`](https://github.com/emilk/egui/blob/master/egui_demo_app/src/apps/custom3d_wgpu.rs) demo source for a detailed usage example.
pub struct CallbackFn {
    prepare: Box<PrepareCallback>,
    paint: Box<PaintCallback>,
}

type PrepareCallback = dyn Fn(&wgpu::Device, &wgpu::Queue, &mut TypeMap) + Sync + Send;
type PaintCallback =
    dyn for<'a, 'b> Fn(PaintCallbackInfo, &'a mut wgpu::RenderPass<'b>, &'b TypeMap) + Sync + Send;

impl Default for CallbackFn {
    fn default() -> Self {
        CallbackFn {
            prepare: Box::new(|_, _, _| ()),
            paint: Box::new(|_, _, _| ()),
        }
    }
}

impl CallbackFn {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the prepare callback
    pub fn prepare<F>(mut self, prepare: F) -> Self
    where
        F: Fn(&wgpu::Device, &wgpu::Queue, &mut TypeMap) + Sync + Send + 'static,
    {
        self.prepare = Box::new(prepare) as _;
        self
    }

    /// Set the paint callback
    pub fn paint<F>(mut self, paint: F) -> Self
    where
        F: for<'a, 'b> Fn(PaintCallbackInfo, &'a mut wgpu::RenderPass<'b>, &'b TypeMap)
            + Sync
            + Send
            + 'static,
    {
        self.paint = Box::new(paint) as _;
        self
    }
}

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
    // Uniform buffers need to be at least 16 bytes in WebGL.
    // See https://github.com/gfx-rs/wgpu/issues/2072
    _padding: [u32; 2],
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
    /// Map of egui texture IDs to textures and their associated bindgroups (texture view +
    /// sampler). The texture may be None if the TextureId is just a handle to a user-provided
    /// sampler.
    textures: HashMap<egui::TextureId, (Option<wgpu::Texture>, wgpu::BindGroup)>,
    next_user_texture_id: u64,
    /// Storage for use by [`egui::PaintCallback`]'s that need to store resources such as render
    /// pipelines that must have the lifetime of the renderpass.
    pub paint_callback_resources: type_map::TypeMap,
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
        let module = device.create_shader_module(shader);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("egui_uniform_buffer"),
            contents: bytemuck::cast_slice(&[UniformBuffer {
                screen_size_in_points: [0.0, 0.0],
                _padding: Default::default(),
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
                targets: &[Some(wgpu::ColorTargetState {
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
                })],
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
            next_user_texture_id: 0,
            paint_callback_resources: TypeMap::default(),
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
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_attachment,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_operation,
                    store: true,
                },
            })],
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
        let pixels_per_point = screen_descriptor.pixels_per_point;
        let size_in_pixels = screen_descriptor.size_in_pixels;

        // Whether or not we need to reset the renderpass state because a paint callback has just
        // run.
        let mut needs_reset = true;

        let mut index_buffers = self.index_buffers.iter();
        let mut vertex_buffers = self.vertex_buffers.iter();

        for egui::ClippedPrimitive {
            clip_rect,
            primitive,
        } in paint_jobs
        {
            if needs_reset {
                rpass.set_viewport(
                    0.0,
                    0.0,
                    size_in_pixels[0] as f32,
                    size_in_pixels[1] as f32,
                    0.0,
                    1.0,
                );
                rpass.set_pipeline(&self.render_pipeline);
                rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
                needs_reset = false;
            }

            let PixelRect {
                x,
                y,
                width,
                height,
            } = calculate_pixel_rect(clip_rect, pixels_per_point, size_in_pixels);

            // Skip rendering with zero-sized clip areas.
            if width == 0 || height == 0 {
                // If this is a mesh, we need to advance the index and vertex buffer iterators
                if let Primitive::Mesh(_) = primitive {
                    index_buffers.next().unwrap();
                    vertex_buffers.next().unwrap();
                }
                continue;
            }

            rpass.set_scissor_rect(x, y, width, height);

            match primitive {
                Primitive::Mesh(mesh) => {
                    let index_buffer = index_buffers.next().unwrap();
                    let vertex_buffer = vertex_buffers.next().unwrap();

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
                Primitive::Callback(callback) => {
                    let cbfn = if let Some(c) = callback.callback.downcast_ref::<CallbackFn>() {
                        c
                    } else {
                        // We already warned in the `prepare` callback
                        continue;
                    };

                    if callback.rect.is_positive() {
                        needs_reset = true;

                        // Set the viewport rect
                        let PixelRect {
                            x,
                            y,
                            width,
                            height,
                        } = calculate_pixel_rect(&callback.rect, pixels_per_point, size_in_pixels);
                        rpass.set_viewport(
                            x as f32,
                            y as f32,
                            width as f32,
                            height as f32,
                            0.0,
                            1.0,
                        );

                        // Set the scissor rect
                        let PixelRect {
                            x,
                            y,
                            width,
                            height,
                        } = calculate_pixel_rect(clip_rect, pixels_per_point, size_in_pixels);
                        // Skip rendering with zero-sized clip areas.
                        if width == 0 || height == 0 {
                            continue;
                        }
                        rpass.set_scissor_rect(x, y, width, height);

                        (cbfn.paint)(
                            PaintCallbackInfo {
                                viewport: callback.rect,
                                clip_rect: *clip_rect,
                                pixels_per_point,
                                screen_size_px: size_in_pixels,
                            },
                            rpass,
                            &self.paint_callback_resources,
                        );
                    }
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
            queue_write_data_to_texture(
                texture.as_ref().expect("Tried to update user texture."),
                origin,
            );
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
            let filter = match image_delta.filter {
                egui::TextureFilter::Nearest => wgpu::FilterMode::Nearest,
                egui::TextureFilter::Linear => wgpu::FilterMode::Linear,
            };
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                mag_filter: filter,
                min_filter: filter,
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
            self.textures.insert(id, (Some(texture), bind_group));
        };
    }

    pub fn free_texture(&mut self, id: &egui::TextureId) {
        self.textures.remove(id);
    }

    /// Get the WGPU texture and bind group associated to a texture that has been allocated by egui.
    ///
    /// This could be used by custom paint hooks to render images that have been added through with
    /// [`egui_extras::RetainedImage`](https://docs.rs/egui_extras/latest/egui_extras/image/struct.RetainedImage.html)
    /// or [`egui::Context::load_texture`].
    pub fn get_texture(
        &self,
        id: &egui::TextureId,
    ) -> Option<&(Option<wgpu::Texture>, wgpu::BindGroup)> {
        self.textures.get(id)
    }

    /// Registers a `wgpu::Texture` with a `egui::TextureId`.
    ///
    /// This enables the application to reference the texture inside an image ui element.
    /// This effectively enables off-screen rendering inside the egui UI. Texture must have
    /// the texture format `TextureFormat::Rgba8UnormSrgb` and
    /// Texture usage `TextureUsage::SAMPLED`.
    pub fn register_native_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
    ) -> egui::TextureId {
        self.register_native_texture_with_sampler_options(
            device,
            texture,
            wgpu::SamplerDescriptor {
                label: Some(
                    format!(
                        "egui_user_image_{}_texture_sampler",
                        self.next_user_texture_id
                    )
                    .as_str(),
                ),
                mag_filter: texture_filter,
                min_filter: texture_filter,
                ..Default::default()
            },
        )
    }

    /// Registers a `wgpu::Texture` with a `egui::TextureId` while also accepting custom
    /// `wgpu::SamplerDescriptor` options.
    ///
    /// This allows applications to specify individual minification/magnification filters as well as
    /// custom mipmap and tiling options.
    ///
    /// The `Texture` must have the format `TextureFormat::Rgba8UnormSrgb` and usage
    /// `TextureUsage::SAMPLED`. Any compare function supplied in the `SamplerDescriptor` will be
    /// ignored.
    #[allow(clippy::needless_pass_by_value)] // false positive
    pub fn register_native_texture_with_sampler_options(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler_descriptor: wgpu::SamplerDescriptor<'_>,
    ) -> egui::TextureId {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: None,
            ..sampler_descriptor
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(
                format!(
                    "egui_user_image_{}_texture_bind_group",
                    self.next_user_texture_id
                )
                .as_str(),
            ),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let id = egui::TextureId::User(self.next_user_texture_id);
        self.textures.insert(id, (None, bind_group));
        self.next_user_texture_id += 1;

        id
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
                _padding: Default::default(),
            }]),
        );

        let mut mesh_idx = 0;
        for egui::ClippedPrimitive { primitive, .. } in paint_jobs.iter() {
            match primitive {
                Primitive::Mesh(mesh) => {
                    let data: &[u8] = bytemuck::cast_slice(&mesh.indices);
                    if mesh_idx < self.index_buffers.len() {
                        self.update_buffer(device, queue, &BufferType::Index, mesh_idx, data);
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
                    if mesh_idx < self.vertex_buffers.len() {
                        self.update_buffer(device, queue, &BufferType::Vertex, mesh_idx, data);
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

                    mesh_idx += 1;
                }
                Primitive::Callback(callback) => {
                    let cbfn = if let Some(c) = callback.callback.downcast_ref::<CallbackFn>() {
                        c
                    } else {
                        tracing::warn!("Unknown paint callback: expected `egui_gpu::CallbackFn`");
                        continue;
                    };

                    (cbfn.prepare)(device, queue, &mut self.paint_callback_resources);
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

/// A Rect in physical pixel space, used for setting viewport and cliipping rectangles.
struct PixelRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Convert the Egui clip rect to a physical pixel rect we can use for the GPU viewport/scissor
fn calculate_pixel_rect(
    clip_rect: &egui::Rect,
    pixels_per_point: f32,
    target_size: [u32; 2],
) -> PixelRect {
    // Transform clip rect to physical pixels.
    let clip_min_x = pixels_per_point * clip_rect.min.x;
    let clip_min_y = pixels_per_point * clip_rect.min.y;
    let clip_max_x = pixels_per_point * clip_rect.max.x;
    let clip_max_y = pixels_per_point * clip_rect.max.y;

    // Make sure clip rect can fit within an `u32`.
    let clip_min_x = clip_min_x.clamp(0.0, target_size[0] as f32);
    let clip_min_y = clip_min_y.clamp(0.0, target_size[1] as f32);
    let clip_max_x = clip_max_x.clamp(clip_min_x, target_size[0] as f32);
    let clip_max_y = clip_max_y.clamp(clip_min_y, target_size[1] as f32);

    let clip_min_x = clip_min_x.round() as u32;
    let clip_min_y = clip_min_y.round() as u32;
    let clip_max_x = clip_max_x.round() as u32;
    let clip_max_y = clip_max_y.round() as u32;

    let width = (clip_max_x - clip_min_x).max(1);
    let height = (clip_max_y - clip_min_y).max(1);

    // Clip scissor rectangle to target size.
    let x = clip_min_x.min(target_size[0]);
    let y = clip_min_y.min(target_size[1]);
    let width = width.min(target_size[0] - x);
    let height = height.min(target_size[1] - y);

    PixelRect {
        x,
        y,
        width,
        height,
    }
}
