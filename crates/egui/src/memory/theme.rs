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

    /// Chooses between [`Self::Dark`] or [`Self::Light`] based on a boolean value.
    pub fn from_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self::Dark
        } else {
            Self::Light
        }
    }
}
