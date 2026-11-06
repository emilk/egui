# epaint - egui paint library

[![Latest version](https://img.shields.io/crates/v/epaint.svg)](https://crates.io/crates/epaint)
[![Documentation](https://docs.rs/epaint/badge.svg)](https://docs.rs/epaint)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

A bare-bones 2D graphics library for turning simple 2D shapes and text into textured triangles.

Made for [`egui`](https://github.com/emilk/egui/).

## Line breaking for scripts without spaces

Thai, Lao, Khmer and Myanmar are written without spaces between words; Unicode line breaking
(UAX #14, class SA) finds their word boundaries with a dictionary. epaint ships a
maximum-matching breaker over the Unicode/ICU line-break dictionaries behind the
`sea_dictionary` cargo feature (the data files in `src/text/dictionaries/` carry their
copyright/license headers). Without the feature — or for any script, with your own
segmenter — install a `text::LineBreaker` implementation via `TextWrapping::line_breaker`.
