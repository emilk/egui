use std::default::Default;
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Localization {
    pub slider_tooltip: String,
    pub click_copy: String,
    pub cp_blending: String,
    pub cp_additive: String,
    pub cp_normal: String,
    pub cp_selected_color: String,
    pub cp_hue: String,
    pub cp_saturation: String,
    pub cp_value: String,
}

impl Default for Localization {
    fn default() -> Self {
        Self {
            slider_tooltip: "Drag to edit or click to enter a value.\nPress 'Shift' while dragging for better control".to_string(),
            click_copy: "Click to copy".to_string(),
            cp_blending: "Blending".to_string(),
            cp_additive: "Additive".to_string(),
            cp_normal: "Normal".to_string(),
            cp_selected_color: "Selected color".to_string(),
            cp_hue: "Hue".to_string(),
            cp_saturation: "Saturation".to_string(),
            cp_value: "Value".to_string(),
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
            Language::English => Localization::default(),
            Language::BahasaMalaysia => Localization::malay(),
        }
    }

    pub fn malay() -> Self {
        Self {
            slider_tooltip: "Tarik untuk ubah atau klik untuk masukkan jumlah.\nTekan 'Shift' sambil tarik untuk pergerakan lebih terkawal.".to_string(),
            click_copy: "Klik untuk salin".to_string(),
            cp_blending: "Campuran".to_string(),
            cp_additive: "Tambahan".to_string(),
            cp_normal: "Biasa".to_string(),
            cp_selected_color: "Warna pilihan".to_string(),
            cp_hue: "Rona".to_string(),
            cp_saturation: "Ketepuan".to_string(),
            cp_value: "Nilai".to_string(),
        }
    }
}
