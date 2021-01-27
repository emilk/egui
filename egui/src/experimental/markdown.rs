use super::markdown_parser::*;
use crate::*;

/// Parse and display a VERY simple and small subset of Markdown.
pub fn markdown(ui: &mut Ui, markdown: &str) {
    ui.horizontal_wrapped(|ui| {
        let row_height = ui.fonts()[TextStyle::Body].row_height();
        let style = ui.style_mut();
        style.spacing.interact_size.y = row_height;
        style.spacing.item_spacing = vec2(0.0, 2.0);

        for item in MarkdownParser::new(markdown) {
            match item {
                MarkdownItem::Newline => {
                    // ui.label("\n"); // too much spacing (paragraph spacing)
                    ui.allocate_exact_size(vec2(0.0, row_height), Sense::hover()); // make sure we take up some height
                    ui.end_row();
                }
                MarkdownItem::Separator => {
                    ui.add(Separator::new().horizontal());
                }
                MarkdownItem::BulletPoint(indent) => {
                    let indent = indent as f32 * row_height / 3.0;
                    ui.allocate_exact_size(vec2(indent, row_height), Sense::hover());
                    bullet_point(ui);
                }
                MarkdownItem::Body(range) => {
                    ui.label(range);
                }
                MarkdownItem::Heading(range) => {
                    ui.heading(range);
                }
                MarkdownItem::Emphasis(range) => {
                    ui.colored_label(Color32::WHITE, range);
                }
                MarkdownItem::InlineCode(range) => {
                    ui.code(range);
                }
                MarkdownItem::Hyperlink(text, url) => {
                    ui.add(Hyperlink::new(url).text(text));
                }
            };
        }
    });
}

fn bullet_point(ui: &mut Ui) -> Response {
    let row_height = ui.fonts()[TextStyle::Body].row_height();

    let (rect, response) = ui.allocate_exact_size(vec2(row_height, row_height), Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        rect.height() / 5.0,
        ui.style().visuals.text_color(),
    );
    response
}
