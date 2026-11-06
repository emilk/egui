//! RTL font fixture support for tests, see [`HarnessBuilder::with_rtl_font`].
//!
//! [HarnessBuilder]: crate::HarnessBuilder

/// The environment variable [`HarnessBuilder::with_rtl_font`] reads the test font
/// path from.
///
/// [HarnessBuilder]: crate::HarnessBuilder
pub const RTL_FONT_ENV_VAR: &str = "KITTEST_RTL_FONT";

/// The name the font installed by [`HarnessBuilder::with_rtl_font`] is registered
/// under in [`egui::FontDefinitions`].
///
/// [HarnessBuilder]: crate::HarnessBuilder
pub const RTL_FONT_NAME: &str = "kittest-rtl-font";

/// The path of the Arabic-capable test font configured via [`RTL_FONT_ENV_VAR`],
/// if one is set.
///
/// A `~/`-prefix is expanded against `HOME` when that variable is set.
///
/// Always `None` on wasm, where the fixture is a no-op (see
/// [`HarnessBuilder::with_rtl_font`]).
///
/// [HarnessBuilder]: crate::HarnessBuilder
#[cfg(target_arch = "wasm32")]
pub fn rtl_font_path() -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn rtl_font_path() -> Option<std::path::PathBuf> {
    let raw = std::env::var_os(RTL_FONT_ENV_VAR)?;
    if raw.is_empty() {
        return None;
    }
    Some(expand_home(&std::path::PathBuf::from(raw)))
}

/// Expand a leading `~/` against `HOME`; anything else is returned unchanged.
#[cfg(not(target_arch = "wasm32"))]
fn expand_home(path: &std::path::Path) -> std::path::PathBuf {
    let Some(string) = path.to_str() else {
        return path.to_owned();
    };
    let Some(rest) = string.strip_prefix("~/") else {
        return path.to_owned();
    };
    match std::env::var_os("HOME") {
        Some(home) if !home.is_empty() => std::path::PathBuf::from(home).join(rest),
        _ => path.to_owned(),
    }
}

/// The context setup run by [`HarnessBuilder::with_rtl_font`].
///
/// [HarnessBuilder]: crate::HarnessBuilder
#[cfg(not(target_arch = "wasm32"))]
#[expect(clippy::print_stderr)] // Tests may run without a logger installed.
pub(crate) fn install_rtl_font(ctx: &egui::Context) {
    let Some(path) = rtl_font_path() else {
        eprintln!(
            "{RTL_FONT_ENV_VAR} is not set: no Arabic-capable test font is installed, so \
             assertions that need RTL-script glyphs will be skipped. Point it at an \
             Arabic-capable TTF (e.g. Noto Naskh Arabic, Noto Sans Arabic or Amiri, \
             all OFL-licensed) to run them."
        );
        return;
    };

    let font_data = match std::fs::read(&path) {
        Ok(font_data) => font_data,
        Err(err) => {
            eprintln!(
                "Failed to read {RTL_FONT_ENV_VAR}={}: {err}; assertions that need \
                 RTL-script glyphs will be skipped.",
                path.display()
            );
            return;
        }
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        RTL_FONT_NAME.to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(font_data)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push(RTL_FONT_NAME.to_owned());
    }
    ctx.set_fonts(fonts);
}
