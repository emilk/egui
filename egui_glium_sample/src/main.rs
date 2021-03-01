#[macro_use]
extern crate glium;

use cgmath::SquareMatrix;
#[allow(unused_imports)]
use glium::{glutin, Surface};
use glium::{IndexBuffer, VertexBuffer};
use std::time::Instant;
use egui_glium::{Painter, GliumInputState, init_clipboard, native_pixels_per_point, screen_size_in_pixels, handle_output, input_to_egui, seconds_since_midnight};
use egui::Rect;
use epi::{IntegrationInfo, RepaintSignal};
use std::sync::Arc;
use glium::backend::glutin::glutin::event_loop::ControlFlow;
use std::borrow::Borrow;
use std::cell::RefCell;

struct Renderer {
    size: [u32; 2],

    model_vertex_buffer: VertexBuffer<Vertex>,
    model_index_buffer: IndexBuffer<u16>,
    model_data: [ModelData; 4],
    shadow_map_shaders: glium::Program,
    shadow_texture:glium::texture::DepthTexture2d,
    render_shaders: glium::Program,

    depth_texture:glium::texture::DepthTexture2d,
    light_t: f64,
    light_rotating: bool,
    camera_t: f64,
    camera_rotating: bool,
    start: Instant,
}
impl Renderer {
    fn init(display:&glium::Display,render_target:&glium::texture::SrgbTexture2d) -> Self {
        let shadow_map_size = 1024;
        let width=render_target.width();
        let height=render_target.height();
        let size=[width,height];
        // Create the boxes to render in the scene
        let (model_vertex_buffer, model_index_buffer) = create_box(display);
        let  model_data = [
            ModelData::color([0.4, 0.4, 0.4])
                .translate([0.0, -2.5, 0.0])
                .scale(5.0),
            ModelData::color([0.6, 0.1, 0.1])
                .translate([0.0, 0.252, 0.0])
                .scale(0.5),
            ModelData::color([0.1, 0.6, 0.1])
                .translate([0.9, 0.5, 0.1])
                .scale(0.5),
            ModelData::color([0.1, 0.1, 0.6])
                .translate([-0.8, 0.75, 0.1])
                .scale(0.5),
        ];

        let shadow_map_shaders = glium::Program::from_source(
            display,
            // Vertex Shader
            "
            #version 330 core
            in vec4 position;
            uniform mat4 depth_mvp;
            void main() {
              gl_Position = depth_mvp * position;
            }
        ",
            // Fragement Shader
            "
            #version 330 core
            layout(location = 0) out float fragmentdepth;
            void main(){
                fragmentdepth = gl_FragCoord.z;
            }
        ",
            None,
        )
        .unwrap();

        let render_shaders = glium::Program::from_source(
            display,
            // Vertex Shader
            "
            #version 330 core
            uniform mat4 mvp;
            uniform mat4 depth_bias_mvp;
            uniform mat4 model_matrix;
            uniform vec4 model_color;
            in vec4 position;
            in vec4 normal;
            out vec4 shadow_coord;
            out vec4 model_normal;
            void main() {
            	gl_Position =  mvp * position;
            	model_normal = model_matrix * normal;
            	shadow_coord = depth_bias_mvp * position;
            }
        ",
            // Fragement Shader
            "
            #version 330 core
            uniform sampler2DShadow shadow_map;
            uniform vec3 light_loc;
            uniform vec4 model_color;
            in vec4 shadow_coord;
            in vec4 model_normal;
            out vec4 color;
            void main() {
                vec3 light_color = vec3(1,1,1);
            	float bias = 0.0; // Geometry does not require bias
            	float lum = max(dot(normalize(model_normal.xyz), normalize(light_loc)), 0.0);
            	float visibility = texture(shadow_map, vec3(shadow_coord.xy, (shadow_coord.z-bias)/shadow_coord.w));
            	color = vec4(max(lum * visibility, 0.05) * model_color.rgb * light_color, 1.0);
            }
        ",
            None).unwrap();
        let shadow_texture =
            glium::texture::DepthTexture2d::empty(display, shadow_map_size, shadow_map_size)
                .unwrap();

        let depth_texture=glium::texture::DepthTexture2d::empty(display,size[0],size[1]).unwrap();
        let  start = Instant::now();

        let  light_t: f64 = 8.7;
        let  light_rotating = true;
        let  camera_t: f64 = 8.22;
        let  camera_rotating = false;
        Self{
            size,

            model_vertex_buffer,
            model_index_buffer,
            model_data,
            shadow_map_shaders,
            shadow_texture,
            render_shaders,

            depth_texture,
            light_t,
            light_rotating,
            camera_t,
            camera_rotating,
            start
        }
    }
    fn render(&mut self,display:&glium::Display,virtual_screen:&glium::texture::SrgbTexture2d) {
        let elapsed_dur = self.start.elapsed();
        let secs = (elapsed_dur.as_secs() as f64) + (elapsed_dur.subsec_nanos() as f64) * 1e-9;
        self.start = Instant::now();

        if self.camera_rotating {
            self.camera_t += secs * 0.7;
        }
        if self.light_rotating {
            self.light_t += secs * 0.7;
        }



        // Rotate the light around the center of the scene
        let light_loc = {
            let x = 3.0 * self.light_t.cos();
            let z = 3.0 * self.light_t.sin();
            [x as f32, 5.0, z as f32]
        };

        // Render the scene from the light's point of view into depth buffer
        // ===============================================================================
        {
            // Orthographic projection used to demostrate a far-away light source
            let w = 4.0;
            let depth_projection_matrix: cgmath::Matrix4<f32> =
                cgmath::ortho(-w, w, -w, w, -10.0, 20.0);
            let view_center: cgmath::Point3<f32> = cgmath::Point3::new(0.0, 0.0, 0.0);
            let view_up: cgmath::Vector3<f32> = cgmath::Vector3::new(0.0, 1.0, 0.0);
            let depth_view_matrix =
                cgmath::Matrix4::look_at_rh(light_loc.into(), view_center, view_up);

            let mut draw_params: glium::draw_parameters::DrawParameters<'_> = Default::default();
            draw_params.depth = glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            };
            draw_params.backface_culling = glium::BackfaceCullingMode::CullClockwise;

            // Write depth to shadow map texture
            let mut target =
                glium::framebuffer::SimpleFrameBuffer::depth_only(display, &self.shadow_texture)
                    .unwrap();
            target.clear_color(1.0, 1.0, 1.0, 1.0);
            target.clear_depth(1.0);

            // Draw each model
            for md in &mut self.model_data {
                let depth_mvp = depth_projection_matrix * depth_view_matrix * md.model_matrix;
                md.depth_mvp = depth_mvp;

                let uniforms = uniform! {
                    depth_mvp: Into::<[[f32; 4]; 4]>::into(md.depth_mvp),
                };

                target
                    .draw(
                        &self.model_vertex_buffer,
                        &self.model_index_buffer,
                        &self.shadow_map_shaders,
                        &uniforms,
                        &draw_params,
                    )
                    .unwrap();
            }
        }

        // Render the scene from the camera's point of view
        // ===============================================================================
        let screen_ratio = (self.size[0] / self.size[1]) as f32;
        let perspective_matrix: cgmath::Matrix4<f32> =
            cgmath::perspective(cgmath::Deg(45.0), screen_ratio, 0.0001, 100.0);
        let camera_x = 3.0 * self.camera_t.cos();
        let camera_z = 3.0 * self.camera_t.sin();
        let view_eye: cgmath::Point3<f32> =
            cgmath::Point3::new(camera_x as f32, 2.0, camera_z as f32);
        let view_center: cgmath::Point3<f32> = cgmath::Point3::new(0.0, 0.0, 0.0);
        let view_up: cgmath::Vector3<f32> = cgmath::Vector3::new(0.0, 1.0, 0.0);
        let view_matrix: cgmath::Matrix4<f32> =
            cgmath::Matrix4::look_at_rh(view_eye, view_center, view_up);

        let bias_matrix: cgmath::Matrix4<f32> = [
            [0.5, 0.0, 0.0, 0.0],
            [0.0, 0.5, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.0],
            [0.5, 0.5, 0.5, 1.0],
        ]
        .into();

        let mut draw_params: glium::draw_parameters::DrawParameters<'_> = Default::default();
        draw_params.depth = glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLessOrEqual,
            write: true,
            ..Default::default()
        };
        draw_params.backface_culling = glium::BackfaceCullingMode::CullCounterClockwise;
        draw_params.blend = glium::Blend::alpha_blending();


        let mut target=glium::framebuffer::SimpleFrameBuffer::with_depth_buffer(display,virtual_screen,&self.depth_texture).unwrap();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

        // Draw each model
        for md in &self.model_data {
            let mvp = perspective_matrix * view_matrix * md.model_matrix;
            let depth_bias_mvp = bias_matrix * md.depth_mvp;

            let uniforms = uniform! {
                light_loc: light_loc,
                perspective_matrix: Into::<[[f32; 4]; 4]>::into(perspective_matrix),
                view_matrix: Into::<[[f32; 4]; 4]>::into(view_matrix),
                model_matrix: Into::<[[f32; 4]; 4]>::into(md.model_matrix),
                model_color: md.color,

                mvp: Into::<[[f32;4];4]>::into(mvp),
                depth_bias_mvp: Into::<[[f32;4];4]>::into(depth_bias_mvp),
                shadow_map: glium::uniforms::Sampler::new(&self.shadow_texture)
                    .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                    .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                    .depth_texture_comparison(Some(glium::uniforms::DepthTextureComparison::LessOrEqual)),
            };

            target
                .draw(
                    &self.model_vertex_buffer,
                    &self.model_index_buffer,
                    &self.render_shaders,
                    &uniforms,
                    &draw_params,
                )
                .unwrap();
        }
       // target.finish().unwrap();
    }
}

fn create_box(display: & glium::Display) -> (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u16>) {
    let box_vertex_buffer = glium::VertexBuffer::new(
        display,
        &[
            // Max X
            Vertex {
                position: [0.5, -0.5, -0.5, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
            },
            // Min X
            Vertex {
                position: [-0.5, -0.5, -0.5, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
            },
            // Max Y
            Vertex {
                position: [-0.5, 0.5, -0.5, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
            },
            // Min Y
            Vertex {
                position: [-0.5, -0.5, -0.5, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
            },
            // Max Z
            Vertex {
                position: [-0.5, -0.5, 0.5, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
            },
            // Min Z
            Vertex {
                position: [-0.5, -0.5, -0.5, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
            },
        ],
    )
    .unwrap();

    let mut indexes = Vec::new();
    for face in 0..6u16 {
        indexes.push(4 * face + 0);
        indexes.push(4 * face + 1);
        indexes.push(4 * face + 2);
        indexes.push(4 * face + 0);
        indexes.push(4 * face + 2);
        indexes.push(4 * face + 3);
    }
    let box_index_buffer = glium::IndexBuffer::new(
        display,
        glium::index::PrimitiveType::TrianglesList,
        &indexes,
    )
    .unwrap();
    (box_vertex_buffer, box_index_buffer)
}

#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 4],
    normal: [f32; 4],
}
implement_vertex!(Vertex, position, normal);

#[derive(Clone, Debug)]
struct ModelData {
    model_matrix: cgmath::Matrix4<f32>,
    depth_mvp: cgmath::Matrix4<f32>,
    color: [f32; 4],
}
impl ModelData {
    pub fn color(c: [f32; 3]) -> Self {
        Self {
            model_matrix: cgmath::Matrix4::identity(),
            depth_mvp: cgmath::Matrix4::identity(),
            color: [c[0], c[1], c[2], 1.0],
        }
    }
    pub fn scale(mut self, s: f32) -> Self {
        self.model_matrix = self.model_matrix * cgmath::Matrix4::from_scale(s);
        self
    }
    pub fn translate(mut self, t: [f32; 3]) -> Self {
        self.model_matrix = self.model_matrix * cgmath::Matrix4::from_translation(t.into());
        self
    }
}

#[derive(Clone, Copy, Debug)]
struct DebugVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(DebugVertex, position, tex_coords);
impl DebugVertex {
    pub fn new(position: [f32; 2], tex_coords: [f32; 2]) -> Self {
        Self {
            position,
            tex_coords,
        }
    }
}
enum RequestRepaintEvent{
    RequestRedraw,
}

struct GliumRepaintSignal(
    std::sync::Mutex<glutin::event_loop::EventLoopProxy<RequestRepaintEvent>>,
);

impl epi::RepaintSignal for GliumRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(RequestRepaintEvent::RequestRedraw).ok();
    }
}
fn main() {
    //init glium
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let wb=glutin::window::WindowBuilder::new();
    let cb=glutin::ContextBuilder::new().with_vsync(true);
    let display=glium::Display::new(wb,cb,&event_loop).unwrap();
    //init egui
    let repaint_signal = std::sync::Arc::new(GliumRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));
    let mut ctx = egui::CtxRef::default();
    let mut input_state = GliumInputState::from_pixels_per_point(native_pixels_per_point(&display));

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    let mut painter = Painter::new(&display);
    let mut clipboard = init_clipboard();
    let virtual_screen=glium::texture::SrgbTexture2d::empty(&display,640,480).unwrap();

    // move texture to painter
    let texture_id_for_frame_buffer=painter.assume_glium_texture_as_egui_texture(virtual_screen);
    //init renderer
    let mut renderer =Renderer::init(&display, painter.rental_texture(texture_id_for_frame_buffer).unwrap());
    //assume


    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let pixels_per_point = input_state
                .raw
                .pixels_per_point
                .unwrap_or_else(|| ctx.pixels_per_point());

            let frame_start = Instant::now();
            input_state.raw.time = Some(start_time.elapsed().as_nanos() as f64 * 1e-9);
            input_state.raw.screen_rect = Some(Rect::from_min_size(
                Default::default(),
                screen_size_in_pixels(&display) / pixels_per_point,
            ));

            ctx.begin_frame(input_state.raw.take());
            let mut app_output = epi::backend::AppOutput::default();
            let mut frame = epi::backend::FrameBuilder {
                info:IntegrationInfo{
                    web_info: None,
                    cpu_usage: None,
                    seconds_since_midnight: Some(seconds_since_midnight().unwrap()),
                    native_pixels_per_point: Some(native_pixels_per_point(&display))
                },
                tex_allocator: &mut painter,
                #[cfg(feature = "http")]
                http: http.clone(),
                output: &mut app_output,
                repaint_signal:repaint_signal.clone(),
            }
                .build();
            //app.update(&ctx, &mut frame);
            renderer.render(&display,painter.rental_texture(texture_id_for_frame_buffer).unwrap());
            egui::Window::new("Offscreen render").fixed_size([640.0,480.0]).show(&ctx,|ui|{
                ui.image(texture_id_for_frame_buffer,[640.0,480.0]);
            });
            let (egui_output, shapes) = ctx.end_frame();
            let clipped_meshes = ctx.tessellate(shapes);

            let frame_time = (Instant::now() - frame_start).as_secs_f64() as f32;
            previous_frame_time = Some(frame_time);
            painter.paint_meshes(
                &display,
                ctx.pixels_per_point(),
                egui::Rgba::from_rgb(0.0,0.0,0.0),
                clipped_meshes,
                &ctx.texture(),
            );

            {
                let epi::backend::AppOutput { quit, window_size } = app_output;

                if let Some(window_size) = window_size {
                    display.gl_window().window().set_inner_size(
                        glutin::dpi::PhysicalSize {
                            width: (ctx.pixels_per_point() * window_size.x).round(),
                            height: (ctx.pixels_per_point() * window_size.y).round(),
                        }
                            .to_logical::<f32>(native_pixels_per_point(&display) as f64),
                    );
                }

                *control_flow = if quit {
                    glutin::event_loop::ControlFlow::Exit
                } else if egui_output.needs_repaint {
                    display.gl_window().window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else {
                    glutin::event_loop::ControlFlow::Wait
                };
            }

            handle_output(egui_output, &display, clipboard.as_mut());

        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),
            glutin::event::Event::WindowEvent { event, .. } => {
                input_to_egui(
                    ctx.pixels_per_point(),
                    event,
                    clipboard.as_mut(),
                    &mut input_state,
                    control_flow,
                );
                display.gl_window().window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            glutin::event::Event::UserEvent(RequestRepaintEvent) => {
                display.gl_window().window().request_redraw();
            }

            _ => (),
        }
    });
}


