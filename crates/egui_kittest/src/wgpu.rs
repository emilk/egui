use core::{iter::once, time::Duration};
use std::sync::Arc;
#[expect(
    clippy::disallowed_types,
    reason = "WARP render tests may wait longer than epaint::mutex's 10-second deadlock timeout"
)]
use std::sync::{Mutex, MutexGuard};

use egui::TexturesDelta;
use egui_wgpu::{RenderState, ScreenDescriptor, WgpuSetup, wgpu};
use image::RgbaImage;

use crate::texture_to_image::texture_to_image;

/// Timeout for waiting on the GPU to finish rendering.
///
/// Windows will reset native drivers after 2 seconds of being stuck (known was TDR - timeout detection & recovery).
/// However, software rasterizers like lavapipe may not do that and take longer if there's a lot of work in flight.
/// In the end, what we really want to protect here against is undetected errors that lead to device loss
/// and therefore infinite waits it happens occasionally on MacOS/Metal as of writing.
pub(crate) const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

#[expect(
    clippy::disallowed_types,
    reason = "WARP render tests may wait longer than epaint::mutex's 10-second deadlock timeout"
)]
static WARP_RENDER_LOCK: Mutex<()> = Mutex::new(());

fn should_serialize_warp_adapter(backend: wgpu::Backend, device_type: wgpu::DeviceType) -> bool {
    backend == wgpu::Backend::Dx12 && device_type == wgpu::DeviceType::Cpu
}

fn warp_render_lock(is_warp_adapter: bool) -> Option<MutexGuard<'static, ()>> {
    is_warp_adapter.then(|| {
        WARP_RENDER_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    })
}

/// Default wgpu setup used for the wgpu renderer.
pub fn default_wgpu_setup() -> egui_wgpu::WgpuSetup {
    // No display handle needed for headless testing — we don't present to a window.
    let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();

    // WebGPU not supported yet since we rely on blocking screenshots.
    setup
        .instance_descriptor
        .backends
        .remove(wgpu::Backends::BROWSER_WEBGPU);

    // Prefer software rasterizers.
    setup.native_adapter_selector = Some(Arc::new(|adapters, _surface| {
        let mut adapters = adapters.iter().collect::<Vec<_>>();

        // Adapters are already sorted by preferred backend by wgpu, but let's be explicit.
        adapters.sort_by_key(|a| match a.get_info().backend {
            wgpu::Backend::Metal => 0,
            wgpu::Backend::Vulkan => 1,
            wgpu::Backend::Dx12 => 2,
            wgpu::Backend::Gl => 4,
            wgpu::Backend::BrowserWebGpu => 6,
            wgpu::Backend::Noop => 7,
        });

        // Prefer CPU adapters, otherwise if we can't, prefer discrete GPU over integrated GPU.
        adapters.sort_by_key(|a| match a.get_info().device_type {
            wgpu::DeviceType::Cpu => 0, // CPU is the best for our purposes!
            wgpu::DeviceType::DiscreteGpu => 1,
            wgpu::DeviceType::Other
            | wgpu::DeviceType::IntegratedGpu
            | wgpu::DeviceType::VirtualGpu => 2,
        });

        adapters
            .first()
            .map(|a| (*a).clone())
            .ok_or_else(|| "No adapter found".to_owned())
    }));

    egui_wgpu::WgpuSetup::CreateNew(setup)
}

pub fn create_render_state(
    setup: WgpuSetup,
    options: egui_wgpu::RendererOptions,
) -> egui_wgpu::RenderState {
    // No display handle needed for headless testing — we don't present to a window.
    let instance = pollster::block_on(setup.new_instance());

    pollster::block_on(egui_wgpu::RenderState::create(
        &egui_wgpu::WgpuConfiguration {
            wgpu_setup: setup,
            ..Default::default()
        },
        &instance,
        None,
        options,
    ))
    .expect("Failed to create render state")
}

/// Utility to render snapshots from a [`crate::Harness`] using [`egui_wgpu`].
pub struct WgpuTestRenderer {
    render_state: Option<RenderState>,
    is_warp_adapter: bool,
}

impl Default for WgpuTestRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl WgpuTestRenderer {
    /// Create a new [`WgpuTestRenderer`] with the default setup.
    pub fn new() -> Self {
        Self::from_render_state(create_render_state(
            default_wgpu_setup(),
            egui_wgpu::RendererOptions::PREDICTABLE,
        ))
    }

    /// Create a new [`WgpuTestRenderer`] with the given setup.
    pub fn from_setup(setup: WgpuSetup) -> Self {
        Self::from_render_state(create_render_state(
            setup,
            egui_wgpu::RendererOptions::PREDICTABLE,
        ))
    }

    /// Create a new [`WgpuTestRenderer`] from an existing [`RenderState`].
    ///
    /// # Panics
    /// Panics if the [`RenderState`] has been used before.
    pub fn from_render_state(render_state: RenderState) -> Self {
        assert!(
            render_state
                .renderer
                .read()
                .texture(&egui::epaint::TextureId::Managed(0))
                .is_none(),
            "The RenderState passed in has been used before, pass in a fresh RenderState instead."
        );
        let adapter_info = render_state.adapter.get_info();
        Self {
            render_state: Some(render_state),
            is_warp_adapter: should_serialize_warp_adapter(
                adapter_info.backend,
                adapter_info.device_type,
            ),
        }
    }

    /// Create a new [`WgpuTestRenderer`] with custom render options.
    pub fn with_render_options(options: egui_wgpu::RendererOptions) -> Self {
        Self::from_render_state(create_render_state(default_wgpu_setup(), options))
    }

    fn render_state(&self) -> Option<&RenderState> {
        self.render_state.as_ref()
    }
}

impl Drop for WgpuTestRenderer {
    fn drop(&mut self) {
        let _lock = warp_render_lock(self.is_warp_adapter);
        drop(self.render_state.take());
    }
}

impl crate::TestRenderer for WgpuTestRenderer {
    #[cfg(feature = "eframe")]
    fn setup_eframe(&self, cc: &mut eframe::CreationContext<'_>, frame: &mut eframe::Frame) {
        if let Some(render_state) = self.render_state() {
            cc.wgpu_render_state = Some(render_state.clone());
            frame.wgpu_render_state = Some(render_state.clone());
        }
    }

    fn handle_delta(&mut self, delta: &mut TexturesDelta) {
        let _lock = warp_render_lock(self.is_warp_adapter);
        let Some(render_state) = self.render_state() else {
            return;
        };
        let mut renderer = render_state.renderer.write();
        #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
        for (id, images) in delta.set.drain() {
            for image in images {
                renderer.update_texture(&render_state.device, &render_state.queue, id, &image);
            }
        }

        #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
        for id in delta.free.drain() {
            renderer.free_texture(&id);
        }
    }

    /// Render the [`crate::Harness`] and return the resulting image.
    fn render(
        &mut self,
        ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<RgbaImage, String> {
        let _lock = warp_render_lock(self.is_warp_adapter);
        let Some(render_state) = self.render_state() else {
            return Err("WgpuTestRenderer was dropped".to_owned());
        };
        let mut renderer = render_state.renderer.write();

        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Egui Command Encoder"),
                });

        let size = ctx.content_rect().size() * ctx.pixels_per_point();
        let screen = ScreenDescriptor {
            pixels_per_point: ctx.pixels_per_point(),
            size_in_pixels: [size.x.round() as u32, size.y.round() as u32],
        };

        let tessellated = ctx.tessellate(output.shapes.clone(), ctx.pixels_per_point());

        let user_buffers = renderer.update_buffers(
            &render_state.device,
            &render_state.queue,
            &mut encoder,
            &tessellated,
            &screen,
        );

        let texture = render_state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Egui Texture"),
                size: wgpu::Extent3d {
                    width: screen.size_in_pixels[0],
                    height: screen.size_in_pixels[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: render_state.target_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Egui Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                })
                .forget_lifetime();

            renderer.render(&mut pass, &tessellated, &screen);
        }

        render_state
            .queue
            .submit(core::iter::chain(user_buffers, once(encoder.finish())));

        render_state
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(WAIT_TIMEOUT),
            })
            .map_err(|err| format!("PollError: {err}"))?;

        Ok(texture_to_image(
            &render_state.device,
            &render_state.queue,
            &texture,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::{sync::Arc, thread};

    #[test]
    fn warp_rendering_is_serialized() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let threads = (0..8)
            .map(|_| {
                let active = Arc::clone(&active);
                let max_active = Arc::clone(&max_active);
                thread::Builder::new()
                    .name("warp-render-test".to_owned())
                    .spawn(move || {
                        let _lock = warp_render_lock(true);
                        let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(now_active, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(10));
                        active.fetch_sub(1, Ordering::SeqCst);
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();

        for thread in threads {
            thread.join().unwrap();
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn only_cpu_dx12_adapters_are_classified_as_warp() {
        assert!(should_serialize_warp_adapter(
            wgpu::Backend::Dx12,
            wgpu::DeviceType::Cpu
        ));
        assert!(!should_serialize_warp_adapter(
            wgpu::Backend::Dx12,
            wgpu::DeviceType::DiscreteGpu
        ));
        assert!(!should_serialize_warp_adapter(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::Cpu
        ));
    }
}
