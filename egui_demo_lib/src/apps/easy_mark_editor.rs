use egui::*;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(PartialEq)]
pub struct EasyMarkEditor {
    code: String,
}

impl Default for EasyMarkEditor {
    fn default() -> Self {
        Self {
            code: DEFAULT_CODE.trim().to_owned(),
        }
    }
}

impl epi::App for EasyMarkEditor {
    fn name(&self) -> &str {
        "🖹 EasyMark editor"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
}

impl EasyMarkEditor {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
        ui.separator();
        ui.columns(2, |columns| {
            ScrollArea::auto_sized()
                .id_source("source")
                .show(&mut columns[0], |ui| {
                    // ui.text_edit_multiline(&mut self.code);
                    ui.add(TextEdit::multiline(&mut self.code).text_style(TextStyle::Monospace));
                });
            ScrollArea::auto_sized()
                .id_source("rendered")
                .show(&mut columns[1], |ui| {
                    egui::experimental::easy_mark(ui, &self.code);
                });
        });
    }
}

const DEFAULT_CODE: &str = r#"
# EasyMark
EasyMark is a markup language, designed for extreme simplicity.

```
WARNING: EasyMark is still an evolving specification,
and is also missing some features.
```

----------------

# At a glance
- inline text:
  - normal, `code`, *strong*, ~strikethrough~, _underline_, /italics/
  - `\` escapes the next character
  - [hyperlink](https://github.com/emilk/egui)
  - Embedded URL: <https://github.com/emilk/egui>
- `# ` header
- `---` separator (horizontal line)
- `> ` quote
- `- ` bullet list
- `1. ` numbered list
- \`\`\` code fence

# Design
> /"Why do what everyone else is doing, when everyone else is already doing it?"
>   \- Emil

Goals:
1. easy to parse
2. easy to learn
3. similar to markdown

[The reference parser](https://github.com/emilk/egui/blob/master/egui/src/experimental/easy_mark_parser.rs) is \~250 lines of code, using only the Rust standard library. The parser uses no look-ahead or recursion.

There is never more than one way to accomplish the same thing, and each special character is only used for one thing. For instance `*` is used for *strong* and `-` is used for bullet lists. There is no alternative way to specify the *strong* style or getting a bullet list.

Similarity to markdown is kept when possible, but with much less ambiguity and some improvements (like _underlining_).

# Details
All style changes are single characters, so it is `*strong*`, NOT `**strong**`. Style is reset by a matching character, or at the end of the line.

Style change characters and escapes (`\`) work everywhere except for in inline code, code blocks and in URLs.

You can mix styles. For instance: /italics _underline_/ and *strong `code`*.

You can use styles on URLs: ~my webpage is at <http://www.example.com>~.

Newlines are preserved. If you want to continue text on the same line, just do so. Alternatively, escape the newline by ending the line with a backslash (`\`). \
Escaping the newline effectively ignores it.

The style characters are chosen to be similar to what they are representing:
  `_` = _underline_
  `~` = ~strikethrough~ (`-` is too common in normal text)
  `/` = /italics/
  `*` = *strong*

# TODO
- Sub-headers (`## h2`, `### h3` etc)
- Images
  - we want to be able to optionally specify size (width and\/or height)
  - centering of images is very desirable
  - captioning (image with a text underneath it)
  - `![caption=My image][width=200][center](url)` ?
- Tables
"#;
