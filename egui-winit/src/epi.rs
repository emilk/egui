pub fn window_builder(
    native_options: &epi::NativeOptions,
    window_settings: &Option<crate::WindowSettings>,
) -> winit::window::WindowBuilder {
    let window_icon = native_options.icon_data.clone().and_then(load_icon);

    let mut window_builder = winit::window::WindowBuilder::new()
        .with_always_on_top(native_options.always_on_top)
        .with_maximized(native_options.maximized)
        .with_decorations(native_options.decorated)
        .with_resizable(native_options.resizable)
        .with_transparent(native_options.transparent)
        .with_window_icon(window_icon);

    window_builder =
        window_builder_drag_and_drop(window_builder, native_options.drag_and_drop_support);

    let initial_size_points = native_options.initial_window_size;

    if let Some(window_settings) = window_settings {
        window_builder = window_settings.initialize_window(window_builder);
    } else if let Some(initial_size_points) = initial_size_points {
        window_builder = window_builder.with_inner_size(winit::dpi::LogicalSize {
            width: initial_size_points.x as f64,
            height: initial_size_points.y as f64,
        });
    }

    window_builder
}

fn load_icon(icon_data: epi::IconData) -> Option<winit::window::Icon> {
    winit::window::Icon::from_rgba(icon_data.rgba, icon_data.width, icon_data.height).ok()
}

#[cfg(target_os = "windows")]
fn window_builder_drag_and_drop(
    window_builder: winit::window::WindowBuilder,
    enable: bool,
) -> winit::window::WindowBuilder {
    window_builder.with_drag_and_drop(enable)
}

#[cfg(not(target_os = "windows"))]
fn window_builder_drag_and_drop(
    window_builder: winit::window::WindowBuilder,
    _enable: bool,
) -> winit::window::WindowBuilder {
    // drag and drop can only be disabled on windows
    window_builder
}

pub fn handle_app_output(
    window: &winit::window::Window,
    current_pixels_per_point: f32,
    app_output: epi::backend::AppOutput,
) {
    let epi::backend::AppOutput {
        quit: _,
        window_size,
        decorated,
        drag_window,
    } = app_output;

    if let Some(decorated) = decorated {
        window.set_decorations(decorated);
    }

    if let Some(window_size) = window_size {
        window.set_inner_size(
            winit::dpi::PhysicalSize {
                width: (current_pixels_per_point * window_size.x).round(),
                height: (current_pixels_per_point * window_size.y).round(),
            }
            .to_logical::<f32>(crate::native_pixels_per_point(window) as f64),
        );
    }

    if drag_window {
        let _ = window.drag_window();
    }
}
