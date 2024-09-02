#[cfg(feature = "disable_text_snapping")]
#[test]
fn test_layout_sizes_without_snapping() {
    let fonts1_0 = epaint::text::Fonts::new(1.0, 1024, epaint::text::FontDefinitions::default());
    let fonts1_1 = epaint::text::Fonts::new(1.1, 1024, epaint::text::FontDefinitions::default());
    let wrapping = epaint::text::TextWrapping {
        max_width: 72.0 * 8.5 - 72.0,
        ..Default::default()
    };
    let mut job = epaint::text::LayoutJob {
        wrap: wrapping,
        ..Default::default()
    };
    job.append(
        "Hello, epaint! Thanks for the awesome GUI library!",
        0.0,
        epaint::text::TextFormat {
            font_id: epaint::FontId::new(10.0, epaint::FontFamily::Proportional),
            color: epaint::Color32::WHITE,
            ..Default::default()
        },
    );
    let galley1_0 = fonts1_0.layout_job(job.clone());
    let galley1_1 = fonts1_1.layout_job(job.clone());
    assert_eq!(galley1_0.rect.size(), galley1_1.rect.size());
}

#[test]
fn test_layout_sizes_with_snapping() {
    let fonts1_0 = epaint::text::Fonts::new(1.0, 1024, epaint::text::FontDefinitions::default());
    let fonts1_1 = epaint::text::Fonts::new(1.1, 1024, epaint::text::FontDefinitions::default());
    let wrapping = epaint::text::TextWrapping {
        max_width: 72.0 * 8.5 - 72.0,
        ..Default::default()
    };
    let mut job = epaint::text::LayoutJob {
        wrap: wrapping,
        ..Default::default()
    };
    job.append(
        "Hello, epaint! Thanks for the awesome GUI library!",
        0.0,
        epaint::text::TextFormat {
            font_id: epaint::FontId::new(10.0, epaint::FontFamily::Proportional),
            color: epaint::Color32::WHITE,
            ..Default::default()
        },
    );
    let galley1_0 = fonts1_0.layout_job(job.clone());
    let galley1_1 = fonts1_1.layout_job(job.clone());
    assert_ne!(galley1_0.rect.height(), galley1_1.rect.height());
    assert_ne!(galley1_0.rect.width(), galley1_1.rect.width());
}
