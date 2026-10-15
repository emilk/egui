# egui_system_fonts

[![Latest version](https://img.shields.io/crates/v/egui_system_fonts.svg)](https://crates.io/crates/egui_system_fonts)
[![Documentation](https://docs.rs/egui_system_fonts/badge.svg)](https://docs.rs/egui_system_fonts)

Load system fonts on demand for characters that the fonts installed in [egui](https://github.com/emilk/egui) lack.

The default egui fonts cover Latin, Greek, and Cyrillic. When egui is asked to render e.g. Japanese, Arabic, or Devanagari, it draws `◻`. `egui_system_fonts` provides a [`SystemFontProvider`] that asks the operating system for a font that covers the character, using the same font fallback as native apps (CoreText, DirectWrite, fontconfig, or Android's `fonts.xml`, via [`fontique`](https://crates.io/crates/fontique)). The font file is memory-mapped and installed as a fallback font in egui.

`eframe` installs it on native by default (feature `system_fonts`). Other integrations can add it themselves:

```rust
egui_ctx.add_font_provider(std::sync::Arc::new(egui_system_fonts::SystemFontProvider::new()));
```

System emoji fonts are skipped for now, since they contain bitmaps or color glyphs that epaint cannot render yet.
