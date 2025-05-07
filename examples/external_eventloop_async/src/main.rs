#![allow(rustdoc::missing_crate_level_docs)] // it's an example

#[cfg(target_os = "linux")]
mod app;

#[cfg(target_os = "linux")]
fn main() -> std::io::Result<()> {
    app::run()
}

// Do not check `app` on unsupported platforms when check "--all-features" is used in CI.
#[cfg(not(target_os = "linux"))]
fn main() {
    println!("This example only supports Linux.");
}
