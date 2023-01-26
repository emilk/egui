#![allow(unsafe_code)]

use std::num::NonZeroU64;
use std::ops::Range;
use std::{borrow::Cow, collections::HashMap, num::NonZeroU32};

use type_map::concurrent::TypeMap;
use wgpu;
use wgpu::util::DeviceExt as _;

use epaint::{emath::NumExt, PaintCallbackInfo, Primitive, Vertex};

/// A callback function that can be used to compose an [`epaint::PaintCallback`] for custom WGPU
/// rendering.
///
/// The callback is composed of two functions: `prepare` and `paint`:
/// - `prepare` is called every frame before `paint`, and can use the passed-in
///   [`wgpu::Device`] and [`wgpu::Buffer`] to allocate or modify GPU resources such as buffers.
/// - `paint` is called after `prepare` and is given access to the [`wgpu::RenderPass`] so
///   that it can issue draw commands into the same [`wgpu::RenderPass`] that is used for
///   all other egui elements.
///
/// The final argument of both the `prepare` and `paint` callbacks is a the
/// [`paint_callback_resources`][crate::renderer::Renderer::paint_callback_resources].
/// `paint_callback_resources` has the same lifetime as the Egui render pass, so it can be used to
/// store buffers, pipelines, and other information that needs to be accessed during the render
/// pass.
///
/// # Example
///
/// See the [`custom3d_wgpu`](https://github.com/emilk/egui/blob/master/crates/egui_demo_app/src/apps/custom3d_wgpu.rs) demo source for a detailed usage example.
pub struct CallbackFn {
    prepare: Box<PrepareCallback>,
    paint: Box<PaintCallback>,
}

type PrepareCallback = dyn Fn(
        &wgpu::Device,
        &wgpu::Queue,
        &mut wgpu::CommandEncoder,
        &mut TypeMap,
    ) -> Vec<wgpu::CommandBuffer>
    + Sync
    + Send;

type PaintCallback =
    dyn for<'a, 'b> Fn(PaintCallbackInfo, &'a mut wgpu::RenderPass<'b>, &'b TypeMap) + Sync + Send;

impl Default for CallbackFn {
    fn default() -> Self {
        CallbackFn {
            prepare: Box::new(|_, _, _, _| Vec::new()),
            paint: Box::new(|_, _, _| ()),
        }
    }
}

impl CallbackFn {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the prepare callback.
    ///
    /// The passed-in `CommandEncoder` is egui's and can be used directly to register
    /// wgpu commands for simple use cases.
    /// This allows reusing the same [`wgpu::CommandEncoder`] for all callbacks and egui
    /// rendering itself.
    ///
    /// For more complicated use cases, one can also return a list of arbitrary
    /// `CommandBuffer`s and have complete control over how they get created and fed.
    /// In particular, this gives an opportunity to parallelize command registration and
    /// prevents a faulty callback from poisoning the main wgpu pipeline.
    ///
    /// When using eframe, the main egui command buffer, as well as all user-defined
    /// command buffers returned by this function, are guaranteed to all be submitted
    /// at once in a single call.
    pub fn prepare<F>(mut self, prepare: F) -> Self
    where
        F: Fn(
                &wgpu::Device,
                &wgpu::Queue,
                &mut wgpu::CommandEncoder,
                &mut TypeMap,
            ) -> Vec<wgpu::CommandBuffer>
            + Sync
            + Send
            + 'static,
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

struct SlicedBuffer {
    buffer: wgpu::Buffer,
    slices: Vec<Range<wgpu::BufferAddress>>,
    capacity: wgpu::BufferAddress,
}

/// Renderer for a egui based GUI.
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,

    index_buffer: SlicedBuffer,
    vertex_buffer: SlicedBuffer,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    /// Map of egui texture IDs to textures and their associated bindgroups (texture view +
    /// sampler). The texture may be None if the TextureId is just a handle to a user-provided
    /// sampler.
    textures: HashMap<epaint::TextureId, (Option<wgpu::Texture>, wgpu::BindGroup)>,
    next_user_texture_id: u64,
    samplers: HashMap<epaint::textures::TextureOptions, wgpu::Sampler>,

    /// Storage for use by [`epaint::PaintCallback`]'s that need to store resources such as render
    /// pipelines that must have the lifetime of the renderpass.
    pub paint_callback_resources: TypeMap,
}

impl Renderer {
    /// Creates a renderer for a egui UI.
    ///
    /// `output_color_format` should preferably be [`wgpu::TextureFormat::Rgba8Unorm`] or
    /// [`wgpu::TextureFormat::Bgra8Unorm`], i.e. in gamma-space.
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        output_depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
    ) -> Self {
        crate::profile_function!();

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("egui"),
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

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui_uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<UniformBuffer>() as _),
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
                    buffer: &uniform_buffer,
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

        let depth_stencil = output_depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("egui_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                entry_point: "vs_main",
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
            depth_stencil,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                count: msaa_samples,
                mask: !0,
            },

            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: if output_color_format.describe().srgb {
                    tracing::warn!("Detected a linear (sRGBA aware) framebuffer {:?}. egui prefers Rgba8Unorm or Bgra8Unorm", output_color_format);
                    "fs_main_linear_framebuffer"
                } else {
                    "fs_main_gamma_framebuffer" // this is what we prefer
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_color_format,
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

        const VERTEX_BUFFER_START_CAPACITY: wgpu::BufferAddress =
            (std::mem::size_of::<Vertex>() * 1024) as _;
        const INDEX_BUFFER_START_CAPACITY: wgpu::BufferAddress =
            (std::mem::size_of::<u32>() * 1024 * 3) as _;

        Self {
            pipeline,
            vertex_buffer: SlicedBuffer {
                buffer: create_vertex_buffer(device, VERTEX_BUFFER_START_CAPACITY),
                slices: Vec::with_capacity(64),
                capacity: VERTEX_BUFFER_START_CAPACITY,
            },
            index_buffer: SlicedBuffer {
                buffer: create_index_buffer(device, INDEX_BUFFER_START_CAPACITY),
                slices: Vec::with_capacity(64),
                capacity: INDEX_BUFFER_START_CAPACITY,
            },
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            textures: HashMap::new(),
            next_user_texture_id: 0,
            samplers: HashMap::new(),
            paint_callback_resources: TypeMap::default(),
        }
    }

    /// Executes the egui renderer onto an existing wgpu renderpass.
    pub fn render<'rp>(
        &'rp self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        paint_jobs: &[epaint::ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
    ) {
        crate::profile_function!();

        let pixels_per_point = screen_descriptor.pixels_per_point;
        let size_in_pixels = screen_descriptor.size_in_pixels;

        // Whether or not we need to reset the render pass because a paint callback has just
        // run.
        let mut needs_reset = true;

        let mut index_buffer_slices = self.index_buffer.slices.iter();
        let mut vertex_buffer_slices = self.vertex_buffer.slices.iter();

        for epaint::ClippedPrimitive {
            clip_rect,
            primitive,
        } in paint_jobs
        {
            if needs_reset {
                render_pass.set_viewport(
                    0.0,
                    0.0,
                    size_in_pixels[0] as f32,
                    size_in_pixels[1] as f32,
                    0.0,
                    1.0,
                );
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                needs_reset = false;
            }

            {
                let rect = ScissorRect::new(clip_rect, pixels_per_point, size_in_pixels);

                if rect.width == 0 || rect.height == 0 {
                    // Skip rendering zero-sized clip areas.
                    if let Primitive::Mesh(_) = primitive {
                        // If this is a mesh, we need to advance the index and vertex buffer iterators:
                        index_buffer_slices.next().unwrap();
                        vertex_buffer_slices.next().unwrap();
                    }
                    continue;
                }

                render_pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
            }

            match primitive {
                Primitive::Mesh(mesh) => {
                    let index_buffer_slice = index_buffer_slices.next().unwrap();
                    let vertex_buffer_slice = vertex_buffer_slices.next().unwrap();

                    if let Some((_texture, bind_group)) = self.textures.get(&mesh.texture_id) {
                        render_pass.set_bind_group(1, bind_group, &[]);
                        render_pass.set_index_buffer(
                            self.index_buffer.buffer.slice(index_buffer_slice.clone()),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.set_vertex_buffer(
                            0,
                            self.vertex_buffer.buffer.slice(vertex_buffer_slice.clone()),
                        );
                        render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
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
                        crate::profile_scope!("callback");

                        needs_reset = true;

                        {
                            // We're setting a default viewport for the render pass as a
                            // courtesy for the user, so that they don't have to think about
                            // it in the simple case where they just want to fill the whole
                            // paint area.
                            //
                            // The user still has the possibility of setting their own custom
                            // viewport during the paint callback, effectively overriding this
                            // one.

                            let min = (callback.rect.min.to_vec2() * pixels_per_point).round();
                            let max = (callback.rect.max.to_vec2() * pixels_per_point).round();

                            render_pass.set_viewport(
                                min.x,
                                min.y,
                                max.x - min.x,
                                max.y - min.y,
                                0.0,
                                1.0,
                            );
                        }

                        (cbfn.paint)(
                            PaintCallbackInfo {
                                viewport: callback.rect,
                                clip_rect: *clip_rect,
                                pixels_per_point,
                                screen_size_px: size_in_pixels,
                            },
                            render_pass,
                            &self.paint_callback_resources,
                        );
                    }
                }
            }
        }

        render_pass.set_scissor_rect(0, 0, size_in_pixels[0], size_in_pixels[1]);
    }

    /// Should be called before `render()`.
    pub fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: epaint::TextureId,
        image_delta: &epaint::ImageDelta,
    ) {
        crate::profile_function!();

        let width = image_delta.image.width() as u32;
        let height = image_delta.image.height() as u32;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let data_color32 = match &image_delta.image {
            epaint::ImageData::Color(image) => {
                assert_eq!(
                    width as usize * height as usize,
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                Cow::Borrowed(&image.pixels)
            }
            epaint::ImageData::Font(image) => {
                assert_eq!(
                    width as usize * height as usize,
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                Cow::Owned(image.srgba_pixels(None).collect::<Vec<_>>())
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
            // Use same label for all resources associated with this texture id (no point in retyping the type)
            let label_str = format!("egui_texid_{:?}", id);
            let label = Some(label_str.as_str());
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb, // Minspec for wgpu WebGL emulation is WebGL2, so this should always be supported.
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            });
            let sampler = self
                .samplers
                .entry(image_delta.options)
                .or_insert_with(|| create_sampler(image_delta.options, device));
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label,
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
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });
            let origin = wgpu::Origin3d::ZERO;
            queue_write_data_to_texture(&texture, origin);
            self.textures.insert(id, (Some(texture), bind_group));
        };
    }

    pub fn free_texture(&mut self, id: &epaint::TextureId) {
        self.textures.remove(id);
    }

    /// Get the WGPU texture and bind group associated to a texture that has been allocated by egui.
    ///
    /// This could be used by custom paint hooks to render images that have been added through with
    /// [`egui_extras::RetainedImage`](https://docs.rs/egui_extras/latest/egui_extras/image/struct.RetainedImage.html)
    /// or [`epaint::Context::load_texture`](https://docs.rs/egui/latest/egui/struct.Context.html#method.load_texture).
    pub fn texture(
        &self,
        id: &epaint::TextureId,
    ) -> Option<&(Option<wgpu::Texture>, wgpu::BindGroup)> {
        self.textures.get(id)
    }

    /// Registers a `wgpu::Texture` with a `epaint::TextureId`.
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
    ) -> epaint::TextureId {
        self.register_native_texture_with_sampler_options(
            device,
            texture,
            wgpu::SamplerDescriptor {
                label: Some(format!("egui_user_image_{}", self.next_user_texture_id).as_str()),
                mag_filter: texture_filter,
                min_filter: texture_filter,
                ..Default::default()
            },
        )
    }

    /// Registers a `wgpu::Texture` with an existing `epaint::TextureId`.
    ///
    /// This enables applications to reuse `TextureId`s.
    pub fn update_egui_texture_from_wgpu_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
        id: epaint::TextureId,
    ) {
        self.update_egui_texture_from_wgpu_texture_with_sampler_options(
            device,
            texture,
            wgpu::SamplerDescriptor {
                label: Some(format!("egui_user_image_{}", self.next_user_texture_id).as_str()),
                mag_filter: texture_filter,
                min_filter: texture_filter,
                ..Default::default()
            },
            id,
        );
    }

    /// Registers a `wgpu::Texture` with a `epaint::TextureId` while also accepting custom
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
    ) -> epaint::TextureId {
        crate::profile_function!();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: None,
            ..sampler_descriptor
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(format!("egui_user_image_{}", self.next_user_texture_id).as_str()),
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

        let id = epaint::TextureId::User(self.next_user_texture_id);
        self.textures.insert(id, (None, bind_group));
        self.next_user_texture_id += 1;

        id
    }

    /// Registers a `wgpu::Texture` with an existing `epaint::TextureId` while also accepting custom
    /// `wgpu::SamplerDescriptor` options.
    ///
    /// This allows applications to reuse `TextureId`s created with custom sampler options.
    #[allow(clippy::needless_pass_by_value)] // false positive
    pub fn update_egui_texture_from_wgpu_texture_with_sampler_options(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler_descriptor: wgpu::SamplerDescriptor<'_>,
        id: epaint::TextureId,
    ) {
        crate::profile_function!();

        let (_user_texture, user_texture_binding) = self
            .textures
            .get_mut(&id)
            .expect("Tried to update a texture that has not been allocated yet.");

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: None,
            ..sampler_descriptor
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(format!("egui_user_image_{}", self.next_user_texture_id).as_str()),
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

        *user_texture_binding = bind_group;
    }

    /// Uploads the uniform, vertex and index data used by the renderer.
    /// Should be called before `render()`.
    ///
    /// Returns all user-defined command buffers gathered from prepare callbacks.
    pub fn update_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        paint_jobs: &[epaint::ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
    ) -> Vec<wgpu::CommandBuffer> {
        crate::profile_function!();

        let screen_size_in_points = screen_descriptor.screen_size_in_points();

        {
            crate::profile_scope!("uniforms");
            // Update uniform buffer
            queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[UniformBuffer {
                    screen_size_in_points,
                    _padding: Default::default(),
                }]),
            );
        }

        // Determine how many vertices & indices need to be rendered.
        let (vertex_count, index_count) = {
            crate::profile_scope!("count_vertices_indices");
            paint_jobs.iter().fold((0, 0), |acc, clipped_primitive| {
                match &clipped_primitive.primitive {
                    Primitive::Mesh(mesh) => {
                        (acc.0 + mesh.vertices.len(), acc.1 + mesh.indices.len())
                    }
                    Primitive::Callback(_) => acc,
                }
            })
        };

        {
            // Resize index buffer if needed:
            self.index_buffer.slices.clear();
            let required_size = (std::mem::size_of::<u32>() * index_count) as u64;
            if self.index_buffer.capacity < required_size {
                self.index_buffer.capacity =
                    (self.index_buffer.capacity * 2).at_least(required_size);
                self.index_buffer.buffer = create_index_buffer(device, self.index_buffer.capacity);
            }
        }

        {
            // Resize vertex buffer if needed:
            self.vertex_buffer.slices.clear();
            let required_size = (std::mem::size_of::<Vertex>() * vertex_count) as u64;
            if self.vertex_buffer.capacity < required_size {
                self.vertex_buffer.capacity =
                    (self.vertex_buffer.capacity * 2).at_least(required_size);
                self.vertex_buffer.buffer =
                    create_vertex_buffer(device, self.vertex_buffer.capacity);
            }
        }

        // Upload index & vertex data and call user callbacks
        let mut user_cmd_bufs = Vec::new(); // collect user command buffers

        crate::profile_scope!("primitives");
        for epaint::ClippedPrimitive { primitive, .. } in paint_jobs.iter() {
            match primitive {
                Primitive::Mesh(mesh) => {
                    {
                        let index_offset = self.index_buffer.slices.last().unwrap_or(&(0..0)).end;
                        let data = bytemuck::cast_slice(&mesh.indices);
                        queue.write_buffer(&self.index_buffer.buffer, index_offset, data);
                        self.index_buffer
                            .slices
                            .push(index_offset..(data.len() as wgpu::BufferAddress + index_offset));
                    }
                    {
                        let vertex_offset = self.vertex_buffer.slices.last().unwrap_or(&(0..0)).end;
                        let data = bytemuck::cast_slice(&mesh.vertices);
                        queue.write_buffer(&self.vertex_buffer.buffer, vertex_offset, data);
                        self.vertex_buffer.slices.push(
                            vertex_offset..(data.len() as wgpu::BufferAddress + vertex_offset),
                        );
                    }
                }
                Primitive::Callback(callback) => {
                    let cbfn = if let Some(c) = callback.callback.downcast_ref::<CallbackFn>() {
                        c
                    } else {
                        tracing::warn!("Unknown paint callback: expected `egui_wgpu::CallbackFn`");
                        continue;
                    };

                    crate::profile_scope!("callback");
                    user_cmd_bufs.extend((cbfn.prepare)(
                        device,
                        queue,
                        encoder,
                        &mut self.paint_callback_resources,
                    ));
                }
            }
        }

        user_cmd_bufs
    }
}

fn create_sampler(
    options: epaint::textures::TextureOptions,
    device: &wgpu::Device,
) -> wgpu::Sampler {
    let mag_filter = match options.magnification {
        epaint::textures::TextureFilter::Nearest => wgpu::FilterMode::Nearest,
        epaint::textures::TextureFilter::Linear => wgpu::FilterMode::Linear,
    };
    let min_filter = match options.minification {
        epaint::textures::TextureFilter::Nearest => wgpu::FilterMode::Nearest,
        epaint::textures::TextureFilter::Linear => wgpu::FilterMode::Linear,
    };
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(&format!(
            "egui sampler (mag: {:?}, min {:?})",
            mag_filter, min_filter
        )),
        mag_filter,
        min_filter,
        ..Default::default()
    })
}

fn create_vertex_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    crate::profile_function!();
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("egui_vertex_buffer"),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        size,
        mapped_at_creation: false,
    })
}

fn create_index_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    crate::profile_function!();
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("egui_index_buffer"),
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        size,
        mapped_at_creation: false,
    })
}

/// A Rect in physical pixel space, used for setting cliipping rectangles.
struct ScissorRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl ScissorRect {
    fn new(clip_rect: &epaint::Rect, pixels_per_point: f32, target_size: [u32; 2]) -> Self {
        // Transform clip rect to physical pixels:
        let clip_min_x = pixels_per_point * clip_rect.min.x;
        let clip_min_y = pixels_per_point * clip_rect.min.y;
        let clip_max_x = pixels_per_point * clip_rect.max.x;
        let clip_max_y = pixels_per_point * clip_rect.max.y;

        // Round to integer:
        let clip_min_x = clip_min_x.round() as u32;
        let clip_min_y = clip_min_y.round() as u32;
        let clip_max_x = clip_max_x.round() as u32;
        let clip_max_y = clip_max_y.round() as u32;

        // Clamp:
        let clip_min_x = clip_min_x.clamp(0, target_size[0]);
        let clip_min_y = clip_min_y.clamp(0, target_size[1]);
        let clip_max_x = clip_max_x.clamp(clip_min_x, target_size[0]);
        let clip_max_y = clip_max_y.clamp(clip_min_y, target_size[1]);

        Self {
            x: clip_min_x,
            y: clip_min_y,
            width: clip_max_x - clip_min_x,
            height: clip_max_y - clip_min_y,
        }
    }
}

#[test]
fn renderer_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Renderer>();
}
