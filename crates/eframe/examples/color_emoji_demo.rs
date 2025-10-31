
use eframe::egui;
  use std::sync::Arc;

  pub fn main() -> eframe::Result<()> {
      let font_bytes = std::fs::read("NotoColorEmoji.ttf")
          .expect("download NotoColorEmoji.ttf next to the repo root");
      let mut fonts = egui::FontDefinitions::default();
      fonts.font_data.insert(
          "noto-color".into(),
          Arc::new(egui::FontData::from_owned(font_bytes)),
      );
      fonts
          .families
          .get_mut(&egui::FontFamily::Proportional)
          .unwrap()
          .insert(0, "noto-color".into());

      eframe::run_simple_native("Color emoji demo", eframe::NativeOptions::default(), move
  |ctx, _| {
          ctx.set_fonts(fonts.clone());
          egui::CentralPanel::default().show(ctx, |ui| {
              ui.heading("Color emoji test");
              ui.label("😀 😁 😂 🤣 😍 🤖 🧠 🌈");
              ui.colored_label(egui::Color32::LIGHT_BLUE, "Tinted text keeps emoji colors
  🌟🍀🍣");
          });
      })
  }

