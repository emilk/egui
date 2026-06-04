//! Experiment (Taffy-style deep-tree blowup):
//!
//! egui's atom layout re-measures a grown nested `Layout` atom so its content can reflow at the
//! resolved width (the cross-after-main pass in `AtomLayout::measure`, plus the re-measure in
//! `paint_at`). Unlike Taffy, there is currently *no measurement cache*. So a chain of nested
//! `grow` layouts where every level fills past its content should re-measure each child more than
//! once per level — `O(2^depth)` — reproducing the kind of exponential blowup Taffy's PR #246
//! cache fixed (a deep tree that went from ~17s to ~3ms).
//!
//! Run with:
//! ```sh
//! cargo nextest run -p egui_tests -E 'test(atom_reflow_blowup)' --run-ignored all --no-capture
//! ```

use egui::{Atom, AtomExt as _, AtomLayout, Vec2};
use egui_kittest::Harness;
use std::time::{Duration, Instant};

/// A chain of `depth` nested layouts. Each level is a single `grow` child inside a parent whose
/// `min_size` is *strictly larger* than the child's own (the child's natural width = its own
/// `min_size`). So every parent grows its child past its natural width — `grow_main > 0` — which
/// triggers a re-measure of the child, recursively, all the way down.
fn nested(depth: usize) -> AtomLayout<'static> {
    if depth == 0 {
        AtomLayout::new("leaf")
    } else {
        let child = Atom::layout(nested(depth - 1)).atom_grow(true);
        // Larger at the top, decreasing toward the leaves, so each level genuinely grows its child.
        AtomLayout::new(child).min_size(Vec2::new(50.0 * depth as f32, 0.0))
    }
}

#[test]
#[ignore = "perf experiment, run manually with --run-ignored"]
fn atom_reflow_blowup() {
    let mut prev: Option<Duration> = None;
    for depth in 1..=30 {
        let start = Instant::now();
        let mut harness = Harness::builder().build_ui(|ui| {
            nested(depth).show(ui);
        });
        harness.run_steps(1);
        let elapsed = start.elapsed();

        let ratio = prev.map_or(String::new(), |p| {
            format!("(x{:.2} vs prev)", elapsed.as_secs_f64() / p.as_secs_f64())
        });
        println!("depth {depth:2}: {elapsed:>12.3?}  {ratio}");
        prev = Some(elapsed);

        if elapsed > Duration::from_secs(3) {
            println!("... aborting: blowup confirmed");
            break;
        }
    }
}
