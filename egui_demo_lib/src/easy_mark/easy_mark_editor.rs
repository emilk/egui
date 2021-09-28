use egui::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct EasyMarkEditor {
    code: String,
    highlight_editor: bool,
    show_rendered: bool,

    #[cfg_attr(feature = "serde", serde(skip))]
    highlighter: crate::easy_mark::MemoizedEasymarkHighlighter,
}

impl PartialEq for EasyMarkEditor {
    fn eq(&self, other: &Self) -> bool {
        (&self.code, self.highlight_editor, self.show_rendered)
            == (&other.code, other.highlight_editor, other.show_rendered)
    }
}

impl Default for EasyMarkEditor {
    fn default() -> Self {
        Self {
            code: DEFAULT_CODE.trim().to_owned(),
            highlight_editor: true,
            show_rendered: true,
            highlighter: Default::default(),
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
        egui::Grid::new("controls").show(ui, |ui| {
            ui.checkbox(&mut self.highlight_editor, "Highlight editor");
            egui::reset_button(ui, self);
            ui.end_row();

            ui.checkbox(&mut self.show_rendered, "Show rendered");
            ui.add(crate::__egui_github_link_file!());
        });

        ui.separator();

        if self.show_rendered {
            ui.columns(2, |columns| {
                ScrollArea::vertical()
                    .id_source("source")
                    .show(&mut columns[0], |ui| self.editor_ui(ui));
                ScrollArea::vertical()
                    .id_source("rendered")
                    .show(&mut columns[1], |ui| {
                        // TODO: we can save some more CPU by caching the rendered output.
                        crate::easy_mark::easy_mark(ui, &self.code);
                    });
            });
        } else {
            ScrollArea::vertical()
                .id_source("source")
                .show(ui, |ui| self.editor_ui(ui))
        }
    }

    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            code, highlighter, ..
        } = self;

        if self.highlight_editor {
            let mut layouter = |ui: &egui::Ui, easymark: &str, wrap_width: f32| {
                let mut layout_job = highlighter.highlight(ui.visuals(), easymark);
                layout_job.wrap_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };

            ui.add(
                egui::TextEdit::multiline(code)
                    .desired_width(f32::INFINITY)
                    .text_style(egui::TextStyle::Monospace) // for cursor height
                    .layouter(&mut layouter),
            );
        } else {
            ui.add(egui::TextEdit::multiline(code).desired_width(f32::INFINITY));
        }
        // let cursor = TextEdit::cursor(response.id);
        // TODO: cmd-i, cmd-b, etc for italics, bold, ....
    }
}

// ----------------------------------------------------------------------------

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
  - normal, `code`, *strong*, ~strikethrough~, _underline_, /italics/, ^raised^, $small$
  - `\` escapes the next character
  - [hyperlink](https://github.com/emilk/egui)
  - Embedded URL: <https://github.com/emilk/egui>
- `# ` header
- `---` separator (horizontal line)
- `> ` quote
- `- ` bullet list
- `1. ` numbered list
- \`\`\` code fence
- a^2^ + b^2^ = c^2^
- $Remember to read the small print$

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
  `~` = ~strikethrough~ (`-` is used for bullet points)
  `/` = /italics/
  `*` = *strong*
  `$` = $small$
  `^` = ^raised^

# TODO
- Sub-headers (`## h2`, `### h3` etc)
- Images
  - we want to be able to optionally specify size (width and\/or height)
  - centering of images is very desirable
  - captioning (image with a text underneath it)
  - `![caption=My image][width=200][center](url)` ?
- Nicer URL:s
  - `<url>` and `[url](url)` do the same thing yet look completely different.
  - let's keep similarity with images
- Tables
- Inspiration: <https://mycorrhiza.lesarbr.es/page/mycomarkup>
"#;
