use super::easy_mark_parser as easy_mark;
use crate::*;

/// Parse and display a VERY simple and small subset of Markdown.
pub fn easy_mark(ui: &mut Ui, easy_mark: &str) {
    easy_mark_it(ui, easy_mark::Parser::new(easy_mark))
}

pub fn easy_mark_it<'em>(ui: &mut Ui, items: impl Iterator<Item = easy_mark::Item<'em>>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
        ui.set_row_height(ui.fonts()[TextStyle::Body].row_height());

        for item in items {
            item_ui(ui, item);
        }
    });
}

pub fn item_ui(ui: &mut Ui, item: easy_mark::Item<'_>) {
    let row_height = ui.fonts()[TextStyle::Body].row_height();
    let one_indent = row_height / 2.0;

    match item {
        easy_mark::Item::Newline => {
            // ui.label("\n"); // too much spacing (paragraph spacing)
            ui.allocate_exact_size(vec2(0.0, row_height), Sense::hover()); // make sure we take up some height
            ui.end_row();
            ui.set_row_height(row_height);
        }

        easy_mark::Item::Text(style, text) => {
            ui.add(label_from_style(text, &style));
        }
        easy_mark::Item::Hyperlink(style, text, url) => {
            let label = label_from_style(text, &style);
            ui.add(Hyperlink::from_label_and_url(label, url));
        }

        easy_mark::Item::Separator => {
            ui.add(Separator::new().horizontal());
        }
        easy_mark::Item::Indentation(indent) => {
            let indent = indent as f32 * one_indent;
            ui.allocate_exact_size(vec2(indent, row_height), Sense::hover());
        }
        easy_mark::Item::QuoteIndent => {
            let rect = ui
                .allocate_exact_size(vec2(2.0 * one_indent, row_height), Sense::hover())
                .0;
            let rect = rect.expand2(ui.style().spacing.item_spacing * 0.5);
            ui.painter().line_segment(
                [rect.center_top(), rect.center_bottom()],
                (1.0, ui.visuals().weak_text_color()),
            );
        }
        easy_mark::Item::BulletPoint => {
            ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
            bullet_point(ui, one_indent);
            ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
        }
        easy_mark::Item::NumberedPoint(number) => {
            let width = 3.0 * one_indent;
            numbered_point(ui, width, number);
            ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
        }
        easy_mark::Item::CodeBlock(_language, code) => {
            let where_to_put_background = ui.painter().add(Shape::Noop);
            let mut rect = ui.monospace(code).rect;
            rect = rect.expand(1.0); // looks better
            rect.max.x = ui.max_rect_finite().max.x;
            let code_bg_color = ui.visuals().code_bg_color;
            ui.painter().set(
                where_to_put_background,
                Shape::rect_filled(rect, 1.0, code_bg_color),
            );
        }
    };
}

fn label_from_style(text: &str, style: &easy_mark::Style) -> Label {
    let easy_mark::Style {
        heading,
        quoted,
        code,
        strong,
        underline,
        strikethrough,
        italics,
        small,
        raised,
    } = *style;

    let small = small || raised; // Raised text is also smaller

    let mut label = Label::new(text);
    if heading && !small {
        label = label.heading().strong();
    }
    if small && !heading {
        label = label.small();
    }
    if code {
        label = label.code();
    }
    if strong {
        label = label.strong();
    } else if quoted {
        label = label.weak();
    }
    if underline {
        label = label.underline();
    }
    if strikethrough {
        label = label.strikethrough();
    }
    if italics {
        label = label.italics();
    }
    if raised {
        label = label.raised();
    }
    label
}

fn bullet_point(ui: &mut Ui, width: f32) -> Response {
    let row_height = ui.fonts()[TextStyle::Body].row_height();
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        rect.height() / 8.0,
        ui.visuals().strong_text_color(),
    );
    response
}

fn numbered_point(ui: &mut Ui, width: f32, number: &str) -> Response {
    let row_height = ui.fonts()[TextStyle::Body].row_height();
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    let text = format!("{}.", number);
    let text_color = ui.visuals().strong_text_color();
    ui.painter().text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        text,
        TextStyle::Body,
        text_color,
    );
    response
}
