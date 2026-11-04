//! UI translation for the demo app and demo library.
//!
//! Strings in code stay English and are the lookup keys; a per-language catalogue (`*.tsv`, see
//! `i18n/ar.tsv` for the format) maps them to display text at render time. A string without a
//! translation is shown in English, so catalogues can cover a subset of the UI and unknown
//! strings never break it. The default language is English, so nothing changes unless a language
//! is picked.
//!
//! # Adding a language
//! 1. Add `xx.tsv` next to `ar.tsv` (copy its header; translate from the *meaning* of the English
//!    text, clean-room).
//! 2. Add one row to [`LANGUAGES`] (code, native name, catalogue, plural rule).
//!
//! # Looking strings up
//! - [`t`]: a plain string in the current language. [`tr`]: for a specific language (tests).
//! - [`fmt`]: fill `{name}` placeholders in a (possibly translated) template; translators may
//!   reorder placeholders freely. Rust format specs (`{:.0}` etc) never go in a catalogue: the
//!   value is pre-formatted in Rust and passed as an argument.
//! - [`split_template`]: split a translated template around one `{name}` placeholder, for a
//!   sentence that embeds a widget (a hyperlink) rather than plain text.
//!
//! The module is std-only, so it builds everywhere [`egui_demo_lib`](crate) does, including wasm.

use core::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

/// One supported UI language.
pub struct LangInfo {
    /// BCP 47 code, lowercase (`en`, `ar`). Also the value persisted in the app state.
    pub code: &'static str,
    /// The language's name in itself, shown in the language picker.
    pub name: &'static str,
    /// Raw TSV catalogue contents (empty for English, the source language).
    pub source: &'static str,
    /// Plural-form index for a count. English: 0 = one, 1 = other. Arabic uses the six CLDR
    /// forms (0 zero, 1 one, 2 two, 3 few, 4 many, 5 other).
    pub plural: fn(u64) -> usize,
    /// Parsed catalogue, built once on first use.
    catalog: OnceLock<Catalog>,
}

/// English: 1 → one, everything else → other.
fn plural_one_other(n: u64) -> usize {
    usize::from(n != 1)
}

/// Arabic (CLDR): 0 → zero, 1 → one, 2 → two, n % 100 in 3–10 → few, in 11–99 → many, else other.
fn plural_arabic(n: u64) -> usize {
    match (n, n % 100) {
        (0, _) => 0,
        (1, _) => 1,
        (2, _) => 2,
        (_, 3..=10) => 3,
        (_, 11..=99) => 4,
        _ => 5,
    }
}

/// The registry. English first: it is the fallback and the source language. (A fixed-size array
/// rather than a slice: a `static` borrow of a temporary containing a `OnceLock` is not allowed.)
pub static LANGUAGES: [LangInfo; 2] = [
    LangInfo {
        code: "en",
        name: "English",
        source: "",
        plural: plural_one_other,
        catalog: OnceLock::new(),
    },
    LangInfo {
        code: "ar",
        name: "العربية",
        source: include_str!("i18n/ar.tsv"),
        plural: plural_arabic,
        catalog: OnceLock::new(),
    },
];

impl LangInfo {
    fn catalog(&self) -> &Catalog {
        self.catalog.get_or_init(|| Catalog::parse(self.source))
    }
}

/// A language the UI can be shown in (a handle into [`LANGUAGES`]). `Copy`, `Eq` by code.
#[derive(Clone, Copy)]
pub struct Lang(&'static LangInfo);

impl core::fmt::Debug for Lang {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Lang({})", self.0.code)
    }
}

impl PartialEq for Lang {
    fn eq(&self, other: &Self) -> bool {
        self.0.code == other.0.code
    }
}

impl Eq for Lang {}

impl Lang {
    /// English, the source language and the fallback.
    pub const EN: Self = Self(&LANGUAGES[0]);

    /// A language by its exact code; `None` when unregistered.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        LANGUAGES.iter().find(|l| l.code == code).map(Self)
    }

    /// Every registered language, English first.
    pub fn all() -> impl Iterator<Item = Self> {
        LANGUAGES.iter().map(Self)
    }

    /// The BCP 47 code.
    #[must_use]
    pub fn code(self) -> &'static str {
        self.0.code
    }

    /// The language's name in itself.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0.name
    }

    fn catalog(self) -> &'static Catalog {
        self.0.catalog()
    }
}

/// A parsed catalogue: English source → translation. Malformed lines are skipped, so a bad
/// translation never breaks the UI.
#[derive(Default)]
struct Catalog {
    plain: HashMap<String, String>,
}

/// `catalog.rs`-style unescaping: `\n`, `\t` and `\\` inside a column; anything else (including a
/// trailing backslash) is kept as written.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            // `\\` is an escaped backslash; a lone trailing `\` is kept as written.
            Some('\\') | None => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    out
}

impl Catalog {
    /// Parse a catalogue file. Malformed lines are skipped with a warning, never a panic; the
    /// tests insist the bundled catalogues have none.
    fn parse(text: &str) -> Self {
        let mut catalog = Self::default();
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let mut cols = line.split('\t');
            match (cols.next(), cols.next(), cols.next(), cols.next()) {
                (Some(""), Some(src), Some(tr), None) if !src.is_empty() && !tr.is_empty() => {
                    catalog.plain.insert(unescape(src), unescape(tr));
                }
                _ => {
                    log::warn!(
                        "translation catalogue line {}: expected `<TAB>source<TAB>translation` \
                         (an empty context column); line skipped",
                        n + 1
                    );
                }
            }
        }
        catalog
    }

    fn plain(&self, s: &str) -> Option<&str> {
        self.plain.get(s).map(String::as_str)
    }
}

thread_local! {
    // Independent app/test threads must not change each other's drawing language.
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// Set the UI language for drawing (the demo app calls this once per frame from its persisted
/// state), so widgets can translate without every call site carrying a language around.
pub fn set_current(lang: Lang) {
    CURRENT.set(lang);
}

/// The language the UI is drawn in.
#[must_use]
pub fn current() -> Lang {
    CURRENT.get()
}

/// Translate an English UI string into the current language; unknown strings come back unchanged.
#[must_use]
pub fn t(s: &str) -> &str {
    tr(current(), s)
}

/// Translate an English UI string for a specific language; unknown strings come back unchanged.
#[must_use]
pub fn tr(lang: Lang, s: &str) -> &str {
    lang.catalog().plain(s).unwrap_or(s)
}

/// Fill `{name}` placeholders in a (possibly translated) template. Unknown placeholders are left
/// as written, so a catalogue row and its arguments can't silently disagree.
#[must_use]
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// Split a translated `{name}` template around its placeholder, for a sentence that embeds one
/// widget (a hyperlink).
///
/// Render `head`, then the widget with its own label, then `tail`, all inside the same
/// `horizontal_wrapped`. Whitespace is preserved exactly: head and tail keep the template's bytes
/// around the placeholder, which matters where `item_spacing.x` is zeroed and a fragment's space
/// is the only separator. `(template, "")` when the placeholder is absent.
#[must_use]
pub fn split_template<'t>(template: &'t str, name: &str) -> (&'t str, &'t str) {
    let placeholder = format!("{{{name}}}");
    match template.find(&placeholder) {
        Some(pos) => {
            let after = pos + placeholder.len();
            (&template[..pos], &template[after..])
        }
        None => (template, ""),
    }
}

/// The plural form index of `n` under `lang`'s rule (see [`LangInfo::plural`]).
#[must_use]
pub fn plural_index(lang: Lang, n: u64) -> usize {
    (lang.0.plural)(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::View as _;

    /// The four `fmt()` templates and the three link-embedding sentences keep their placeholders
    /// identical on both sides; everything else in a catalogue has no braces.
    fn placeholders(s: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut rest = s;
        while let Some(a) = rest.find('{') {
            let after = &rest[a + 1..];
            match after.find('}') {
                Some(b) => {
                    out.push(&after[..b]);
                    rest = &after[b + 1..];
                }
                None => break,
            }
        }
        out
    }

    /// The whitespace run at the end of `s` (the separator some fragments rely on).
    fn whitespace_suffix(s: &str) -> &str {
        let cut = s
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_whitespace())
            .map_or(0, |(i, c)| i + c.len_utf8());
        &s[cut..]
    }

    /// The whitespace run at the start of `s`.
    fn whitespace_prefix(s: &str) -> &str {
        let cut = s
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
            .map_or(s.len(), |(i, _)| i);
        &s[..cut]
    }

    /// The leading emoji (plus variation selector) of a display name, if any.
    fn emoji_prefix(s: &str) -> Option<&str> {
        if s.chars().next().is_some_and(|c| !c.is_ascii()) {
            s.split_once(' ').map(|(prefix, _)| prefix)
        } else {
            None
        }
    }

    /// Every English string the v1 Arabic catalogue must cover: the files of the touch table.
    /// A key missing from `ar.tsv` (or present but identical to its English source, i.e. an
    /// untranslated placeholder row) fails this test, so coverage is a gate, not a hope.
    const SCOPED_KEYS: &[&str] = &[
        // demo_app_windows.rs — the 42 sidebar names
        "✨ Demos",
        "〰 Band",
        "〰️ Bézier Curve",
        "⌨️ Code Editor",
        "⌨️ Code Example",
        "😀 Completion",
        "♫ Dancing Strings",
        "✋ Drag and Drop",
        "🖥️ Extra Viewport",
        "🔤 Font Book",
        "▣ Frame",
        "✨ Highlighting",
        "\u{20E3} Interactive Container",
        "✨ Misc Demos",
        "💭 Modals",
        "👌 Multi Touch",
        "🖊️ Painting",
        "📰 Panels",
        "\u{2755} Popups",
        "🔍 Scene",
        "📷 Screenshot",
        "↕️ Scrolling",
        "↔️ Sliders",
        "▣ Strip",
        "☰ Table",
        "✏️ TextEdit",
        "📄 Text Layout",
        "💬 Tooltips",
        "⟲ Undo Redo",
        "🗄️ Widget Gallery",
        "🔲 Window Options",
        "About egui",
        "Clipboard Test",
        "Cursor Test",
        "Grid Test",
        "ID Test",
        "Input Event History",
        "Input Test",
        "Layout Test",
        "Manual Layout Test",
        "SVG Test",
        "Tessellation Test",
        "Window Resize Test",
        // demo_app_windows.rs — the rest of the demo shell
        "Demos",
        "Tests",
        "Continue to the demo!",
        "⏷ demos",
        "Organize windows",
        "File",
        "Current zoom: {zoom}%",
        "The UI zoom level, on top of the operating system's default value",
        "Organize Windows",
        "Reset egui memory",
        "Forget scroll, positions, sizes etc",
        // wrap_app.rs — app chrome
        "📝 EasyMark editor",
        "⬇️ HTTP",
        "🕑 Fractal Clock",
        "🖼️ Image Viewer",
        "🔺 3D painting",
        "🎨 Rendering test",
        "💻 Backend",
        "Reset egui",
        "Reset everything",
        "Dropping files:\n",
        "Dropped files",
        "🌐 Language",
        // backend_panel.rs
        "egui windows:",
        "Press down all modifiers and hover a widget to see a callstack for it",
        "🔈 Screen reader",
        "Experimental feature: checking this will turn on the screen reader on supported platforms",
        "Quit",
        "Mode:",
        "Reactive",
        "Repaint when there are animations or input (e.g. mouse movement)",
        "Continuous",
        "Repaint everything each frame",
        "Repainting the UI each frame. FPS: {fps}",
        "Only running UI code when there are animations or input.",
        "More…",
        "Total ui frames:",
        "Total ui passes:",
        "Wait 2s, then request repaint after another 3s",
        "Request discard",
        "Discard denied!",
        "egui running inside {eframe}.",
        "Web info (location)",
        "Renderer:",
        "Backend:",
        "Device Type:",
        "Name:",
        "Driver:",
        "Driver info:",
        "Vendor:",
        "Device:",
        "PCI Bus ID:",
        "Subgroup size:",
        "Transient saves memory:",
        "Limit bucket:",
        "Others:",
        "🔳 Fullscreen (F11)",
        "Fullscreen the window",
        "Resize to…",
        "Present mode:",
        "Desired max frame latency:",
        "Default",
        "Lower value = lower latency",
        "Higher value = higher throughput/FPS",
        "🔧 Settings",
        "🔍 Inspection",
        "📝 Memory",
        "📤 Output Events",
        // frame_history.rs
        "Mean CPU usage: {cpu} ms / frame",
        "Includes all app logic, egui layout, tessellation, and rendering.\nDoes not include waiting for vsync.",
        "📊 CPU usage history",
        "egui CPU usage history",
        // about.rs
        "egui is an immediate mode GUI library written in Rust. egui runs natively on {os}, and on the web it is compiled to WebAssembly and rendered with WebGL or WebGPU.",
        " Everything you see is rendered as textured triangles. There is no DOM, HTML, JS or CSS. Just Rust.",
        "egui is easy to use, portable, and fast.",
        "This is how you create a button in egui:",
        "There are no callbacks or messages, and no button state to store. ",
        "Read more about immediate mode {here}.",
        "here",
        "egui development is sponsored by {rerun}, a startup building a data platform for robotics. ",
        "For an example of a professional egui app, run {viewer} (in your browser!).",
        "Links",
        "egui documentation",
        // widget_gallery.rs
        "Visible",
        "Uncheck to hide all the widgets.",
        "Interactive",
        "Uncheck to inspect how the widgets look when disabled.",
        "Opacity",
        "Reduce this value to make widgets semi-transparent",
        "The full egui documentation.\nYou can also click the different widgets names in the left column.",
        "Source code of the widget gallery",
        "\u{e624} egui on GitHub",
        "Welcome to the widget gallery!",
        "Label",
        "Hyperlink",
        "TextEdit",
        "Button",
        "Link",
        "Checkbox",
        "RadioButton",
        "SelectableLabel",
        "ComboBox",
        "Slider",
        "RangeSlider",
        "DragValue",
        "ProgressBar",
        "Color picker",
        "Image",
        "Button with image",
        "DatePickerButton",
        "Separator",
        "CollapsingHeader",
        "Spinner",
        "Write something here",
        "Click me!",
        "First",
        "Second",
        "Third",
        "Take your pick",
        "The progress bar can be animated!",
        "Click to see what is hidden!",
        "It's a ",
        "Custom widget",
        "It's easy to create your own widgets!\nThis toggle switch is just 15 lines of code.",
        "Search egui docs for",
        // Whitelist: strings that legitimately read the same in Arabic, with the reason.
        // (none — every scoped key has a real translation)
    ];

    /// Untranslated placeholder rows (translation == key), each with a reason. Must stay empty.
    fn identity_whitelist() -> &'static [&'static str] {
        &["⬇️ HTTP"] // a technology name that reads the same in Arabic
    }

    /// The Arabic language handle (`unwrap` is fine in tests: it is in the registry).
    fn ar() -> Lang {
        Lang::from_code("ar").unwrap()
    }

    fn ar_entries() -> Vec<(String, String)> {
        ar().catalog()
            .plain
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    #[test]
    fn catalog_parses_and_matches_its_keys() {
        let entries = ar_entries();
        assert!(!entries.is_empty());

        for (key, translation) in &entries {
            // Placeholders appear on both sides, and Rust format specs never enter a catalogue.
            if key.contains('{') || translation.contains('{') {
                assert_eq!(placeholders(key), placeholders(translation), "key: {key:?}");
            }
            assert!(!key.contains("{:"), "Rust format spec in key: {key:?}");

            // Leading/trailing whitespace is load-bearing (some fragments are separated only by
            // it, where `item_spacing.x` is zeroed), so translations preserve it byte-exactly.
            if key.ends_with(char::is_whitespace) {
                assert_eq!(
                    whitespace_suffix(key),
                    whitespace_suffix(translation),
                    "trailing whitespace of: {key:?}"
                );
            }
            if key.starts_with(char::is_whitespace) {
                assert_eq!(
                    whitespace_prefix(key),
                    whitespace_prefix(translation),
                    "leading whitespace of: {key:?}"
                );
            }

            // A trailing "…" of a dialog command is kept.
            assert_eq!(
                key.ends_with('…'),
                translation.ends_with('…'),
                "key: {key:?}"
            );

            // Emoji prefixes are byte-identical.
            if let Some(prefix) = emoji_prefix(key) {
                assert!(
                    translation.starts_with(prefix),
                    "translation of {key:?} lost the emoji prefix {prefix:?}"
                );
            }
        }
    }

    #[test]
    fn every_scoped_key_resolves_in_arabic() {
        for key in SCOPED_KEYS {
            let translated = tr(ar(), key);
            assert!(!translated.is_empty(), "empty translation for {key:?}");
            if !identity_whitelist().contains(key) {
                assert_ne!(translated, *key, "missing or untranslated key: {key:?}");
            }
        }
    }

    #[test]
    fn templates_and_links_round_trip() {
        // Each `fmt()` template filled with sample args yields today's English `format!` output.
        assert_eq!(
            fmt(tr(Lang::EN, "Current zoom: {zoom}%"), &[("zoom", "100")],),
            "Current zoom: 100%"
        );
        assert_eq!(
            fmt(
                tr(Lang::EN, "Repainting the UI each frame. FPS: {fps}"),
                &[("fps", "60.0")],
            ),
            "Repainting the UI each frame. FPS: 60.0"
        );
        assert_eq!(
            fmt(
                tr(Lang::EN, "Mean CPU usage: {cpu} ms / frame"),
                &[("cpu", "1.23")],
            ),
            "Mean CPU usage: 1.23 ms / frame"
        );
        assert_eq!(
            fmt(
                tr(
                    Lang::EN,
                    "egui is an immediate mode GUI library written in Rust. egui runs natively on {os}, and on the web it is compiled to WebAssembly and rendered with WebGL or WebGPU."
                ),
                &[("os", "\u{1F34E}\u{1F427}\u{1FA9C}")],
            ),
            "egui is an immediate mode GUI library written in Rust. egui runs natively on \u{1F34E}\u{1F427}\u{1FA9C}, and on the web it is compiled to WebAssembly and rendered with WebGL or WebGPU."
        );

        // `split_template` keeps the template's exact whitespace, in English...
        let (head, tail) = split_template(
            "egui development is sponsored by {rerun}, a startup building a data platform for robotics. ",
            "rerun",
        );
        assert!(head.ends_with("sponsored by "));
        assert!(tail.ends_with("robotics. "));
        assert_eq!(
            format!("{head}Rerun.io{tail}"),
            "egui development is sponsored by Rerun.io, a startup building a data platform for robotics. "
        );

        // ...and with the placeholder moved anywhere in a translated sentence.
        let (head, tail) = split_template("بداية الجملة {rerun} ونهايتها. ", "rerun");
        assert_eq!(head, "بداية الجملة ");
        assert_eq!(tail, " ونهايتها. ");
        let (head, tail) = split_template("{here} في البداية.", "here");
        assert_eq!(head, "");
        assert_eq!(tail, " في البداية.");

        // Absent placeholder: the whole template is the head.
        assert_eq!(
            split_template("no placeholder", "x"),
            ("no placeholder", "")
        );

        // Unknown placeholders are left as written.
        assert_eq!(fmt("Hello {unknown}", &[("a", "b")]), "Hello {unknown}");
    }

    #[test]
    fn language_switch_round_trip() {
        set_current(Lang::EN);
        assert_eq!(t("Demos"), "Demos");
        assert_eq!(t("definitely not a key"), "definitely not a key");

        set_current(ar());
        assert_ne!(t("Demos"), "Demos");
        assert_eq!(t("definitely not a key"), "definitely not a key");

        set_current(Lang::EN);
        assert_eq!(t("Demos"), "Demos");
    }

    /// Mirrors `test_egui_e2e`: render the demo app (with the About, Code Example and Widget
    /// Gallery windows open, as by default) and the Widget Gallery directly, in Arabic. Guards
    /// against catalogue-driven panics; glyph shapes are system-font dependent and not asserted.
    #[test]
    fn render_without_panic_in_arabic() {
        set_current(ar());

        let ctx = egui::Context::default();
        let raw_input = egui::RawInput::default();
        const NUM_FRAMES: usize = 5;

        let mut demo_windows = crate::DemoWindows::default();
        for _ in 0..NUM_FRAMES {
            let mut full_output = ctx.run_ui(raw_input.clone(), |ui| {
                demo_windows.ui(ui);
            });
            let clipped_primitives =
                ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            assert!(!clipped_primitives.is_empty());
            full_output.textures_delta.clear(); // Don't panic on drop with unapplied deltas
        }

        let mut gallery = crate::WidgetGallery::default();
        for _ in 0..NUM_FRAMES {
            let mut full_output = ctx.run_ui(raw_input.clone(), |ui| {
                gallery.ui(ui);
            });
            let clipped_primitives =
                ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            assert!(!clipped_primitives.is_empty());
            full_output.textures_delta.clear();
        }

        set_current(Lang::EN);
    }

    /// Malformed catalogue data never panics: bad lines are skipped and lookups fall back to
    /// English.
    #[test]
    fn malformed_catalogue_falls_back_to_english() {
        let garbage = "no tabs here\n\t\nmissing\tcolumns\n\tok\t\n\t\t\n";
        let catalog = Catalog::parse(garbage); // must not panic
        assert!(catalog.plain.is_empty());

        // A usable row among bad ones is still found; the rest fall back.
        let mixed = "bad line\n\tDemos\tالعروض التوضيحية\n\t\n";
        let catalog = Catalog::parse(mixed);
        assert_eq!(catalog.plain("Demos"), Some("العروض التوضيحية"));
        assert_eq!(catalog.plain("not there"), None);
    }

    #[test]
    fn plural_forms_follow_cldr_arabic() {
        assert_eq!(plural_index(ar(), 0), 0); // zero
        assert_eq!(plural_index(ar(), 1), 1); // one
        assert_eq!(plural_index(ar(), 2), 2); // two
        assert_eq!(plural_index(ar(), 5), 3); // few (3–10)
        assert_eq!(plural_index(ar(), 11), 4); // many (11–99)
        assert_eq!(plural_index(ar(), 100), 5); // other
        assert_eq!(plural_index(Lang::EN, 1), 0);
        assert_eq!(plural_index(Lang::EN, 2), 1);
    }
}
