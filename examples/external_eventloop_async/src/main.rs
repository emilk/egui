#![allow(rustdoc::missing_crate_level_docs, reason = "it's an example")]

#[cfg(target_os = "linux")]
mod app;

#[cfg(target_os = "linux")]
fn main() -> std::io::Result<()> {
    app::run()
}

// Do not check `app` on unsupported platforms when check "--all-features" is used in CI.
#[cfg(not(target_os = "linux"))]
fn main() {
    #![expect(clippy::print_stdout, reason = "Used for testing")]
    println!("This example only supports Linux.");
}
