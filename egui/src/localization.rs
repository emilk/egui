use std::default::Default;
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Localization {
    pub slider_tooltip: String,
}

impl Default for Localization {
    fn default() -> Self {
        Self {
            slider_tooltip: "Drag to edit or click to enter a value.\nPress 'Shift' while dragging for better control".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub enum Language {
    English,
    BahasaMalaysia,
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

impl Localization {
    pub(crate) fn load_new_localization(&mut self, localization: &Localization) {
        *self = Localization {
            ..localization.to_owned()
        };
    }

    pub fn get_localization(lang: Language) -> Self {
        match lang {
            Language::English => Localization::english(),
            Language::BahasaMalaysia => Localization::malay(),
        }
    }

    fn english() -> Self {
        Self {
            slider_tooltip: "Drag to edit or click to enter a value.\nPress 'Shift' while dragging for better control".to_string(),
        }
    }

    pub fn malay() -> Self {
        Self {
            slider_tooltip: "Tarik untuk ubah atau klik untuk masukkan jumlah.\nTekan 'Shift' sambil tarik untuk pergerakan lebih terkawal.".to_string(),
        }
    }
}
