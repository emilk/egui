# `epaint_default_fonts` -  fonts for epaint and egui

[![Latest version](https://img.shields.io/crates/v/epaint_default_fonts.svg)](https://crates.io/crates/epaint_default_fonts)
[![Documentation](https://docs.rs/epaint_default_fonts/badge.svg)](https://docs.rs/epaint_default_fonts)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

Default fonts that are used in `epaint` and `egui`. Not intended for use as a standalone library.

Made for [`egui`](https://github.com/emilk/egui/).

## Font hinting

`epaint` relies on embedded TrueType hinting instructions for crisp text on low-dpi
screens: skrifa only auto-hints a font that lacks an interpreter program, so a font with
no glyph instructions renders unhinted and blurry.

`Radio Canada` shipped without glyph instructions, so it was hinted with
[`ttfautohint`](https://www.freetype.org/ttfautohint/), which preserves its variable axes:

```sh
ttfautohint --windows-compatibility \
    "RadioCanada-VariableFont_wdth,wght.ttf" "RadioCanada-VariableFont_wdth,wght.ttf"
```
