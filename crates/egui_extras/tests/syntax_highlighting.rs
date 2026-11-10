//! Snapshots of the fallback highlighter, used when the `syntect` feature is off.
#![cfg(not(feature = "syntect"))]

use egui_extras::syntax_highlighting::{CodeTheme, code_view_ui};
use egui_kittest::{Harness, SnapshotResults};

const SAMPLES: &[(&str, &str)] = &[
    (
        "rs",
        "/* block */ fn f<'a>(s: &'a str) -> usize {\n    // line\n    s.len() + \"esc \\\" quote\".len()\n}",
    ),
    (
        "py",
        "def f(x):  # comment\n    \"\"\"Doc \"string\".\"\"\"\n    return 'it' if x else None",
    ),
    (
        "ts",
        "const s: string = `hi ${name}`; /* block */\nconsole.log(s); // line",
    ),
    ("json", "{\"key\": [1, true, null], \"esc\": \"a\\\"b\"}"),
    ("sh", "if [ -f \"$f\" ]; then echo 'yes'; fi  # comment"),
    ("sql", "SELECT name FROM t WHERE id = 'x' -- comment"),
    ("toml", "[package]\nname = 'egui'  # comment"),
    ("yaml", "key: \"value\"  # comment\nlist: [true, null]"),
];

#[test]
fn fallback_syntax_highlighting() {
    let mut results = SnapshotResults::new();
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(480.0, 1.0))
            .with_theme(theme)
            .build_ui(|ui| {
                let code_theme = CodeTheme::from_style(ui.style());
                for (language, code) in SAMPLES {
                    ui.weak(*language);
                    code_view_ui(ui, &code_theme, code, language);
                }
            });
        harness.run();
        harness.fit_contents();
        harness.snapshot(match theme {
            egui::Theme::Dark => "syntax_highlighting_dark",
            egui::Theme::Light => "syntax_highlighting_light",
        });
        results.extend_harness(&mut harness);
    }
}
