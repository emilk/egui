# Contributing guidelines

## Introduction

`egui` has been an on-and-off weekend project of mine since late 2018. I am grateful to any help I can get, but bare in mind that sometimes I can be slow to respond because I am busy with other things!

/ Emil


## Discussion

You can ask questions, share screenshots and more at [GitHub Discussions](https://github.com/emilk/egui/discussions).

There is also an `egui` channel on the Embark discord at <https://discord.gg/vY8ZGS292W> (NOTE: I work at [Embark](https://www.embark-studios.com/), but `egui` is my hobby project).


## Filing an issue

[Issues](https://github.com/emilk/egui/issues) are for bug reports and feature requests. Issues are not for asking questions (use [Discussions](https://github.com/emilk/egui/discussions) for that).

Always make sure there is not already a similar issue to the one you are creating.

If you are filing a bug, please provide a way to reproduce it.


## Making a PR

First file an issue (or find an existing one) and announce that you plan to work on something. That way we will avoid having several people doing double work. Please ask for feedback before you start working on something non-trivial!

Browse through [`ARCHITECTURE.md`](https://github.com/emilk/egui/blob/master/ARCHITECTURE.md) to get a sense of how all pieces connects.

You can test your code locally by running `./sh/check.sh`.

When you have something that works, open a draft PR. You may get some helpful feedback early!
When you feel the PR is ready to go, do a self-review of the code, and then open it for review.

Please keep pull requests small and focused.

Do not include the `.js` and `.wasm` build artifacts generated for building for web.
`git` is not great at storing large files like these, so we only commit a new web demo after a new egui release.


## Creating an integration for egui

If you make an integration for `egui` for some engine or renderer, please share it with the world!
I will add a link to it from the `egui` README.md so others can easily find it.

Read the section on integrations at <https://github.com/emilk/egui#integrations>.


## Code Conventions
Conventions unless otherwise specified:

* angles are in radians
* `Vec2::X` is right and `Vec2::Y` is down.
* `Pos2::ZERO` is left top.

While using an immediate mode gui is simple, implementing one is a lot more tricky. There are many subtle corner-case you need to think through. The `egui` source code is a bit messy, partially because it is still evolving.

* read some code before writing your own
* follow the `egui` code style
* write idiomatic rust
* avoid `unsafe`
* avoid code that can cause panics
* use good names for everything
* add docstrings to types, `struct` fields and all `pub fn`.
* add some example code (doc-tests)
* before making a function longer, consider adding a helper function
* break the above rules when it makes sense
