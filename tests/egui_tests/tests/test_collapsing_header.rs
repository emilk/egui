//! Tests for [`egui::CollapsingHeader`].

use egui::{Atom, AtomExt as _, CollapsingHeader, Image, ImageSource, TextWrapMode, Vec2};
use egui_kittest::{Harness, kittest::Queryable as _};

const LONG_TITLE: &str = "Plan (1/3) Find gaps in the camera timestamps across all fifty episodes";

const EFRAME_ICON: ImageSource<'static> =
    egui::include_image!("../../../crates/eframe/data/icon.png");
const RING: ImageSource<'static> =
    egui::include_image!("../../../crates/egui_demo_lib/data/ring.png");

fn image(source: ImageSource<'static>, alt_text: &str) -> Atom<'static> {
    Image::new(source)
        .alt_text(alt_text)
        .atom_size(Vec2::splat(16.0))
}

/// A truncated header stays within its parent, and still reports its full title.
#[test]
fn collapsing_header_truncates_long_title() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 60.0))
        .build_ui(|ui| {
            CollapsingHeader::new(LONG_TITLE)
                .wrap_mode(TextWrapMode::Truncate)
                .show(ui, |ui| ui.label("Body"));
        });
    harness.run();

    let header = harness.get_by_label(LONG_TITLE);
    assert!(
        header.rect().right() <= 200.0,
        "header overflows: {:?}",
        header.rect()
    );
    harness.snapshot("collapsing_header_truncated");
}

/// Images and text are laid out after the icon, and a grow atom right-aligns what follows it.
#[test]
fn collapsing_header_with_atoms() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 100.0))
        .build_ui(|ui| {
            egui_extras::install_image_loaders(ui.ctx());
            CollapsingHeader::new((image(EFRAME_ICON, "eframe"), "Image and text"))
                .default_open(true)
                .show(ui, |ui| ui.label("Body"));

            ui.visuals_mut().collapsing_header_frame = true;
            CollapsingHeader::new(("Left", Atom::grow(), "Right")).show(ui, |ui| ui.label("Body"));
        });
    harness.run();

    harness.get_by_label("Image and text");
    harness.get_by_label("Left Right");
    harness.snapshot("collapsing_header_with_atoms");
}

/// Headers without text get their ids from their images, so they toggle independently.
#[test]
fn collapsing_headers_with_only_images() {
    let mut harness = Harness::new_ui(|ui| {
        CollapsingHeader::new(image(EFRAME_ICON, "eframe")).show(ui, |ui| ui.label("eframe body"));
        CollapsingHeader::new(image(RING, "ring")).show(ui, |ui| ui.label("ring body"));
    });
    harness.run();
    assert!(harness.query_by_label("eframe body").is_none());

    harness.get_by_label("ring").click();
    harness.run();

    assert!(harness.query_by_label("eframe body").is_none());
    harness.get_by_label("ring body");
}
