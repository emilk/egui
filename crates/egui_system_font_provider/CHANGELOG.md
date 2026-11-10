# Changelog for egui_system_font_provider
All notable changes to the `egui_system_font_provider` crate will be noted in this file.


This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## Unreleased

### Fixed
- The system font provider no longer picks fonts that map Arabic characters but cannot shape
  them (missing `init`/`medi`/`fina` GSUB features and no `morx`), which produced
  disconnected letters. Cluster script resolution now considers the whole grapheme cluster
  rather than only its first character. Monospace fallback now chains into sans-serif for
  scripts no monospace font covers.

## 0.37.0 - Initial release
