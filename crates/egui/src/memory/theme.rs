use crate::Button;

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
        if dark_mode { Self::Dark } else { Self::Light }
    }
}

impl Theme {
    /// Show small toggle-button for light and dark mode.
    /// This is not the best design as it doesn't allow switching back to "follow system".
    #[must_use]
    pub(crate) fn small_toggle_button(self, ui: &mut crate::Ui) -> Option<Self> {
        #![expect(clippy::collapsible_else_if)]
        if self == Self::Dark {
            if ui
                .add(Button::new("☀").frame(false))
                .on_hover_text("Switch to light mode")
                .clicked()
            {
                return Some(Self::Light);
            }
        } else {
            if ui
                .add(Button::new("🌙").frame(false))
                .on_hover_text("Switch to dark mode")
                .clicked()
            {
                return Some(Self::Dark);
            }
        }
        None
    }
}

/// The user's theme preference.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ThemePreference {
    /// Dark mode: light text on a dark background.
    Dark,

    /// Light mode: dark text on a light background.
    Light,

    /// Follow the system's theme preference.
    #[default]
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

impl ThemePreference {
    /// A short icon representing this preference.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Dark => "🌙",
            Self::Light => "☀",
            Self::System => "💻",
        }
    }

    /// A short name for this preference, e.g. "dark".
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    /// Show a row of buttons for picking between dark mode, light mode,
    /// and following the system theme.
    ///
    /// The button of the current preference is highlighted.
    /// Each button is an icon (see [`Self::icon`]) with a tooltip explaining it.
    ///
    /// See also [`crate::global_theme_preference_buttons`], which does the same
    /// for the theme of the whole [`crate::Context`].
    pub fn buttons(&mut self, ui: &mut crate::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            ui.selectable_value(self, Self::System, Self::System.icon())
                .on_hover_ui(system_hover_ui);

            ui.selectable_value(self, Self::Dark, Self::Dark.icon())
                .on_hover_text("Use the dark mode theme");

            ui.selectable_value(self, Self::Light, Self::Light.icon())
                .on_hover_text("Use the light mode theme");
        });
    }

    /// Show a row of buttons for picking the theme preference.
    #[deprecated = "Renamed to `buttons`"]
    pub fn radio_buttons(&mut self, ui: &mut crate::Ui) {
        self.buttons(ui);
    }
}

fn system_hover_ui(ui: &mut crate::Ui) {
    ui.label("Follow the system theme preference.");

    ui.add_space(4.0);

    if let Some(system_theme) = ui.input(|i| i.raw.system_theme) {
        ui.label(format!(
            "The current system theme is: {}",
            match system_theme {
                Theme::Dark => "dark",
                Theme::Light => "light",
            }
        ));
    } else {
        ui.label("The system theme is unknown.");
    }
}
