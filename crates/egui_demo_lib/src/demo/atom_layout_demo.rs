use egui::{
    Align2, Atom, AtomExt as _, AtomLayout, Atoms, Color32, CornerRadius, Direction, Frame, Margin,
    RichText, Stroke, TextWrapMode, Vec2,
};

// A small palette used to colour-code the cards.
const BLUE: Color32 = Color32::from_rgb(0x61, 0xAF, 0xEF);
const RED: Color32 = Color32::from_rgb(0xE0, 0x6C, 0x75);
const GREEN: Color32 = Color32::from_rgb(0x98, 0xC3, 0x79);
const AMBER: Color32 = Color32::from_rgb(0xD1, 0x9A, 0x66);
const PURPLE: Color32 = Color32::from_rgb(0xC6, 0x78, 0xDD);
const CYAN: Color32 = Color32::from_rgb(0x56, 0xB6, 0xC2);

struct Card {
    accent: Color32,
    title: &'static str,
    description: &'static str,
    tags: &'static [&'static str],
}

const CARDS: &[Card] = &[
    Card {
        accent: BLUE,
        title: "Northern Lights",
        description: "Chasing the shimmering green aurora across a frozen lake under a perfectly \
                      clear arctic sky.",
        tags: &["aurora", "night", "long-exposure", "iceland", "winter"],
    },
    Card {
        accent: GREEN,
        title: "Rainforest Canopy",
        description: "A slow walk through the misty treetops at dawn, alive with insects and \
                      distant birdsong.",
        tags: &["jungle", "macro", "wildlife", "humid"],
    },
    Card {
        accent: AMBER,
        title: "Desert Dunes",
        description: "Endless ridges of fine sand shifting and glowing in the warm late afternoon \
                      light.",
        tags: &["sahara", "golden-hour", "minimal", "heat", "travel", "sand"],
    },
    Card {
        accent: PURPLE,
        title: "City After Rain",
        description: "Saturated neon reflections rippling on the wet pavement in the heart of a \
                      busy downtown.",
        tags: &["urban", "neon", "reflections"],
    },
    Card {
        accent: CYAN,
        title: "Coral Gardens",
        description: "Drifting weightlessly over a vivid reef that is absolutely bursting with \
                      colour and motion.",
        tags: &["ocean", "diving", "macro", "blue", "fish", "warm", "reef"],
    },
    Card {
        accent: RED,
        title: "Autumn Trail",
        description: "A quiet woodland path carpeted in red and gold maple leaves on a crisp \
                      October morning.",
        tags: &["forest", "fall", "hike", "leaves"],
    },
];

/// A responsive card gallery built entirely out of [`AtomLayout`]s.
///
/// Think of it as flexbox: the outer list is a wrapping row of `grow` cards, and inside each card
/// a column holds a mock image, a title, a description and a wrapping row of `grow` tags. Resize
/// the window to watch the cards and tags reflow.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct AtomLayoutDemo {}

impl crate::Demo for AtomLayoutDemo {
    fn name(&self) -> &'static str {
        "🖼 Atom Layout"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new(self.name())
            .default_width(640.0)
            .default_height(560.0)
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for AtomLayoutDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.small(
            "A responsive card gallery — one wrapping AtomLayout of grow cards, each itself a \
             column with a wrapping row of grow tags. Resize the window to watch it reflow.",
        );
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("cards_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut cards = Atoms::default();
                for c in CARDS {
                    cards.push_right(card(ui, c));
                }
                AtomLayout::new(cards)
                    .wrap(true)
                    .gap(12.0)
                    .align2(Align2::LEFT_TOP)
                    // Fill the available width so the `grow` cards stretch to share each row.
                    .min_size(Vec2::new(ui.available_width(), 0.0))
                    .show(ui);
            });
    }
}

/// One card: a vertical column of [ mock image · title · description · tags · footer ]. Marked
/// `grow` so cards share each row's width; the contents re-wrap to the grown width automatically.
fn card(ui: &egui::Ui, card: &Card) -> Atom<'static> {
    // Mock image: an empty layout with a coloured fill. It's a nested layout, so it stretches to
    // the full card width.
    let image = Atom::layout(
        AtomLayout::new(())
            .frame(
                Frame::new()
                    .fill(card.accent.gamma_multiply(0.8))
                    .corner_radius(CornerRadius::same(6)),
            )
            .min_size(Vec2::new(0.0, 96.0)),
    );

    // Tags: a wrapping row where each tag grows to justify the line.
    let mut tag_atoms = Atoms::default();
    for t in card.tags {
        tag_atoms.push_right(tag(ui, card.accent, t).atom_grow(true));
    }
    let tags = Atom::layout(
        AtomLayout::new(tag_atoms)
            .wrap(true)
            .gap(4.0)
            .align2(Align2::LEFT_TOP),
    );

    // Footer: Like / Share buttons that split the card width.
    let footer = Atom::layout(
        AtomLayout::new((
            footer_button(ui, "♥ Like").atom_grow(true),
            footer_button(ui, "↗ Share").atom_grow(true),
        ))
        .gap(6.0),
    );

    let card_frame = Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(8));

    let column = AtomLayout::new((
        image,
        RichText::new(card.title)
            .strong()
            .atom_align(Align2::LEFT_CENTER),
        RichText::new(card.description)
            .small()
            .weak()
            .atom_align(Align2::LEFT_TOP)
            // `shrink` lets the description wrap to the card width instead of its full text width
            // dictating how wide the card has to be.
            .atom_shrink(true),
        tags,
        // A `grow` spacer eats any leftover vertical space, pinning the footer to the bottom — so
        // footers line up across cards of different heights (cards in a row are equal height).
        Atom::grow(),
        footer,
    ))
    .direction(Direction::TopDown)
    .frame(card_frame)
    .gap(6.0)
    .wrap_mode(TextWrapMode::Wrap)
    // Stretch full-width pieces (image, footer) to the card's grown width.
    .cross_justify(true)
    .align2(Align2::LEFT_TOP);

    // `atom_max_width` sets the card's natural (flex-basis) width; `grow` lets it stretch to share
    // the row, and the core re-measures the contents at that grown width so they reflow.
    Atom::layout(column).atom_grow(true).atom_max_width(230.0)
}

/// A flat footer button (e.g. Like / Share).
fn footer_button(ui: &egui::Ui, text: &str) -> Atom<'static> {
    let visuals = &ui.visuals().widgets.inactive;
    let frame = Frame::new()
        .inner_margin(Margin::symmetric(8, 4))
        .corner_radius(CornerRadius::same(6))
        .fill(visuals.bg_fill)
        .stroke(visuals.bg_stroke);
    Atom::layout(
        AtomLayout::new(RichText::new(text.to_owned()).color(ui.visuals().weak_text_color()))
            .frame(frame)
            .align2(Align2::CENTER_CENTER),
    )
}

/// A small filled tag chip.
fn tag(ui: &egui::Ui, accent: Color32, text: &str) -> Atom<'static> {
    let bg = ui.visuals().window_fill();
    let frame = Frame::new()
        .inner_margin(Margin::symmetric(6, 1))
        .corner_radius(CornerRadius::same(8))
        .fill(bg.lerp_to_gamma(accent, 0.22))
        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)));
    Atom::layout(
        AtomLayout::new(RichText::new(text.to_owned()).small().color(accent))
            .frame(frame)
            .align2(Align2::CENTER_CENTER)
            .gap(0.0),
    )
}
