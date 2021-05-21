use std::default::Default;
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Localization {
    pub lang: &'static str,
    pub slider_tooltip: &'static str,
    pub click_copy: &'static str,
    pub cp_edit: &'static str,
    pub cp_blending: &'static str,
    pub cp_additive: &'static str,
    pub cp_normal: &'static str,
    pub cp_selected_color: &'static str,
    pub cp_hue: &'static str,
    pub cp_saturation: &'static str,
    pub cp_value: &'static str,
    pub lang_text: &'static str,
}

impl Default for Localization {
    fn default() -> Self {
        Self {
            lang: "English",
            slider_tooltip: "Drag to edit or click to enter a value.\nPress 'Shift' while dragging for better control",
            click_copy: "Click to copy",
            cp_edit: "Click to edit color",
            cp_blending: "Blending",
            cp_additive: "Additive",
            cp_normal: "Normal",
            cp_selected_color: "Selected color",
            cp_hue: "Hue",
            cp_saturation: "Saturation",
            cp_value: "Value",
            lang_text: "Language",
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
    pub fn set_localization(&mut self, lang: Language) {
        *self = match lang {
            Language::English => Localization::default(),
            Language::BahasaMalaysia => Localization::malay(),
        };
    }

    pub fn lang(&self) -> Language {
        match self.lang {
            "English" => Language::English,
            "Bahasa Malaysia" => Language::BahasaMalaysia,
            _ => Language::English,
        }
    }

    pub fn malay() -> Self {
        Self {
            lang: "Bahasa Malaysia",
            slider_tooltip: "Tarik untuk ubah atau klik untuk masukkan jumlah.\nTekan 'Shift' sambil tarik untuk pergerakan lebih terkawal.",
            click_copy: "Klik untuk salin",
            cp_edit: "Klik untuk ubah warna",
            cp_blending: "Campuran",
            cp_additive: "Tambahan",
            cp_normal: "Biasa",
            cp_selected_color: "Warna pilihan",
            cp_hue: "Rona",
            cp_saturation: "Ketepuan",
            cp_value: "Nilai",
            lang_text: "Bahasa",
        }
    }
}
