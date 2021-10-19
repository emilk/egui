pub fn window_builder(
    native_options: &epi::NativeOptions,
    window_settings: &Option<crate::WindowSettings>,
    window_icon: Option<winit::window::Icon>,
) -> winit::window::WindowBuilder {
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
