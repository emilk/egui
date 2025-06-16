# Releases
## Cadence
We don't have a regular cadence, but there is usually a new major release every two months or so.

Often a major release is followed by one or two patch releases within a week or two.

## Versioning
All crates under the [`crates/`](crates/) folder are published in lock-step, with the same version number. This means that we won't publish a new breaking change of a single crate without also publishing all other crates. This also means we sometimes do a new release of a crate even though there are no changes to that crate.

The only exception to this are patch releases, where we sometimes only patch a single crate.

The egui version in egui `main` is always the version of the last published crates. This is so that users can easily patch their egui crates to egui `main` if they want to.

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

## Release testing
* [ ] `cargo r -p egui_demo_app` and click around for while
* [ ] update `eframe_template` and test
* [ ] update `egui_plot` and test
* [ ] update `egui_table` and test
* [ ] update `egui_tiles` and test
* [ ] test with Rerun
* [ ] `./scripts/check.sh`
* [ ] check that CI is green

## Preparation
* [ ] make sure there are no important unmerged PRs
* [ ] run `scripts/generate_example_screenshots.sh` if needed
* [ ] write a short release note that fits in a bluesky post
* [ ] record gif for `CHANGELOG.md` release note (and later bluesky post)
* [ ] update changelogs using `scripts/generate_changelog.py --version 0.x.0 --write`
* [ ] bump version numbers in workspace `Cargo.toml`

## Actual release
I usually do this all on the `main` branch, but doing it in a release branch is also fine, as long as you remember to merge it into `main` later.

* [ ] Run `typos`
* [ ] `git commit -m 'Release 0.x.0 - <release title>'`
* [ ] `cargo publish` (see below)
* [ ] `git tag -a 0.x.0 -m 'Release 0.x.0 - <release title>'`
* [ ] `git pull --tags ; git tag -d latest && git tag -a latest -m 'Latest release' && git push --tags origin latest --force ; git push --tags`
* [ ] merge release PR or push to `main`
* [ ] check that CI is green
* [ ] do a GitHub release: https://github.com/emilk/egui/releases/new
  * Follow the format of the last release
* [ ] wait for documentation to build: https://docs.rs/releases/queue

###  `cargo publish`:
```
(cd crates/emath                && cargo publish --quiet)  &&  echo "✅ emath"
(cd crates/ecolor               && cargo publish --quiet)  &&  echo "✅ ecolor"
(cd crates/epaint_default_fonts && cargo publish --quiet)  &&  echo "✅ epaint_default_fonts"
(cd crates/epaint               && cargo publish --quiet)  &&  echo "✅ epaint"
(cd crates/egui                 && cargo publish --quiet)  &&  echo "✅ egui"
(cd crates/egui-winit           && cargo publish --quiet)  &&  echo "✅ egui-winit"
(cd crates/egui-wgpu            && cargo publish --quiet)  &&  echo "✅ egui-wgpu"
(cd crates/eframe               && cargo publish --quiet)  &&  echo "✅ eframe"
(cd crates/egui_kittest         && cargo publish --quiet)  &&  echo "✅ egui_kittest"
(cd crates/egui_extras          && cargo publish --quiet)  &&  echo "✅ egui_extras"
(cd crates/egui_demo_lib        && cargo publish --quiet)  &&  echo "✅ egui_demo_lib"
(cd crates/egui_glow            && cargo publish --quiet)  &&  echo "✅ egui_glow"
```

\<continue with the checklist above\>

## Announcements
* [ ] [Bluesky](https://bsky.app/profile/ernerfeldt.bsky.social)
* [ ] egui discord
* [ ] [r/rust](https://www.reddit.com/r/rust/comments/1bocr5s/announcing_egui_027_with_improved_menus_and/)
* [ ] [r/programming](https://www.reddit.com/r/programming/comments/1bocsf6/announcing_egui_027_an_easytouse_crossplatform/)
* [ ] [This Week in Rust](https://github.com/rust-lang/this-week-in-rust/pull/5167)

## After release
* [ ] publish new `eframe_template`
* [ ] publish new `egui_plot`
* [ ] publish new `egui_table`
* [ ] publish new `egui_tiles`
* [ ] make a PR to `egui_commonmark`
* [ ] make a PR to `rerun`
