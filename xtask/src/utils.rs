#![expect(clippy::unwrap_used)]

use std::{
    env,
    io::{self, Write as _},
    process::Command,
};

use super::DynError;

/// Print the command and its arguments as if the user had typed them
pub fn print_cmd(cmd: &Command) {
    print!("{} ", cmd.get_program().to_string_lossy());
    for arg in cmd.get_args() {
        print!("{} ", arg.to_string_lossy());
    }
    println!();
}

/// Prompt user before running a command
///
/// Adapted from [miri](https://github.com/rust-lang/miri/blob/dba35d2be72f4b78343d1a0f0b4737306f310672/cargo-miri/src/util.rs#L181-L204)
pub fn ask_to_run(mut cmd: Command, ask: bool, reason: &str) -> Result<(), DynError> {
    // Disable interactive prompts in CI (GitHub Actions, Travis, AppVeyor, etc).
    // Azure doesn't set `CI` though (nothing to see here, just Microsoft being Microsoft),
    // so we also check their `TF_BUILD`.
    let is_ci = env::var_os("CI").is_some() || env::var_os("TF_BUILD").is_some();
    if ask && !is_ci {
        let mut buf = String::new();
        print!("The script is going to run: \n\n`{cmd:?}`\n\n To {reason}.\nProceed? [Y/n] ",);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buf).unwrap();
        match buf.trim().to_lowercase().as_ref() {
            "" | "y" | "yes" => {}
            "n" | "no" => return Err("Aborting as per your request".into()),
            a => return Err(format!("Invalid answer `{a}`").into()),
        }
    } else {
        println!("Running `{cmd:?}` to {reason}.");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("failed to {reason}: {status}").into());
    }
    Ok(())
}
