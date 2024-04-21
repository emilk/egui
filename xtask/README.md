## xtask - Task automation

This crate is meant to automate common tasks on the repository. It serves as a
replacement for shell scripts that is more portable across host operating
systems (namely Windows) and hopefully also easier to work with for
contributors who are already familiar with Rust (and not necessarily with shell
scripting).

The executable can be invoked via the subcommand `cargo xtask`, thanks to an
alias defined in `.cargo/config.toml`.

For more information, see <https://github.com/matklad/cargo-xtask>.
