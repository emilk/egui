#![deny(warnings)]

use {
    emigui::{
        label,
        math::vec2,
        widgets::{Button, Label},
        Emigui,
    },
    emigui_glium::Painter,
    glium::glutin,
};

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let pixels_per_point = display.gl_window().get_hidpi_factor() as f32;

    let mut emigui = Emigui::new(pixels_per_point);
    let mut painter = Painter::new(&display);

    let mut raw_input = emigui::RawInput {
        screen_size: {
            let (width, height) = display.get_framebuffer_dimensions();
            vec2(width as f32, height as f32) / pixels_per_point
        },
        pixels_per_point,
        ..Default::default()
    };

    let mut paint = |raw_input| {
        emigui.new_frame(raw_input);
        let mut region = emigui.whole_screen_region();
        let mut region = region.left_column(region.width().min(480.0));
        region.add(label!("Emigui!").text_style(emigui::TextStyle::Heading));
        let exit = region.add(Button::new("Quit")).clicked;
        emigui.example(&mut region);
        let mesh = emigui.paint();
        painter.paint(&display, mesh, emigui.texture());
        exit
    };

    paint(raw_input);

    events_loop.run_forever(|event| {
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => return glutin::ControlFlow::Break,

                glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                    raw_input.screen_size = vec2(width as f32, height as f32) / pixels_per_point;
                    if paint(raw_input) {
                        return glutin::ControlFlow::Break;
                    }
                }
                glutin::WindowEvent::MouseInput { state, .. } => {
                    raw_input.mouse_down = state == glutin::ElementState::Pressed;
                    if paint(raw_input) {
                        return glutin::ControlFlow::Break;
                    }
                }
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    raw_input.mouse_pos = Some(vec2(position.x as f32, position.y as f32));
                    if paint(raw_input) {
                        return glutin::ControlFlow::Break;
                    }
                }
                _ => (),
            },
            _ => (),
        }
        glutin::ControlFlow::Continue
    });
}
