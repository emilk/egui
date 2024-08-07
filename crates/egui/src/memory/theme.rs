/// Dark or Light theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Theme {
    /// Dark mode: light text on a dark background.
    Dark,

    /// Light mode: dark text on a light background.
    Light,
}

impl Theme {
    /// Default visuals for this theme.
    pub fn default_visuals(self) -> crate::Visuals {
        match self {
            Self::Dark => crate::Visuals::dark(),
            Self::Light => crate::Visuals::light(),
        }
    }

    /// Default style for this theme.
    pub fn default_style(self) -> crate::Style {
        crate::Style {
            visuals: self.default_visuals(),
            ..Default::default()
        }
    }

    /// Chooses between [`Self::Dark`] or [`Self::Light`] based on a boolean value.
    pub fn from_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self::Dark
        } else {
            Self::Light
        }
    }
}

/// The user's theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ThemePreference {
    /// Dark mode: light text on a dark background.
    Dark,

    /// Light mode: dark text on a light background.
    Light,

    /// Follow the system's theme preference.
    System,
}

impl From<Theme> for ThemePreference {
    fn from(value: Theme) -> Self {
        match value {
            Theme::Dark => Self::Dark,
            Theme::Light => Self::Light,
        }
    }
}
