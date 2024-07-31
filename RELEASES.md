# Releases
## Cadence
We don't have a regular cadence, but there is usually a new major release every two months or so.

Often a major release is followed by one or two patch releases within a week or two.

## Versioning
All crates under the [`crates/`](crates/) folder are published in lock-step, with the same version number. This means that we won't publish a new breaking change of a single crate without also publishing all other crates. This also means we sometimes do a new release of a crate even though there are no changes to that crate.

The only exception to this are patch releases, where we sometimes only patch a single crate.

The egui version in egui `master` is always the version of the last published crates. This is so that users can easily patch their egui crates to egui `master` if they want to.

## Governance
Releases are generally done by [emilk](https://github.com/emilk/), but the [rerun-io](https://github.com/rerun-io/) organization (where emilk is CTO) also has publish rights to all the crates.


## Rust version policy
Our Minimum Supported Rust Version (MSRV) is always _at least_ two minor release behind the latest Rust version. This means users of egui aren't forced to update to the very latest Rust version.

We don't update the MSRV in a patch release, unless we really, really need to.


# Release process
## Patch release
* [ ] Make a branch off of the latest release
* [ ] cherry-pick what you want to release
* [ ] run `cargo semver-checks`

## Optional polish before a major release
* [ ] improve the demo a bit
* [ ] see if you can make web demo WASM smaller
* [ ] `./scripts/docs.sh`: read and improve documentation of new stuff
* [ ] `cargo update`
* [ ] `cargo outdated` (or manually look for outdated crates in each `Cargo.toml`)
* [ ] `cargo machete`

## Release testing
* [ ] `cargo r -p egui_demo_app` and click around for while
* [ ] `./scripts/build_demo_web.sh --release -g`
  - check frame-rate and wasm size
  - test on mobile
  - test on chromium
  - check the in-browser profiler
* [ ] check the color test
* [ ] update `eframe_template` and test
* [ ] update `egui_plot` and test
* [ ] update `egui_tiles` and test
* [ ] test with Rerun
* [ ] `./scripts/check.sh`
* [ ] check that CI is green

## Preparation
* [ ] run `scripts/generate_example_screenshots.sh` if needed
* [ ] write a short release note that fits in a tweet
* [ ] record gif for `CHANGELOG.md` release note (and later twitter post)
* [ ] update changelogs using `scripts/generate_changelog.py --write`
  - For major releases, always diff to the latest MAJOR release, e.g. `--commit-range 0.27.0..HEAD`
* [ ] bump version numbers in workspace `Cargo.toml`

## Actual release
I usually do this all on the `master` branch, but doing it in a release branch is also fine, as long as you remember to merge it into `master` later.

* [ ] `git commit -m 'Release 0.x.0 - summary'`
* [ ] `cargo publish` (see below)
* [ ] `git tag -a 0.x.0 -m 'Release 0.x.0 - summary'`
* [ ] `git pull --tags ; git tag -d latest && git tag -a latest -m 'Latest release' && git push --tags origin latest --force ; git push --tags`
* [ ] merge release PR or push to `master`
* [ ] check that CI is green
* [ ] do a GitHub release: https://github.com/emilk/egui/releases/new
  * Follow the format of the last release
* [ ] wait for documentation to build: https://docs.rs/releases/queue

###  `cargo publish`:
```
(cd crates/emath                && cargo publish --quiet)  &&  echo "✅ emath"
(cd crates/ecolor               && cargo publish --quiet)  &&  echo "✅ ecolor"
(cd crates/epaint               && cargo publish --quiet)  &&  echo "✅ epaint"
(cd crates/epaint_default_fonts && cargo publish --quiet)  &&  echo "✅ epaint_default_fonts"
(cd crates/egui                 && cargo publish --quiet)  &&  echo "✅ egui"
(cd crates/egui-winit           && cargo publish --quiet)  &&  echo "✅ egui-winit"
(cd crates/egui_extras          && cargo publish --quiet)  &&  echo "✅ egui_extras"
(cd crates/egui-wgpu            && cargo publish --quiet)  &&  echo "✅ egui-wgpu"
(cd crates/egui_demo_lib        && cargo publish --quiet)  &&  echo "✅ egui_demo_lib"
(cd crates/egui_glow            && cargo publish --quiet)  &&  echo "✅ egui_glow"
(cd crates/eframe               && cargo publish --quiet)  &&  echo "✅ eframe"
```

## Announcements
* [ ] [twitter](https://x.com/ernerfeldt/status/1772665412225823105)
* [ ] egui discord
* [ ] [r/rust](https://www.reddit.com/r/rust/comments/1bocr5s/announcing_egui_027_with_improved_menus_and/)
* [ ] [r/programming](https://www.reddit.com/r/programming/comments/1bocsf6/announcing_egui_027_an_easytouse_crossplatform/)
* [ ] [This Week in Rust](https://github.com/rust-lang/this-week-in-rust/pull/5167)

## After release
* [ ] publish new `eframe_template`
* [ ] publish new `egui_plot`
* [ ] publish new `egui_tiles`
