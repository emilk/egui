//! Tests for [`egui_kittest::HarnessBuilder::with_always_render`] and the per-pass render cache.

#![cfg(any(feature = "wgpu", feature = "snapshot"))]

use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use egui_kittest::{Harness, TestRenderer};

/// A renderer that draws nothing and only counts how often it was asked to.
#[derive(Default)]
struct CountingRenderer {
    renders: Arc<AtomicUsize>,
}

impl TestRenderer for CountingRenderer {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        // Dropping an unapplied delta panics, and we have no textures to update.
        delta.clear();
    }

    fn render(
        &mut self,
        _ctx: &egui::Context,
        _output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        self.renders.fetch_add(1, Ordering::Relaxed);
        Ok(image::RgbaImage::new(1, 1))
    }
}

fn harness_with_counter(always_render: bool) -> (Harness<'static>, Arc<AtomicUsize>) {
    let renders = Arc::new(AtomicUsize::new(0));
    let harness = Harness::builder()
        .with_always_render(always_render)
        .renderer(CountingRenderer {
            renders: Arc::clone(&renders),
        })
        .build_ui(|ui| {
            ui.label("Hello!");
        });
    (harness, renders)
}

#[test]
fn renders_nothing_by_default() {
    let (mut harness, renders) = harness_with_counter(false);
    let before = renders.load(Ordering::Relaxed);

    harness.run_steps(4);

    assert_eq!(
        renders.load(Ordering::Relaxed),
        before,
        "without `always_render` a pass should not be rendered"
    );
}

#[test]
fn renders_every_pass_when_always_render_is_on() {
    let (mut harness, renders) = harness_with_counter(true);
    let before = renders.load(Ordering::Relaxed);

    harness.run_steps(4);

    assert_eq!(
        renders.load(Ordering::Relaxed) - before,
        4,
        "`always_render` should render each pass exactly once"
    );
}

#[test]
fn a_pass_is_rendered_at_most_once() {
    let (mut harness, renders) = harness_with_counter(true);
    let before = renders.load(Ordering::Relaxed);

    harness.run_steps(1);
    // A paint callback may do GPU work of its own, so asking again must not render again.
    harness.render().expect("render");
    harness.render().expect("render");

    assert_eq!(
        renders.load(Ordering::Relaxed) - before,
        1,
        "the image of a pass should be rendered once and then reused"
    );
}

#[test]
fn set_always_render_takes_effect_from_the_next_pass() {
    let (mut harness, renders) = harness_with_counter(false);
    harness.run_steps(2);
    let before = renders.load(Ordering::Relaxed);

    harness.set_always_render(true);
    harness.run_steps(3);

    assert_eq!(
        renders.load(Ordering::Relaxed) - before,
        3,
        "turning `always_render` on should render the passes that follow"
    );
}
