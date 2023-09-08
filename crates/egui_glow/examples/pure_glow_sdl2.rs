//! Example how to use pure `egui_glow` with `sdl2`.

#![allow(unsafe_code)]

fn main() {
    env_logger::init();
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let gl_attr = video.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 0);
    let window = video
        .window("Hello egui sdl2!", 1024, 769)
        .opengl()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();
    let gl_context = window.gl_create_context().unwrap();
    window
        .subsystem()
        .gl_set_swap_interval(sdl2::video::SwapInterval::LateSwapTearing)
        .or_else(|_| {
            window
                .subsystem()
                .gl_set_swap_interval(sdl2::video::SwapInterval::VSync)
        })
        .expect("Could not gl_set_swap_interval(...)");

    let (gl, window, mut events_loop, _gl_context) = {
        let gl = unsafe {
            glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _)
        };
        let event_loop = sdl.event_pump().unwrap();
        (gl, window, event_loop, gl_context)
    };
    let gl = std::sync::Arc::new(gl);
    let mut egui_glow = egui_glow::sdl2::EguiGlow::new(&window, gl.clone(), None);

    let clipboard = &mut sdl.video().unwrap().clipboard();

    let mut clear_colour: [f32; 3] = [0.1, 0.1, 0.1];
    let mut name = "Ted";
    let mut age = 41;

    'mainloop: loop {
        let mut quit_clicked: bool = false;
        egui_glow.run(&window, clipboard, |egui_ctx| {
            egui::SidePanel::left("my_left_side_panel").show(egui_ctx, |ui| {
                ui.heading("Hello World!");
                if ui.button("Quit").clicked() {
                    quit_clicked = true;
                }
                ui.color_edit_button_rgb(&mut clear_colour);
                ui.hyperlink_to("here", "http://www.google.com");
            });
            egui::SidePanel::right("my_right_side_panel").show(egui_ctx, |ui| {
                ui.heading("My egui Application");
                ui.horizontal(|ui| {
                    ui.label("Your name: ");
                    ui.text_edit_singleline(&mut name);
                });
                ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
                if ui.button("Click each year").clicked() {
                    age += 1;
                }
                ui.label(format!("Hello '{name}', age {age}"));
            });
        });

        if quit_clicked {
            break 'mainloop;
        }

        unsafe {
            use glow::HasContext as _;
            gl.clear_color(clear_colour[0], clear_colour[1], clear_colour[2], 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        // draw things behind egui here

        egui_glow.paint(&window);

        // draw things on top of egui here
        window.gl_swap_window();

        for event in events_loop.poll_iter() {
            //repaint_after = std::time::Duration::from_secs(0);
            egui_glow.on_event(&event, &window);

            if let sdl2::event::Event::Quit { .. } = event {
                break 'mainloop;
            }
        }
    }
}
