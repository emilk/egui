//! Helper crate for running scripts within the `egui` repo

#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::exit)]

mod deny;
pub(crate) mod utils;

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e}");
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let arg_strings: Vec<_> = std::env::args().skip(1).collect();
    let args: Vec<_> = arg_strings.iter().map(String::as_str).collect();

    match args.as_slice() {
        &[] | &["-h"] | &["--help"] => print_help(),
        &["deny", ..] => deny::deny(&args[1..])?,
        c => Err(format!("Invalid arguments {c:?}"))?,
    }
    Ok(())
}

fn print_help() {
    let help = "
    xtask help

    Subcommands
    deny: Run cargo-deny for all targets

    Options
    -h, --help: print help and exit
        ";
    println!("{help}");
}
