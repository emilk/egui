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
//! 2. Add one row to [`LANGUAGES`] (code, native name, catalogue, plural rule, direction).
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
    /// Whether the language reads right-to-left, so the app can mirror the interface direction
    /// (through [`egui::Context::set_right_to_left`]) when it is selected.
    pub rtl: bool,
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
        rtl: false,
        catalog: OnceLock::new(),
    },
    LangInfo {
        code: "ar",
        name: "العربية",
        source: include_str!("i18n/ar.tsv"),
        plural: plural_arabic,
        rtl: true,
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

    /// Whether the language reads right-to-left.
    #[must_use]
    pub fn is_rtl(self) -> bool {
        self.0.rtl
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
    /// How many lines were skipped as malformed (the tests insist this stays 0 for the bundled
    /// catalogues).
    skipped: usize,
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
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let mut cols = line.split('\t');
            match (cols.next(), cols.next(), cols.next(), cols.next()) {
                (Some(""), Some(src), Some(tr), None) if !src.is_empty() && !tr.is_empty() => {
                    catalog.plain.insert(unescape(src), unescape(tr));
                }
                _ => {
                    catalog.skipped += 1;
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

/// Fill `{name}` placeholders in a (possibly translated) template.
///
/// Unknown placeholders are left as written, so a catalogue row and its arguments can't silently
/// disagree. One pass over the template: a value is inserted verbatim, so a value containing a
/// later placeholder is never substituted itself.
#[must_use]
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open + 1..].find('}').map(|c| open + 1 + c) else {
            break;
        };
        let name = &rest[open + 1..close];
        out.push_str(&rest[..open]);
        match args.iter().find(|(n, _)| *n == name) {
            Some((_, value)) => out.push_str(value),
            // Unknown placeholder: left as written.
            None => out.push_str(&rest[open..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
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
        // partition P2 (craft/i18n-ar-p2 @ 8b0ccce0): the text demos (text_edit, password, completion,
        // code_editor, code_example, highlighting, text_layout, about, font_book)
        "Edit this text",
        "Advanced usage of ",
        "Horizontal align:",
        "Left",
        "Center",
        "Right",
        "Vertical align:",
        "Top",
        "Bottom",
        "Type something!",
        "Selected text: ",
        "Press ctrl+Y to toggle the case of selected text (cmd+Y on Mac)",
        "Move cursor to the:",
        "start",
        "end",
        "Show/hide password",
        " over a ",
        ". Type a colon and the name of an emoji, e.g. ",
        ". The arrow keys select, Enter or Tab accepts, and Escape closes the popup.",
        "Type :smile: …",
        "An example of syntax highlighting in a TextEdit.",
        "Language:",
        "Syntax highlighting powered by ",
        "Compile the demo with the ",
        " feature to enable more accurate syntax highlighting using ",
        "Theme",
        "Code",
        "Example",
        "Name",
        " years",
        "Increment",
        "{name} is {age}",
        "This demo demonstrates highlighting a widget.",
        "Hover me to highlight the button!",
        "Hover the button to highlight the label!",
        "Max rows:",
        "Line-break:",
        "word boundaries",
        "anywhere",
        "Overflow character:",
        "None",
        "Extra letter spacing:",
        "Line height:",
        "Custom",
        " pixels",
        "Justify:",
        "Fill row width",
        "Text:",
        "On web, the browser draws missing glyphs instead. ",
        "On native, eframe falls back to looking for a matching system font. ",
        "Bundle fonts with ",
        "The selected family has {characters} characters, from {fonts} fonts",
        "In fallback order:",
        "Filter:",
        "unknown",
        "Hex",
        "Width",
        "Fonts",
        // partition P3 (craft/i18n-ar-p3 @ 6eb3ac24): the visual demos and app chrome
        // painting.rs
        "Stroke:",
        "Clear Painting",
        "Paint with your mouse/touch!",
        // paint_bezier.rs
        "Colors",
        "Fill color",
        "Curve Stroke",
        "Auxiliary Stroke",
        "Bounding Box Stroke",
        "Global tessellation options",
        "Quadratic Bézier",
        "Cubic Bézier",
        "Move the points by dragging them.",
        "Only convex curves can be accurately filled.",
        // dancing_strings.rs
        "Colored",
        "Demonstrates how a path can have varying color across its length.",
        // band.rs
        "An animated waveform rendered as a rotated band.",
        "Angle",
        "Animation speed",
        "Fill opacity",
        "Stroke opacity",
        "Stroke width",
        "Stroke kind",
        "Inside",
        "Middle",
        "Outside",
        // frame_demo.rs
        "Reset",
        "Content",
        // scene.rs
        "Scene",
        "You can pan by scrolling, and zoom using cmd-scroll. Double click on the background to reset view.",
        "Change mousewheel behavior to zoom instead of scroll",
        "Reset view",
        "You can put a widget anywhere",
        // scrolling.rs
        "Appearance",
        "Scroll to",
        "Scroll a lot of lines",
        "Scroll a large canvas",
        "Stick to end",
        "Bidirectional",
        "When to show scroll bars; resize the window to see the effect.",
        "Content length",
        "A lot of rows, but only the visible ones are laid out, so performance is still good:",
        "A lot of rows, but only the visible ones are painted, so performance is still good:",
        "This is row {}/{}",
        "This is row {}/{}, indented by {} pixels",
        "This shows how you can scroll to a specific item or pixel offset",
        "Scroll to a specific item index:",
        "Track Item",
        "Item align:",
        "None (Bring into view)",
        "Scroll to a specific offset:",
        "Scroll to top",
        "Scroll to bottom",
        "Scroll by",
        "This is item {item}",
        "Rows enter from the bottom, we want the scroll handle to start and stay at bottom unless moved",
        // sliders.rs
        "You can click a slider value to edit it with the keyboard.",
        "i32 demo slider",
        "f64 demo slider",
        "Sliders will intelligently pick how many decimals to show. You can always see the full precision value by hovering the value.",
        "Assign PI",
        "Slider range:",
        "left",
        "right",
        "Toggle trailing color",
        "When enabled, trailing color will be painted up until the handle.",
        "Use steps",
        "When enabled, the minimal value change would be restricted to a given step.",
        "Slider type:",
        "All numeric types (f32, usize, …) are supported.",
        "Slider orientation:",
        "Horizontal",
        "Vertical",
        "Logarithmic",
        "Logarithmic sliders are great for when you want to span a huge range, i.e. from zero to a million.",
        "Logarithmic sliders can include infinity and zero.",
        "Clamping:",
        "Never",
        "Edits",
        "Always",
        "If true, the slider will clamp incoming and outgoing values to the given range.",
        "If false, the slider can show values outside its range, and you cannot enter new values outside the range.",
        "Smart Aim",
        "Smart Aim will guide you towards round values when you drag the slider so you you are more likely to hit 250 than 247.23",
        // strip_demo.rs
        "width: 100%\nheight: 50px",
        "width: 50%\nheight: remaining",
        "width: 50%\nheight: 1/3 of the red region",
        "width: 120px\nheight: 60px",
        "width: 70px\n\nheight: 50%, but at least 60px.",
        // table_demo.rs
        "Striped",
        "Overline some rows",
        "Resizable columns",
        "Clickable rows",
        "Table type:",
        "Few, manual rows",
        "Thousands of rows of same height",
        "Thousands of rows of differing heights",
        "Num rows",
        "Row to scroll to",
        "Row",
        "Clipped text",
        "Expanding content",
        "Interaction",
        "Click me",
        "Extra thick row",
        "Normal row",
        "Thousands of rows of even height",
        "Row {row_index} has some long text that you may want to clip, or it will take up too much horizontal space!",
        // tooltips.rs
        "Tooltips",
        "The scroll area below has many labels with interactive tooltips. The purpose is to test that the tooltips close when you scroll.",
        "Try hovering a label below, then scroll!",
        "This is line {i}",
        "This tooltip is interactive, because the text in it is selectable.",
        "All labels in this demo have tooltips.",
        "Yes, even this one.",
        "Some widgets have multiple tooltips!",
        "The first tooltip.",
        "The second tooltip.",
        "Tooltips can contain interactive widgets.",
        "This tooltip contains a link:",
        "The tooltip has a tooltip in it!",
        "You can put selectable text in tooltips too.",
        "You can select this text.",
        "This tooltip shows at the mouse cursor.",
        "Move me around!!",
        "This tooltip was created with",
        "A different tooltip when widget is disabled.",
        "You can have different tooltips depending on whether or not a widget is enabled:",
        "Check the tooltip of the button below, and see how it changes depending on whether or not it is enabled.",
        "Enabled",
        "Controls whether or not the following button is enabled.",
        "Sometimes clickable",
        // undo_redo.rs
        "Text with undo/redo",
        "Checkbox with undo/redo",
        "⟲ Undo",
        "⟳ Redo",
        // drag_and_drop.rs
        "Item A",
        "Item B",
        "Item C",
        "Item D",
        "Item E",
        "Item F",
        "Item G",
        "Item H",
        "Item I",
        "Item J",
        "Item K",
        "This is a simple example of drag-and-drop in egui.",
        "Drag items between columns.",
        // multi_touch.rs
        "This demo only works on devices with multitouch support (e.g. mobiles, tablets, and trackpads).",
        "Try touch gestures Pinch/Stretch, Rotation, and Pressure with 2+ fingers.",
        "{num_touches}-finger touch",
        "Input source: {}",
        "cursor",
        "none",
        // modals.rs
        "Open User Modal",
        "Open Save Modal",
        "Click one of the buttons to open a modal.",
        "Modals have a backdrop and prevent interaction with the rest of the UI.",
        "You can show modals on top of each other and close the topmost modal with escape or by clicking outside the modal.",
        "Edit User",
        "Role",
        "user",
        "admin",
        "Save",
        "Cancel",
        "Save? Are you sure?",
        "Yes Please",
        "No Thanks",
        "Saving…",
        // popups.rs
        "Open…",
        "Popups can have submenus",
        "SubMenu",
        "Item",
        "Recursive",
        "SubMenus can be disabled",
        "I have an icon!",
        "Item1",
        "Item2",
        "Item3",
        "Item4",
        "Very long text for this item that should be wrapped",
        "Always CloseOnClickOutside",
        "Background",
        "Only visible when checked",
        "Remove myself",
        "Click, right-click and hover me!",
        "Popup contents",
        "Tooltips are popups, too!",
        "Reset to defaults",
        "parent",
        "child",
        "<Select Preset>",
        "Preset",
        "gap",
        "Closes when the user clicks anywhere (inside or outside)",
        "Closes when the user clicks outside the popup",
        "Close only when the button is clicked again",
        "Close behavior",
        // misc_demo_window.rs — collapsing headers and demos
        "Misc widgets",
        "Text layout",
        "System font fallback",
        "Text rotation",
        "Custom Collapsing Header",
        "Tree",
        "Repaint",
        "Checkboxes",
        "Checkboxes with empty labels take up very little space:",
        "checkbox",
        "Radiobuttons are similar:",
        "Checkboxes can be in an indeterminate state:",
        "Check/uncheck all",
        "Item {}",
        "Columns",
        "Column {} out of {}",
        "Delete this",
        "Test box rendering",
        "Resize",
        "Right-to-left",
        "This ui can be resized!",
        "Just pull the handle on the bottom left",
        "Just pull the handle on the bottom right",
        "Ui Stack",
        "Misc",
        "You can pretty easily paint your own small icons:",
        "Many circles of different sizes",
        // misc_demo_window.rs — label demo
        "Text can have",
        "color",
        "and tooltips.",
        "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
        "You can mix in other widgets into text, like",
        "this button",
        "The default font supports all latin and cyrillic characters (ИÅđ…), common math symbols (∫√∞²⅓…), and many emojis (💓🌟📐…).",
        "There is currently no support for right-to-left languages.",
        "See the 🔤 Font Book for more!",
        "There is also a monospace font.",
        "Labels containing long text can be set to elide the text that doesn't fit on a single line using `Label::truncate`. When hovered, the label will show the full text.",
        // misc_demo_window.rs — widgets (angle, password)
        "An angle:",
        "Each τ represents one turn (τ = 2π)",
        "The angle is stored in radians, but presented in degrees",
        "Password:",
        "See the example code for how to use egui to store UI state",
        // misc_demo_window.rs — repaint demo
        "Use this to verify if logic is correctly called while in background.",
        "Reset counts",
        "`ui`: {}, `logic`: {}",
        "`ui` is incremented in `App::ui` (only runs while visible), `logic` in `App::logic` (runs even while hidden).",
        "Repaint continuously (every frame)",
        "Repaint after",
        "delay",
        "In the background (during logic)",
        "Issue the repaint requests from `App::logic` (which runs even while hidden) instead of `App::ui` (which is skipped while hidden).\n\nWith this enabled, hide this tab for a while, then come back: the `logic` count will have kept climbing.",
        "Log each frame",
        "Log each `ui` and `logic` frame to the console.",
        // misc_demo_window.rs — colors, collapsing header, tree, ui stack, fonts
        "egui lets you edit colors stored as either sRGBA or linear RGBA and with or without premultiplied alpha",
        "corner_radius",
        "stroke_width",
        "num_boxes",
        "Example of a collapsing header with custom header:",
        "Click to select/unselect",
        "The body is always custom",
        "Normal collapsing header for comparison",
        "Nothing exciting here",
        "delete",
        "child #{i}",
        "root",
        "The",
        "core type is typically deeply nested in",
        "applications. To provide context to nested code, it maintains a stack with various information.\n\nThis is how the stack looks like here:",
        "id",
        "kind",
        "Hover on UI's ids to display their origin and max rect.",
        "Some non-latin chars to test font fallback:",
        "Arabic",
        "Chinese (simplified)",
        "Chinese (traditional)",
        "Devanagari",
        "Emoji",
        "Hebrew",
        "Japanese",
        "Korean",
        "Thai",
        // misc_demo_window.rs — text-layout fragments (whitespace is load-bearing)
        "This",
        " is a demonstration of ",
        "the egui text layout engine. ",
        "It supports ",
        "different ",
        "colors, ",
        "backgrounds, ",
        "mixing ",
        "fonts, ",
        "raised text, ",
        "with ",
        "underlining",
        " and ",
        "strikethrough",
        ". Of course, ",
        "you can",
        " mix these!",
        "angle",
        "Anchor",
        // interactive_container.rs
        "This demo showcases how to use ",
        " to create interactive container widgets that may contain other widgets.",
        // window_options.rs
        "title:",
        "Can the window be moved by dragging?",
        "Constrain window to the screen",
        "Offset:",
        "Offset x",
        "Offset y",
        "Drag to move:",
        "Where the user can grab the window to move it",
        "Off",
        "Anywhere",
        "The window cannot be dragged to move it (same as movable = false)",
        "Anywhere on touch screens, title-bar only otherwise (default)",
        "Only the title bar moves the window",
        "Drag anywhere on the window to move it",
        "This window is on top: {on_top}.",
        "Disable for 2 seconds",
        "Close",
        "You can collapse / close Windows via Ui::close",
        // extra_viewport.rs
        "This egui integration does not support multiple viewports",
        "egui and eframe supports having multiple native windows like this, which egui calls 'viewports'.",
        "Here you can see all the open viewports:",
        // panels.rs
        "Panels",
        "Expandable Upper Panel",
        "Left Panel",
        "Right Panel",
        "Bottom panel",
        "Bottom panel (collapsed)",
        "Central Panel",
        "Panel toggles:",
        // wrap_app.rs — remaining app chrome
        "NOTE: Some old browsers stuck on WebGL1 without sRGB support will not pass the color test.",
        "📱 iPhone SE 2nd Gen",
        "🖥️ Desktop 720p",
        "🖥️ Desktop 1080p",
        "Recent output events from egui. These are emitted when you interact with widgets, or move focus between them with TAB. They can be hooked up to a screen reader on supported platforms.",
        // accessibility_inspector.rs
        "🔎 AccessKit Inspector",
        "Node ID",
        "Value",
        "Children",
        "Actions",
        "Node not found",
        // apps/http_app.rs
        "HTTP requests made using ",
        "URL:",
        "HINT: paste the url of this page into the field above!",
        "Source code for this example",
        "Random image",
        "Error",
        "Response headers",
        "Click to copy the response body",
        "[binary]",
        // apps/image_viewer.rs
        "URI:",
        "file…",
        "min x",
        "min y",
        "max x",
        "max y",
        "Rotation",
        "origin x",
        "origin y",
        "Background color",
        "Tint",
        "The chosen fit will determine how the image tries to fill the available space",
        "Fit",
        "exact size",
        "fraction",
        "original size",
        "scale",
        "The calculated size will not exceed the maximum size",
        "Aspect ratio is maintained by scaling both sides as necessary",
        "Maintain aspect ratio",
        "Alt text",
        "Forget all images",
        // apps/fractal_clock.rs
        "Settings",
        "The fractal_clock clock is not showing the correct time",
        "Painted line count: {}",
        "Paused",
        "zoom",
        "Start line width",
        "depth",
        "length factor",
        "luminance factor",
        "width factor",
        "Inspired by a screensaver by Rob Mayoff",
        // apps/custom3d_glow.rs + custom3d_wgpu.rs
        "The triangle is being painted using ",
        " (Portable Rust graphics API awesomeness)",
        "It's not a very impressive demo, but it shows you can embed 3D inside of egui.",
        "Drag to rotate!",
        // main.rs — window title
        "egui demo app",
    ];

    /// Scoped keys whose translation legitimately equals the key (technology names),
    /// each with its reason inline; every other scoped key must have a real translation.
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

    /// The bundled Arabic catalogue is clean and complete: nothing skipped as malformed, and no
    /// stray rows — exactly the scoped key set, no more and no less.
    #[test]
    fn arabic_catalogue_has_zero_skipped_or_stray_rows() {
        let catalog = ar().catalog();
        assert_eq!(catalog.skipped, 0, "malformed lines in ar.tsv");
        assert_eq!(
            catalog.plain.len(),
            SCOPED_KEYS.len(),
            "stray or missing rows in ar.tsv"
        );
    }

    /// `#` comment lines (indented ones too) and blank lines are ignored, so a catalogue can
    /// carry its header and section comments.
    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let text = concat!(
            "# Arabic (ar) catalog\n",
            "\n",
            "  # an indented section comment\n",
            "\tDemos\tالعروض\n",
            "# another section\n",
            "\tQuit\tإنهاء\n",
        );
        let catalog = Catalog::parse(text);
        assert_eq!(catalog.skipped, 0);
        assert_eq!(catalog.plain.len(), 2);
        assert_eq!(catalog.plain("Demos"), Some("العروض"));
        assert_eq!(catalog.plain("Quit"), Some("إنهاء"));
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

        // A value is inserted verbatim, even when it contains a later placeholder.
        assert_eq!(fmt("{a} {b}", &[("a", "{b}"), ("b", "two")]), "{b} two");
        assert_eq!(
            fmt("{a} {b} {a}", &[("b", "{a}"), ("a", "one")]),
            "one {a} one"
        );
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
