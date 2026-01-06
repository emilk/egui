# egui_kittest

[![Latest version](https://img.shields.io/crates/v/egui_kittest.svg)](https://crates.io/crates/egui_kittest)
[![Documentation](https://docs.rs/egui_kittest/badge.svg)](https://docs.rs/egui_kittest)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

Ui testing library for egui, based on [kittest](https://github.com/rerun-io/kittest) (an [AccessKit](https://github.com/AccessKit/accesskit) based testing library).

## Example usage
```rust
use egui::accesskit::Toggled;
use egui_kittest::{Harness, kittest::{Queryable, NodeT}};

let mut checked = false;
let app = |ui: &mut egui::Ui| {
    ui.checkbox(&mut checked, "Check me!");
};

let mut harness = Harness::new_ui(app);

let checkbox = harness.get_by_label("Check me!");
assert_eq!(checkbox.accesskit_node().toggled(), Some(Toggled::False));
checkbox.click();

harness.run();

let checkbox = harness.get_by_label("Check me!");
assert_eq!(checkbox.accesskit_node().toggled(), Some(Toggled::True));

// Shrink the window size to the smallest size possible
harness.fit_contents();

// You can even render the ui and do image snapshot tests
#[cfg(all(feature = "wgpu", feature = "snapshot"))]
harness.snapshot("readme_example");
```

## Configuration

You can configure test settings via a `kittest.toml` file in your workspace root.
All possible settings and their defaults:
```toml
# path to the snapshot directory
output_path = "tests/snapshots"

# default threshold for image comparison tests
threshold = 0.6

# default failed_pixel_count_threshold
failed_pixel_count_threshold = 0

[windows]
threshold = 0.6
failed_pixel_count_threshold = 0

[macos]
threshold = 0.6
failed_pixel_count_threshold = 0

[linux]
threshold = 0.6
failed_pixel_count_threshold = 0
```

## Snapshot testing
There is a snapshot testing feature. To create snapshot tests, enable the `snapshot` and `wgpu` features.
Once enabled, you can call `Harness::snapshot` to render the ui and save the image to the `tests/snapshots` directory.

To update the snapshots, run your tests with `UPDATE_SNAPSHOTS=true`, so e.g. `UPDATE_SNAPSHOTS=true cargo test`.
Running with `UPDATE_SNAPSHOTS=true` will cause the tests to succeed.
This is so that you can set `UPDATE_SNAPSHOTS=true` and update all tests, without `cargo test` failing on the first failing crate.

`UPDATE_SNAPSHOTS=true` will only update the images of _failing_ tests.
If you want to update all snapshot images, even those that are within error margins,
run with `UPDATE_SNAPSHOTS=force`.

If you want to have multiple snapshots in the same test, it makes sense to collect the results in a `SnapshotResults`
([look here](https://github.com/emilk/egui/blob/d1fcd740ded5d69016c993a502b52e67f5d492d7/crates/egui_demo_lib/src/demo/demo_app_windows.rs#L387-L420) for an example).
This way they can all be updated at the same time.

You should add the following to your `.gitignore`:
```gitignore
**/tests/snapshots/**/*.diff.png
**/tests/snapshots/**/*.new.png
```

### Guidelines for writing snapshot tests

* Whenever **possible** prefer regular Rust tests or `insta` snapshot tests over image comparison tests because…
  * …compared to regular Rust tests, they can be relatively slow to run
  * …they are brittle since unrelated side effects (like a change in color) can cause the test to fail
  * …images take up repo space
* images should…
  * …be checked in or otherwise be available (egui uses [git LFS](https://git-lfs.com/) files for this purpose)
  * …depict exactly what's tested and nothing else
  * …have a low resolution to avoid growth in repo size
  * …have a low comparison threshold to avoid the test passing despite unwanted differences (the default threshold should be fine for most usecases!)

### What do do when CI / another computer produces a different image?

The default tolerance settings should be fine for almost all gui comparison tests.
However, especially when you're using custom rendering, you may observe images difference with different setups leading to unexpected test failures.

First check whether the difference is due to a change in enabled rendering features, potentially due to difference in hardware (/software renderer) capabilitites.
Generally you should carefully enforcing the same set of features for all test runs, but this may happen nonetheless.

Once you validated that the differences are miniscule and hard to avoid, you can try to _carefully_ adjust the comparison tolerance setting (`SnapshotOptions::threshold`, TODO([#5683](https://github.com/emilk/egui/issues/5683)): as well as number of pixels allowed to differ) for the specific test.

⚠️ **WARNING** ⚠️
Picking too high tolerances may mean that you are missing actual test failures.
It is recommended to manually verify that the tests still break under the right circumstances as expected after adjusting the tolerances.

---

In order to avoid image differences, it can be useful to form an understanding of how they occur in the first place.

Discrepancies can be caused by a variety of implementation details that depend on the concrete GPU, OS, rendering backend (Metal/Vulkan/DX12 etc.) or graphics driver (even between different versions of the same driver).

Common issues include:
* multi-sample anti-aliasing
  * sample placement and sample resolve steps are implementation defined
  * alpha-to-coverage algorithm/pattern can wary wildly between implementations
* texture filtering
  * different implementations may apply different optimizations *even* for simple linear texture filtering
* out of bounds texture access (via `textureLoad`)
  * implementations are free to return indeterminate values instead of clamping
* floating point evaluation, for details see [WGSL spec § 15.7. Floating Point Evaluation](https://www.w3.org/TR/WGSL/#floating-point-evaluation). Notably:
  * rounding mode may be inconsistent
  * floating point math "optimizations" may occur
    * depending on output shading language, different arithmetic optimizations may be performed upon floating point operations even if they change the result
  * floating point denormal flush
    * even on modern implementations, denormal float values may be flushed to zero
  * `NaN`/`Inf` handling
    * whenever the result of a function should yield `NaN`/`Inf`, implementations may free to yield an indeterminate value instead
  * builtin-function function precision & error handling (trigonometric functions and others)
* [partial derivatives (dpdx/dpdx)](https://www.w3.org/TR/WGSL/#dpdx-builtin)
  * implementations are free to use either `dpdxFine` or `dpdxCoarse`
* [...]

From this follow a few simple recommendations (these may or may not apply as they may impose unwanted restrictions on your rendering setup):
* avoid enabling mult-sample anti-aliasing whenever it's not explicitly tested or needed
* do not rely on NaN, Inf and denormal float values
* consider dedicated test paths for texture sampling
* prefer explicit partial derivative functions
