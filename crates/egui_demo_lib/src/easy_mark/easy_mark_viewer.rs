use super::easy_mark_parser as easy_mark;
use egui::*;

/// Parse and display a VERY simple and small subset of Markdown.
pub fn easy_mark(ui: &mut Ui, easy_mark: &str) {
    easy_mark_it(ui, easy_mark::Parser::new(easy_mark));
}

pub fn easy_mark_it<'em>(ui: &mut Ui, items: impl Iterator<Item = easy_mark::Item<'em>>) {
    let initial_size = vec2(
        ui.available_width(),
        ui.spacing().interact_size.y, // Assume there will be
    );

    let layout = Layout::left_to_right(Align::BOTTOM).with_main_wrap(true);

    ui.allocate_ui_with_layout(initial_size, layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let row_height = ui.text_style_height(&TextStyle::Body);
        ui.set_row_height(row_height);

        for item in items {
            item_ui(ui, item);
        }
    });
}

pub fn item_ui(ui: &mut Ui, item: easy_mark::Item<'_>) {
    let row_height = ui.text_style_height(&TextStyle::Body);
    let one_indent = row_height / 2.0;

    match item {
        easy_mark::Item::Newline => {
            // ui.label("\n"); // too much spacing (paragraph spacing)
            ui.allocate_exact_size(vec2(0.0, row_height), Sense::hover()); // make sure we take up some height
            ui.end_row();
            ui.set_row_height(row_height);
        }

        easy_mark::Item::Text(style, text) => {
            ui.label(rich_text_from_style(text, &style));
        }
        easy_mark::Item::Hyperlink(style, text, url) => {
            let label = rich_text_from_style(text, &style);
            ui.add(Hyperlink::from_label_and_url(label, url));
        }

        easy_mark::Item::Separator => {
            ui.add(Separator::default().horizontal());
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
            rect.max.x = ui.max_rect().max.x;
            let code_bg_color = ui.visuals().code_bg_color;
            ui.painter().set(
                where_to_put_background,
                Shape::rect_filled(rect, 1.0, code_bg_color),
            );
        }
    };
}

fn rich_text_from_style(text: &str, style: &easy_mark::Style) -> RichText {
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

    let mut rich_text = RichText::new(text);
    if heading && !small {
        rich_text = rich_text.heading().strong();
    }
    if small && !heading {
        rich_text = rich_text.small();
    }
    if code {
        rich_text = rich_text.code();
    }
    if strong {
        rich_text = rich_text.strong();
    } else if quoted {
        rich_text = rich_text.weak();
    }
    if underline {
        rich_text = rich_text.underline();
    }
    if strikethrough {
        rich_text = rich_text.strikethrough();
    }
    if italics {
        rich_text = rich_text.italics();
    }
    if raised {
        rich_text = rich_text.raised();
    }
    rich_text
}

fn bullet_point(ui: &mut Ui, width: f32) -> Response {
    let row_height = ui.text_style_height(&TextStyle::Body);
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        rect.height() / 8.0,
        ui.visuals().strong_text_color(),
    );
    response
}

fn numbered_point(ui: &mut Ui, width: f32, number: &str) -> Response {
    let font_id = TextStyle::Body.resolve(ui.style());
    let row_height = ui.fonts(|f| f.row_height(&font_id));
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    let text = format!("{}.", number);
    let text_color = ui.visuals().strong_text_color();
    ui.painter().text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        text,
        font_id,
        text_color,
    );
    response
}
