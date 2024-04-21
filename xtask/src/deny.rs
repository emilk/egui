//! Run `cargo deny`
//!
//! Also installs the subcommand if it is not already installed.

use std::process::Command;

use super::DynError;

pub fn deny(args: &[&str]) -> Result<(), DynError> {
    if !args.is_empty() {
        return Err(format!("Invalid arguments: {args:?}").into());
    }
    install_cargo_deny()?;
    let targets = [
        "aarch64-apple-darwin",
        "aarch64-linux-android",
        "i686-pc-windows-gnu",
        "i686-pc-windows-msvc",
        "i686-unknown-linux-gnu",
        "wasm32-unknown-unknown",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-unknown-redox",
    ];
    for target in targets {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "deny",
            "--all-features",
            "--log-level",
            "error",
            "--target",
            target,
            "check",
        ]);
        super::utils::print_cmd(&cmd);
        let status = cmd.status()?;
        if !status.success() {
            return Err(status.to_string().into());
        }
    }
    Ok(())
}

fn install_cargo_deny() -> Result<(), DynError> {
    let already_installed = Command::new("cargo")
        .args(["deny", "--version"])
        .output()
        .is_ok_and(|out| out.status.success());
    if already_installed {
        return Ok(());
    }
    let mut cmd = Command::new("cargo");
    cmd.args(["+stable", "install", "--quiet", "--locked", "cargo-deny"]);
    let reason = "install cargo-deny";
    super::utils::ask_to_run(cmd, true, reason)
}
