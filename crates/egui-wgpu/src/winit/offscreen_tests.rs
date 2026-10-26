// Unit tests for offscreen painting, with access to the painter's private state.
use super::*;

fn block_on<F: core::future::Future>(future: F) -> F::Output {
    struct WakeThread(std::thread::Thread);
    impl std::task::Wake for WakeThread {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = core::task::Waker::from(Arc::new(WakeThread(std::thread::current())));
    let mut context = core::task::Context::from_waker(&waker);
    let mut future = core::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            core::task::Poll::Ready(output) => return output,
            core::task::Poll::Pending => std::thread::park(),
        }
    }
}

/// No window, surface, compositor, or subsequent paint: callbacks must still complete.
#[test]
fn hidden_window_capture_without_surface() {
    let config = WgpuConfiguration {
        wgpu_setup: crate::WgpuSetup::CreateNew(crate::WgpuSetupCreateNew::without_display_handle()),
        ..Default::default()
    };
    let options = RendererOptions::default();
    let mut painter = block_on(Painter::new(
        Context::default(),
        config.clone(),
        false,
        options,
    ));
    painter.render_state = Some(
        block_on(RenderState::create(
            &config,
            &painter.instance,
            None,
            options,
        ))
        .expect("headless wgpu adapter"),
    );

    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        let state = painter.render_state.as_mut().unwrap();
        state.target_format = format;
        *state.renderer.write() = crate::Renderer::new(&state.device, format, options);
        painter.screen_capture_state = None;
        // Odd widths exercise padded rows; desktop size exercises bulk readback.
        for size in [[67, 31], [79, 23], [3840, 2160]] {
            let ctx = Context::default();
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(size[0] as f32 / 2.0, size[1] as f32 / 2.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(egui::pos2(2.0, 2.0), egui::vec2(8.0, 6.0)),
                        0.0,
                        egui::Color32::RED,
                    );
                },
            );
            let primitives = ctx.tessellate(output.shapes, 2.0);
            let mut delta = output.textures_delta;
            delta.free.insert(epaint::TextureId::Managed(0));
            let (tx, rx) = std::sync::mpsc::channel();
            let callbacks = (0..2)
                .map(|_| {
                    let sender = tx.clone();
                    egui::ScreenshotCallback::new(move |image| {
                        sender.send(image).unwrap();
                    })
                })
                .collect();
            drop(tx);
            painter.paint_and_update_textures_impl(
                ViewportId::ROOT,
                size,
                2.0,
                [0.0, 0.0, 1.0, 1.0],
                &primitives,
                &mut delta,
                callbacks,
                None,
            );
            for _ in 0..2 {
                let image = rx
                    .recv_timeout(core::time::Duration::from_secs(15))
                    .expect("offscreen callback without another paint");
                assert_eq!(image.resolution(), [size[0] as usize, size[1] as usize]);
                assert_eq!(image[(0, 0)], egui::Color32::BLUE);
                assert_eq!(image[(8, 8)], egui::Color32::RED);
            }
            assert!(delta.is_empty(), "texture uploads and frees consumed");
            assert!(
                painter
                    .render_state
                    .as_ref()
                    .unwrap()
                    .renderer
                    .read()
                    .texture(&epaint::TextureId::Managed(0))
                    .is_none(),
                "texture freed after submission"
            );
            assert!(painter.surfaces.is_empty(), "no surface was created");
        }
    }
}
