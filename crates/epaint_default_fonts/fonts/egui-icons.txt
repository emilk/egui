egui-icons.ttf

A subset of emoji-icon-font.ttf (https://github.com/jslegers/emoji-icon-font),
containing only the five glyphs that egui exposes as `egui::special_emojis`:

    U+E618  the Android logo
    U+E61F  the Windows logo
    U+E624  the GitHub logo
    U+E625  the word `git`
    U+F8FF  the Apple logo

No platform font has these, so egui bundles them even when the rest of
emoji-icon-font.ttf is left out (the `monochrome_emoji_fonts` feature).

Regenerate with:

    pip install fonttools
    python3 -m fontTools.subset emoji-icon-font.ttf \
        --unicodes=E618,E61F,E624,E625,F8FF \
        --no-hinting --desubroutinize \
        --output-file=egui-icons.ttf

The license is that of emoji-icon-font.ttf: see emoji-icon-font-mit-license.txt.
