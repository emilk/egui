use std::collections::BTreeSet;

use egui::{
    Align2, AtomExt as _, AtomLayout, AtomUi, Button, Color32, CornerRadius, Direction, Frame,
    Margin, RichText, Stroke, TextWrapMode, Vec2, atom,
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

/// Colours pulled from the [`egui::Visuals`] up front, so the card builders (which only see an
/// [`AtomUi`]) don't need to reach back into the [`egui::Ui`].
#[derive(Clone, Copy)]
struct CardTheme {
    card_fill: Color32,
    card_stroke: Stroke,
    chip_fill_base: Color32,
}

/// A responsive card gallery built with the [`AtomUi`] widget API.
///
/// Think of it as nested flexbox, all assembled by adding widgets to an [`AtomUi`] (never
/// hand-building [`egui::Atoms`]):
/// - a top-level row of [ tag-filter sidebar · card gallery ],
/// - the gallery is a wrapping row of `grow` cards,
/// - each card is a column with a wrapping row of `grow` tags and a footer of real [`Button`]s.
///
/// Resize the window to watch every level reflow, and click sidebar tags to filter the cards.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct AtomLayoutDemo {
    /// Titles of the cards the user has "liked" — proves the footer buttons are real, clickable
    /// widgets with their own [`egui::Response`].
    liked: BTreeSet<String>,

    /// Tags selected in the sidebar. A card is shown if this is empty or the card has one of them.
    selected_tags: BTreeSet<String>,
}

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
            "Nested flexbox built entirely with the AtomUi widget API: a tag-filter sidebar next \
             to a wrapping gallery of grow cards, each a column with a wrapping row of grow tags \
             and a footer of real Button widgets. Resize the window to watch every level reflow.",
        );
        ui.add_space(8.0);

        let theme = CardTheme {
            card_fill: ui.visuals().faint_bg_color,
            card_stroke: ui.visuals().widgets.noninteractive.bg_stroke,
            chip_fill_base: ui.visuals().window_fill(),
        };

        egui::ScrollArea::vertical()
            .id_salt("cards_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let full_width = ui.available_width();
                let gap = 12.0;
                // Split the row between sidebar and gallery: the sidebar is locked to this fraction
                // of the width (minus the gap) and the `grow` gallery fills the rest.
                let sidebar_width = (full_width - gap) / 4.0;
                let Self {
                    liked,
                    selected_tags,
                } = self;
                // Top-level flex row: [ sidebar · gallery ]. `min_size` makes the row fill the
                // available width so the `grow` gallery expands beside the fixed-width sidebar.
                ui.atom_builder(
                    AtomLayout::new(())
                        .gap(gap)
                        .align2(Align2::LEFT_TOP)
                        .min_size(Vec2::new(full_width, 0.0)),
                    |root| {
                        sidebar(root, theme, selected_tags, sidebar_width);
                        gallery(root, theme, liked, selected_tags);
                    },
                );
            });
    }
}

/// The tag-filter sidebar: a header above a wrapping, justified row of tag [`Button`]s — laid out
/// exactly like the in-card tags. Clicking a tag toggles it in `selected_tags`, filtering the
/// gallery.
fn sidebar<'a>(
    root: &mut AtomUi<'_, 'a>,
    theme: CardTheme,
    selected_tags: &mut BTreeSet<String>,
    width: f32,
) {
    let frame = Frame::new()
        .fill(theme.card_fill)
        .stroke(theme.card_stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(8));

    root.scope_builder(
        AtomLayout::new(())
            .direction(Direction::TopDown)
            .frame(frame)
            .gap(6.0)
            // Stretch the header and tag row to the sidebar's width.
            .cross_justify(true)
            .align2(Align2::LEFT_TOP)
            // Lock the sidebar to `width`; the `grow` gallery fills the rest of the row.
            .min_size(Vec2::new(width, 0.0)),
        // `min_size` (above) and `atom_max_width` pin the sidebar to `width` exactly: the min stops
        // it shrinking and the max stops the wrapping tag row from expanding past it.
        atom().atom_max_width(width),
        |aui| {
            aui.add(
                atom ().atom_align(Align2::LEFT_CENTER),
                AtomLayout::new(RichText::new("Filter by tag").strong()),
            );

            // A wrapping row where each tag button grows to justify the line — just like the tags
            // inside each card.
            aui.scope_builder(
                AtomLayout::new(())
                    .wrap(true)
                    .gap(4.0)
                    .align2(Align2::LEFT_TOP),
                atom(),
                |aui| {
                    for tag in all_tags() {
                        let selected = selected_tags.contains(tag);
                        // `grow` spacers either side center the label in the grown button.
                        let button =
                            Button::new((atom().atom_grow(true), tag, atom().atom_grow(true)))
                                .selected(selected);
                        if aui.add(atom().atom_grow(true), button).clicked() {
                            if selected {
                                selected_tags.remove(tag);
                            } else {
                                selected_tags.insert(tag.to_owned());
                            }
                        }
                    }
                },
            );

            // Reset filter. Disabling it when nothing is selected would need enabled-state plumbing
            // through AtomUi, so keep it simple and always clear.
            if !selected_tags.is_empty()
                && aui
                    .add(
                        atom(),
                        Button::new((atom().atom_grow(true), "Clear", atom().atom_grow(true))),
                    )
                    .clicked()
            {
                selected_tags.clear();
            }
        },
    );
}

/// The card gallery: a `grow`, wrapping row of cards filtered by the selected tags.
fn gallery<'a>(
    root: &mut AtomUi<'_, 'a>,
    theme: CardTheme,
    liked: &mut BTreeSet<String>,
    selected_tags: &BTreeSet<String>,
) {
    root.scope_builder(
        AtomLayout::new(())
            .wrap(true)
            .gap(12.0)
            .align2(Align2::LEFT_TOP),
        // `grow` lets the gallery fill the width left over by the sidebar; the core re-measures it
        // at that grown width, so the cards inside wrap to fit.
        atom().atom_grow(true),
        |cards| {
            for c in CARDS {
                if card_matches(c, selected_tags) {
                    card(cards, c, theme, liked);
                }
            }
        },
    );
}

/// A card is shown when no tag is selected, or it carries at least one of the selected tags.
fn card_matches(c: &Card, selected_tags: &BTreeSet<String>) -> bool {
    selected_tags.is_empty() || c.tags.iter().any(|t| selected_tags.contains(*t))
}

/// All tags across every card, de-duplicated and sorted (a `BTreeSet` iterates in order).
fn all_tags() -> Vec<&'static str> {
    CARDS
        .iter()
        .flat_map(|c| c.tags.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// One card: a vertical column of [ mock image · title · description · tags · footer ]. The
/// wrapper atom is marked `grow` so cards share each row's width; `atom_max_width` sets its natural
/// (flex-basis) width, and the core re-measures the contents at the grown width so they reflow.
fn card<'a>(
    cards: &mut AtomUi<'_, 'a>,
    c: &'static Card,
    theme: CardTheme,
    liked: &mut BTreeSet<String>,
) {
    let card_frame = Frame::new()
        .fill(theme.card_fill)
        .stroke(theme.card_stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(8));

    cards.scope_builder(
        AtomLayout::new(())
            .direction(Direction::TopDown)
            .frame(card_frame)
            .gap(6.0)
            .wrap_mode(TextWrapMode::Wrap)
            // Stretch full-width pieces (image, footer) to the card's grown width.
            .cross_justify(true)
            .align2(Align2::LEFT_TOP),
        atom().atom_grow(true).atom_max_width(230.0),
        |col| {
            // Mock image: an empty nested layout with a coloured fill. `cross_justify` stretches it
            // to the full card width.
            col.add(
                atom(),
                AtomLayout::new(())
                    .frame(
                        Frame::new()
                            .fill(c.accent.gamma_multiply(0.8))
                            .corner_radius(CornerRadius::same(6)),
                    )
                    .min_size(Vec2::new(0.0, 96.0)),
            );

            // Title.
            col.add(
                atom().atom_align(Align2::LEFT_CENTER),
                AtomLayout::new(RichText::new(c.title).strong()),
            );

            // Description. `shrink` lets it wrap to the card width instead of its full text width
            // dictating how wide the card has to be; the nested layout needs its own `wrap_mode`
            // because nested layouts don't inherit the column's.
            col.add(
                atom().atom_shrink(true).atom_align(Align2::LEFT_TOP),
                AtomLayout::new(RichText::new(c.description).small().weak())
                    .wrap_mode(TextWrapMode::Wrap),
            );

            // Tags: a wrapping row where each tag grows to justify the line.
            col.scope_builder(
                AtomLayout::new(())
                    .wrap(true)
                    .gap(4.0)
                    .align2(Align2::LEFT_TOP),
                atom(),
                |tags| {
                    for t in c.tags {
                        tags.add(atom().atom_grow(true), tag_chip(theme, c.accent, t));
                    }
                },
            );

            // A `grow` spacer eats any leftover vertical space, pinning the footer to the bottom —
            // so footers line up across cards of different heights (cards in a row are equal
            // height).
            col.add(atom().atom_grow(true), atom());

            // Footer: real Like / Share buttons that split the card width.
            col.scope_builder(AtomLayout::new(()).gap(6.0), atom(), |footer| {
                let is_liked = liked.contains(c.title);
                // `grow` spacers on either side of the label center it in the grown button.
                let like_text = if is_liked { "♥ Liked" } else { "♡ Like" };
                if footer
                    .add(
                        atom().atom_grow(true),
                        Button::new((atom().atom_grow(true), like_text, atom().atom_grow(true))),
                    )
                    .clicked()
                {
                    if is_liked {
                        liked.remove(c.title);
                    } else {
                        liked.insert(c.title.to_owned());
                    }
                }
                footer.add(
                    atom().atom_grow(true),
                    Button::new((atom().atom_grow(true), "↗ Share", atom().atom_grow(true))),
                );
            });
        },
    );
}

/// A small filled tag chip (decorative, accent-coloured).
fn tag_chip<'a>(theme: CardTheme, accent: Color32, text: &str) -> AtomLayout<'a> {
    let frame = Frame::new()
        .inner_margin(Margin::symmetric(6, 1))
        .corner_radius(CornerRadius::same(8))
        .fill(theme.chip_fill_base.lerp_to_gamma(accent, 0.22))
        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)));
    AtomLayout::new(RichText::new(text.to_owned()).small().color(accent))
        .frame(frame)
        .align2(Align2::CENTER_CENTER)
        .gap(0.0)
}
