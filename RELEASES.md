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
* [ ] copy this checklist to a new egui issue, called "Release 0.xx.y"
* [ ] close all issues in the milestone for this release

## Special steps for patch release
* [ ] make a branch off of the _latest_ release
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
* [ ] Create a branch called `release-0.xx.0` and open a PR for it
* [ ] run `scripts/generate_example_screenshots.sh` if needed
* [ ] write a short release note that fits in a bluesky post
* [ ] record gif for `CHANGELOG.md` release note (and later bluesky post)
* [ ] update changelogs
  * [ ] run `scripts/generate_changelog.py --version 0.x.0 --write`
  * [ ] read changelogs and clean them up if needed
  * [ ] write a good intro with highlight for the main changelog
* [ ] run `typos`

## Actual release
* [ ] bump version numbers in workspace `Cargo.toml`
* [ ] check that CI for the PR is green
* [ ] publish the crates by running `scripts/publish_crates.sh`
* [ ] `git tag -a 0.x.0 -m 'Release 0.x.0 - <release title>'`
* [ ] `git pull --tags ; git tag -d latest && git tag -a latest -m 'Latest release' && git push --tags origin latest --force ; git push --tags`
* [ ] merge release PR as `Release 0.x.0 - <release title>`
* [ ] check that CI for `main` is green
* [ ] do a GitHub release: https://github.com/emilk/egui/releases/new
  * follow the format of the last release
* [ ] wait for  the documentation build to finish: https://docs.rs/releases/queue
  * [ ] https://docs.rs/egui/ works
  * [ ] https://docs.rs/eframe/ works


## Announcements
* [ ] [Bluesky](https://bsky.app/profile/ernerfeldt.bsky.social)
* [ ] egui discord
* [ ] [r/rust](https://www.reddit.com/r/rust/comments/1bocr5s/announcing_egui_027_with_improved_menus_and/)
* [ ] [r/programming](https://www.reddit.com/r/programming/comments/1bocsf6/announcing_egui_027_an_easytouse_crossplatform/)
* [ ] [This Week in Rust](https://github.com/rust-lang/this-week-in-rust/pull/5167)


## After release
* [ ] update `eframe_template`
* [ ] publish new `egui_plot`
* [ ] publish new `egui_table`
* [ ] publish new `egui_tiles`
* [ ] make a PR to `egui_commonmark`
* [ ] make a PR to `rerun`


## Finally
* [ ] close the milestone
* [ ] close this issue
* [ ] improve `RELEASES.md` with what you learned this time around
