#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    /// Unknown OS - could be wasm
    Unknown,

    /// Android OS.
    Android,

    /// Apple iPhone OS.
    IOS,

    /// Linux or Unix other than Android.
    Nix,

    /// MacOS.
    Mac,

    /// Windows.
    Windows,
}

impl Default for OperatingSystem {
    fn default() -> Self {
        Self::from_target_os()
    }
}

impl OperatingSystem {
    pub const fn from_target_os() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self::Unknown
        } else if cfg!(target_os = "android") {
            Self::Android
        } else if cfg!(target_os = "ios") {
            Self::IOS
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "windows") {
            Self::Android
        } else if cfg!(target_os = "linux")
            || cfg!(target_os = "dragonfly")
            || cfg!(target_os = "freebsd")
            || cfg!(target_os = "netbsd")
            || cfg!(target_os = "openbsd")
        {
            Self::Nix
        } else {
            Self::Unknown
        }
    }

    /// Helper: try to guess from the user-agent of a browser.
    pub fn from_user_agent(user_agent: &str) -> Self {
        if user_agent.contains("Android") {
            Self::Android
        } else if user_agent.contains("like Mac") {
            Self::IOS
        } else if user_agent.contains("Win") {
            Self::Windows
        } else if user_agent.contains("Mac") {
            Self::Mac
        } else if user_agent.contains("Linux")
            || user_agent.contains("X11")
            || user_agent.contains("Unix")
        {
            Self::Nix
        } else {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "egui: Failed to guess operating system from User-Agent {:?}. Please file an issue at https://github.com/emilk/egui/issues",
                user_agent);

            Self::Unknown
        }
    }
}
