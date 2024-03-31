//! Run `cargo deny`
//!
//! Also installs the subcommand if it is not already installed.

use std::process::Command;

use super::DynError;

pub fn deny(args: &[&str]) -> Result<(), DynError> {
    if !args.is_empty() {
        eprintln!("Warning: arguments ignored: {args:?}");
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
        let status = Command::new("cargo")
            .args([
                "deny",
                "--all-features",
                "--log-level",
                "error",
                "--target",
                target,
                "check",
            ])
            .status()?;
        if !status.success() {
            return Err(status.to_string().into());
        }
    }
    Ok(())
}

fn install_cargo_deny() -> Result<(), DynError> {
    let status = Command::new("cargo")
        .args(["+stable", "install", "--quiet", "--locked", "cargo-deny"])
        .status()?;
    if !status.success() {
        return Err(status.to_string().into());
    }
    Ok(())
}
