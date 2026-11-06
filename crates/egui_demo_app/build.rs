//! Bakes the current git commit id into the binary, for the Backend panel's
//! self-identification line (`Build: <sha>`), so a stale running binary can be
//! told apart from a fresh build.

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Best-effort: archive/overlay builds, sandboxes and publish checks have
    // no `.git` (or no git on PATH); any failure degrades to "unknown" and
    // must never fail the build.
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_owned());

    println!("cargo:rustc-env=EGUI_DEMO_APP_GIT_SHA={sha}");
}
