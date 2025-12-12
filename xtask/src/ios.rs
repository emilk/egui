//! iOS app bundling functionality
//!
//! Creates .app bundles for iOS simulator or device targets.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::DynError;

/// Target platform for iOS builds
#[derive(Debug, Clone, Copy)]
enum IosTarget {
    /// Physical iOS device (aarch64-apple-ios)
    Device,
    /// iOS Simulator (aarch64-apple-ios-sim)
    Simulator,
}

impl IosTarget {
    fn triple(&self) -> &'static str {
        match self {
            Self::Device => "aarch64-apple-ios",
            Self::Simulator => "aarch64-apple-ios-sim",
        }
    }
}

/// Configuration for iOS bundle
struct BundleConfig {
    package: String,
    target: IosTarget,
    release: bool,
    install: bool,
    launch: bool,
    simulator_name: Option<String>,
}

pub fn bundle_ios(args: &[&str]) -> Result<(), DynError> {
    let config = parse_args(args)?;

    // Step 1: Build the Rust code
    println!("Building {} for {}...", config.package, config.target.triple());
    build_rust(&config)?;

    // Step 2: Create the app bundle
    println!("Creating app bundle...");
    let app_path = create_bundle(&config)?;
    println!("Created: {}", app_path.display());

    // Step 3: Install to simulator (if requested)
    if config.install {
        let sim_name = config.simulator_name.as_deref().unwrap_or("iPhone 15 Pro");
        println!("Installing to simulator '{}'...", sim_name);
        install_to_simulator(&app_path, sim_name)?;

        // Step 4: Launch (if requested)
        if config.launch {
            let bundle_id = format!("com.egui.{}", config.package.replace('_', "-"));
            println!("Launching {}...", bundle_id);
            launch_on_simulator(&bundle_id, sim_name)?;
        }
    }

    println!("Done!");
    Ok(())
}

fn parse_args(args: &[&str]) -> Result<BundleConfig, DynError> {
    let mut package = None;
    let mut target = IosTarget::Simulator;
    let mut release = false;
    let mut install = false;
    let mut launch = false;
    let mut simulator_name = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-h" | "--help" => {
                print_ios_help();
                std::process::exit(0);
            }
            "-p" | "--package" => {
                i += 1;
                package = args.get(i).map(|s| s.to_string());
            }
            "--sim" | "--simulator" => {
                target = IosTarget::Simulator;
            }
            "--device" => {
                target = IosTarget::Device;
            }
            "-r" | "--release" => {
                release = true;
            }
            "--install" => {
                install = true;
            }
            "--launch" => {
                install = true; // launch implies install
                launch = true;
            }
            "--simulator-name" => {
                i += 1;
                simulator_name = args.get(i).map(|s| s.to_string());
            }
            other => {
                // If no flag, treat as package name
                if !other.starts_with('-') && package.is_none() {
                    package = Some(other.to_string());
                } else {
                    return Err(format!("Unknown argument: {}", other).into());
                }
            }
        }
        i += 1;
    }

    let package = package.ok_or("Package name required. Use -p <name> or provide as argument.")?;

    Ok(BundleConfig {
        package,
        target,
        release,
        install,
        launch,
        simulator_name,
    })
}

fn print_ios_help() {
    println!(
        r#"
cargo xtask bundle-ios - Build and bundle an iOS app

USAGE:
    cargo xtask bundle-ios [OPTIONS] <PACKAGE>
    cargo xtask bundle-ios -p <PACKAGE> [OPTIONS]

ARGS:
    <PACKAGE>    Name of the package to bundle (e.g., hello_ios)

OPTIONS:
    -p, --package <NAME>       Package to bundle
    --sim, --simulator         Build for iOS Simulator (default)
    --device                   Build for physical iOS device
    -r, --release              Build in release mode
    --install                  Install to simulator after building
    --launch                   Install and launch on simulator
    --simulator-name <NAME>    Simulator to use (default: "iPhone 15 Pro")
    -h, --help                 Show this help message

EXAMPLES:
    cargo xtask bundle-ios hello_ios --sim
    cargo xtask bundle-ios -p hello_ios --launch
    cargo xtask bundle-ios hello_ios --device --release
"#
    );
}

fn build_rust(config: &BundleConfig) -> Result<(), DynError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg(&config.package)
        .arg("--target")
        .arg(config.target.triple());

    if config.release {
        cmd.arg("--release");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Cargo build failed with status: {}", status).into());
    }

    Ok(())
}

fn create_bundle(config: &BundleConfig) -> Result<PathBuf, DynError> {
    let profile = if config.release { "release" } else { "debug" };
    let target_dir = Path::new("target")
        .join(config.target.triple())
        .join(profile);

    // Find the binary
    let binary_path = target_dir.join(&config.package);
    if !binary_path.exists() {
        return Err(format!(
            "Binary not found at {}. Make sure the package builds a binary.",
            binary_path.display()
        ).into());
    }

    // Create bundle directory
    let bundle_dir = target_dir.join("bundle").join("ios");
    let app_name = config.package.replace('_', " ");
    let app_name = to_title_case(&app_name);
    let app_path = bundle_dir.join(format!("{}.app", app_name));

    // Clean and recreate
    if app_path.exists() {
        fs::remove_dir_all(&app_path)?;
    }
    fs::create_dir_all(&app_path)?;

    // Copy binary
    let dest_binary = app_path.join(&config.package);
    fs::copy(&binary_path, &dest_binary)?;

    // Create Info.plist
    let bundle_id = format!("com.egui.{}", config.package.replace('_', "-"));
    let info_plist = create_info_plist(&app_name, &bundle_id, &config.package);
    let plist_path = app_path.join("Info.plist");
    fs::write(&plist_path, info_plist)?;

    // Copy icon if it exists
    let icon_path = Path::new("crates/eframe/data/icon.png");
    if icon_path.exists() {
        fs::copy(icon_path, app_path.join("AppIcon.png"))?;
    }

    Ok(app_path)
}

fn create_info_plist(app_name: &str, bundle_id: &str, executable: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>{app_name}</string>
    <key>CFBundleExecutable</key>
    <string>{executable}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{app_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
    <key>UIDeviceFamily</key>
    <array>
        <integer>1</integer>
        <integer>2</integer>
    </array>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>arm64</string>
    </array>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationPortraitUpsideDown</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
</dict>
</plist>
"#
    )
}

fn install_to_simulator(app_path: &Path, simulator_name: &str) -> Result<(), DynError> {
    // Boot the simulator if needed
    let _ = Command::new("xcrun")
        .args(["simctl", "boot", simulator_name])
        .output();

    // Install the app
    let status = Command::new("xcrun")
        .args(["simctl", "install", simulator_name])
        .arg(app_path)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to install app to simulator: {}", status).into());
    }

    // Open Simulator.app
    let _ = Command::new("open")
        .args(["-a", "Simulator"])
        .status();

    Ok(())
}

fn launch_on_simulator(bundle_id: &str, simulator_name: &str) -> Result<(), DynError> {
    let status = Command::new("xcrun")
        .args(["simctl", "launch", simulator_name, bundle_id])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to launch app: {}", status).into());
    }

    Ok(())
}

/// Convert "hello_world" or "hello world" to "Hello World"
fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
