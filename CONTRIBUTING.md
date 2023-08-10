# Contributing guidelines

## Introduction

`egui` has been an on-and-off weekend project of mine since late 2018. I am grateful to any help I can get, but bare in mind that sometimes I can be slow to respond because I am busy with other things!

/ Emil


## Discussion

You can ask questions, share screenshots and more at [GitHub Discussions](https://github.com/emilk/egui/discussions).

There is an `egui` discord at <https://discord.gg/vbuv9Xan65>.


## Filing an issue

[Issues](https://github.com/emilk/egui/issues) are for bug reports and feature requests. Issues are not for asking questions (use [Discussions](https://github.com/emilk/egui/discussions) or [Discord](https://discord.gg/vbuv9Xan65) for that).

Always make sure there is not already a similar issue to the one you are creating.

If you are filing a bug, please provide a way to reproduce it.


## Making a PR

First file an issue (or find an existing one) and announce that you plan to work on something. That way we will avoid having several people doing double work. Please ask for feedback before you start working on something non-trivial!

Browse through [`ARCHITECTURE.md`](ARCHITECTURE.md) to get a sense of how all pieces connects.

You can test your code locally by running `./scripts/check.sh`.

When you have something that works, open a draft PR. You may get some helpful feedback early!
When you feel the PR is ready to go, do a self-review of the code, and then open it for review.

Please keep pull requests small and focused.

Don't worry about having many small commits in the PR - they will be squashed to one commit once merged.

Do not include the `.js` and `.wasm` build artifacts generated for building for web.
`git` is not great at storing large files like these, so we only commit a new web demo after a new egui release.


## Creating an integration for egui

If you make an integration for `egui` for some engine or renderer, please share it with the world!
I will add a link to it from the `egui` README.md so others can easily find it.

Read the section on integrations at <https://github.com/emilk/egui#integrations>.


## Testing the web viewer
* Install some tools with `scripts/setup_web.sh`
* Build with `scripts/build_demo_web.sh`
* Host with `scripts/start_server.sh`
* Open <http://localhost:8888/index.html>


## Code Conventions
Conventions unless otherwise specified:

* angles are in radians
* `Vec2::X` is right and `Vec2::Y` is down.
* `Pos2::ZERO` is left top.

While using an immediate mode gui is simple, implementing one is a lot more tricky. There are many subtle corner-case you need to think through. The `egui` source code is a bit messy, partially because it is still evolving.

* Read some code before writing your own.
* Follow the `egui` code style.
* Add blank lines around all `fn`, `struct`, `enum`, etc.
* `// Comment like this.` and not `//like this`.
* Use `TODO` instead of `FIXME`.
* Add your github handle to the `TODO`:s you write, e.g: `TODO(emilk): clean this up`.
* Write idiomatic rust.
* Avoid `unsafe`.
* Avoid code that can cause panics.
* Use good names for everything.
* Add docstrings to types, `struct` fields and all `pub fn`.
* Add some example code (doc-tests).
* Before making a function longer, consider adding a helper function.
* If you are only using it in one function, put the `use` statement in that function. This improves locality, making it easier to read and move the code.
* When importing a `trait` to use it's trait methods, do this: `use Trait as _;`. That lets the reader know why you imported it, even though it seems unused.
* Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
* Break the above rules when it makes sense.


### Good:
``` rust
/// The name of the thing.
fn name(&self) -> &str {
    &self.name
}

fn foo(&self) {
    // TODO(emilk): implement
}
```

### Bad:
``` rust
//some function
fn get_name(&self) -> &str {
    &self.name
}
fn foo(&self) {
    //FIXME: implement
}
```
