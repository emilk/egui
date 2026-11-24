#![cfg(all(windows, feature = "wgpu"))]

use std::sync::{Arc, Barrier};

use egui_kittest::Harness;

fn warp_setup() -> egui_wgpu::WgpuSetup {
    let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = wgpu::Backends::DX12;
    setup.native_adapter_selector = Some(Arc::new(|adapters, _surface| {
        adapters
            .iter()
            .find(|adapter| {
                let info = adapter.get_info();
                info.backend == wgpu::Backend::Dx12 && info.device_type == wgpu::DeviceType::Cpu
            })
            .cloned()
            .ok_or_else(|| "No DirectX 12 software adapter found".to_owned())
    }));
    egui_wgpu::WgpuSetup::CreateNew(setup)
}

// Opt-in stress test: requires Windows WARP and repeatedly creates independent devices.
#[test]
#[ignore = "requires Windows WARP; run explicitly to stress concurrent harness rendering"]
fn concurrent_warp_harness_rendering() {
    for _ in 0..10 {
        let start = Barrier::new(2);
        std::thread::scope(|scope| {
            for _ in 0..2 {
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    for _ in 0..4 {
                        let mut harness = Harness::builder()
                            .with_size(egui::vec2(800.0, 600.0))
                            .wgpu_setup(warp_setup())
                            .build_ui(|ui| {
                                ui.label("trivial");
                            });

                        harness.run_steps(3);
                        // Align GPU rendering, not just thread startup.
                        start.wait();
                        let image = harness.render().expect("WARP rendering failed");
                        assert_eq!(image.dimensions(), (800, 600));
                        assert!(
                            image.pixels().any(|pixel| pixel[3] != 0),
                            "WARP returned a fully transparent image"
                        );
                    }
                });
            }
        });
    }
}

#[test]
#[ignore = "requires Windows WARP; run explicitly to stress persistent devices"]
fn concurrent_warp_persistent_devices() {
    let start = Barrier::new(2);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            let start = &start;
            scope.spawn(move || {
                let mut harness = Harness::builder()
                    .with_size(egui::vec2(800.0, 600.0))
                    .wgpu_setup(warp_setup())
                    .build_ui(|ui| {
                        ui.label("trivial");
                    });

                for _ in 0..40 {
                    // Advance the pass so render does not return a cached image.
                    harness.run_steps(3);
                    // All devices must be created before either thread starts rendering.
                    // Align every render to test genuinely concurrent draw/readback.
                    start.wait();
                    let image = harness.render().expect("WARP rendering failed");
                    assert_eq!(image.dimensions(), (800, 600));
                    assert!(
                        image.pixels().any(|pixel| pixel[3] != 0),
                        "WARP returned a fully transparent image"
                    );
                }
            });
        }
    });
}

#[test]
#[ignore = "requires Windows WARP; run explicitly to stress render-state creation"]
fn concurrent_warp_render_state_creation() {
    let start = Barrier::new(2);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            let start = &start;
            scope.spawn(move || {
                start.wait();
                for _ in 0..40 {
                    let state = egui_kittest::wgpu::create_render_state(
                        warp_setup(),
                        egui_wgpu::RendererOptions::PREDICTABLE,
                    );
                    drop(state);
                }
            });
        }
    });
}
