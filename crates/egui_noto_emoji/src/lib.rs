//! Optional emoji bundle for egui.
//!
//! This crate keeps the heavy atlas and loader in a separate crate so core egui stays lean.

mod store;

pub use store::{EmojiEntry, EmojiResolution, EmojiStore};

use egui::Context;

/// Register the bundled emoji atlas on the provided [`egui::Context`].
///
/// Call this once during startup, right after you have access to the context.
pub fn install(ctx: &Context) {
    register_store(ctx, &EmojiStore::builtin());
}

/// Register every emoji entry in a store.
pub fn register_store(ctx: &Context, store: &EmojiStore) {
    for entry in store.entries() {
        register_entry(ctx, entry);
    }
}

/// Register a single emoji sprite with all its resolutions.
///
/// ASCII digits/#/* are kept rendered by the base fonts (keycap emoji components).
pub fn register_entry(ctx: &Context, entry: &EmojiEntry) {
    if is_keycap_component(entry.ch()) {
        return;
    }

    // Use multi-resolution registration for sharp rendering at all sizes
    ctx.register_color_glyph_multi(entry.ch(), entry.resolutions());
}

/// Single ASCII characters that are part of the keycap emoji sequences.
/// Those sequences require multiple code points, so keep the plain glyphs rendered by the base fonts.
fn is_keycap_component(c: char) -> bool {
    matches!(c, '#' | '*' | '0'..='9')
}
