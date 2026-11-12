use crate::{InnerResponse, Ui};

/// Select between expanded and compact UI contents based on available width.
///
/// Only the selected closure is called, and it is called once. This makes it safe to use closures
/// that update application state or create widgets with interaction state.
///
/// Use [`Self::breakpoint`] when the change should happen at a deliberate width. Use [`Self::fit`]
/// when the expanded contents have a known minimum width and should be shown whenever they fit.
/// The fit widths are caller declarations; the expanded minimum drives selection and egui does not
/// measure either closure.
/// If even the compact contents do not fit, they are still shown and the surrounding UI handles
/// the constraint as usual.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::AdaptiveLayout::breakpoint(420.0).show(
///     ui,
///     |ui| {
///         ui.horizontal(|ui| {
///             ui.button("Save");
///             ui.button("Export");
///         });
///     },
///     |ui| {
///         ui.menu_button("Actions", |ui| {
///             ui.button("Save");
///             ui.button("Export");
///         });
///     },
/// );
/// # });
/// ```
#[must_use = "You should call .show()"]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveLayout {
    strategy: Strategy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Strategy {
    Breakpoint(f32),
    Fit {
        expanded_min_width: f32,
        compact_min_width: f32,
    },
}

impl AdaptiveLayout {
    /// Select the expanded contents when the available width is at least `width` points.
    ///
    /// At exactly the breakpoint, the expanded contents are selected.
    pub fn breakpoint(width: f32) -> Self {
        Self {
            strategy: Strategy::Breakpoint(width.max(0.0)),
        }
    }

    /// Select the expanded contents when `expanded_min_width` fits in the available width.
    ///
    /// `compact_min_width` documents the minimum width intended for the compact contents. If
    /// neither minimum fits, the compact contents are still shown. These are caller-declared
    /// widths; the closures are not run for measurement.
    pub fn fit(expanded_min_width: f32, compact_min_width: f32) -> Self {
        Self {
            strategy: Strategy::Fit {
                expanded_min_width: expanded_min_width.max(0.0),
                compact_min_width: compact_min_width.max(0.0),
            },
        }
    }

    fn expanded_at(self, available_width: f32) -> bool {
        match self.strategy {
            Strategy::Breakpoint(width) => available_width >= width,
            Strategy::Fit {
                expanded_min_width,
                compact_min_width: _,
            } => available_width >= expanded_min_width,
        }
    }

    /// Show the expanded contents when they fit the configured strategy; otherwise show compact.
    ///
    /// The available width is read from `ui` before the selected closure runs. Exactly one closure
    /// runs per call. State that must persist between the two compositions should be owned outside
    /// them and passed into both closures as needed.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        expanded: impl FnOnce(&mut Ui) -> R,
        compact: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        if self.expanded_at(ui.available_width()) {
            ui.scope(expanded)
        } else {
            ui.scope(compact)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AdaptiveLayout;
    use crate::{Context, RawInput, vec2};
    use core::cell::Cell;

    #[test]
    fn breakpoint_selects_expanded_at_and_above_threshold() {
        let layout = AdaptiveLayout::breakpoint(300.0);
        assert!(!layout.expanded_at(299.9));
        assert!(layout.expanded_at(300.0));
        assert!(layout.expanded_at(300.1));
    }

    #[test]
    fn fit_uses_declared_expanded_minimum_width() {
        let layout = AdaptiveLayout::fit(500.0, 120.0);
        assert!(!layout.expanded_at(499.9));
        assert!(layout.expanded_at(500.0));
        assert!(layout.expanded_at(600.0));
        assert!(!layout.expanded_at(100.0));
    }

    #[test]
    fn only_selected_branch_runs_once_in_nested_ui() {
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let expanded_calls = Cell::new(0);
        let compact_calls = Cell::new(0);

        let output = ctx.run_ui(RawInput::default(), |ui| {
            ui.allocate_ui(vec2(200.0, 100.0), |ui| {
                AdaptiveLayout::breakpoint(200.0).show(
                    ui,
                    |_| expanded_calls.set(expanded_calls.get() + 1),
                    |_| compact_calls.set(compact_calls.get() + 1),
                );
            });
        });
        output.drop_without_applying_deltas();

        assert_eq!(expanded_calls.get(), 1);
        assert_eq!(compact_calls.get(), 0);
    }

    #[test]
    fn branch_is_reselected_when_width_changes_between_frames() {
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let expanded_calls = Cell::new(0);
        let compact_calls = Cell::new(0);

        for width in [250.0, 150.0, 250.0] {
            let output = ctx.run_ui(RawInput::default(), |ui| {
                ui.allocate_ui(vec2(width, 100.0), |ui| {
                    AdaptiveLayout::breakpoint(200.0).show(
                        ui,
                        |_| expanded_calls.set(expanded_calls.get() + 1),
                        |_| compact_calls.set(compact_calls.get() + 1),
                    );
                });
            });
            output.drop_without_applying_deltas();
        }

        assert_eq!(expanded_calls.get(), 2);
        assert_eq!(compact_calls.get(), 1);
    }
}
