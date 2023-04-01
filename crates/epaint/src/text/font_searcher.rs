use std::fs::read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Searches for fonts.
pub struct FontSearcher {
    pub fonts: Vec<(PathBuf, Vec<u8>)>,
}

impl FontSearcher {
    /// Create a new, empty system searcher.
    pub fn new() -> Self {
        Self { fonts: vec![] }
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    pub fn search_system(&mut self) {}

    /// Search for fonts in the macOS system font directories.
    #[cfg(target_os = "macos")]
    pub fn search_system(&mut self) {
        self.search_dir("/Library/Fonts");
        self.search_dir("/Network/Library/Fonts");
        self.search_dir("/System/Library/Fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for fonts in the Windows system font directories.
    #[cfg(windows)]
    pub fn search_system(&mut self) {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

        self.search_dir(Path::new(&windir).join("Fonts"));

        if let Some(roaming) = dirs::config_dir() {
            self.search_dir(roaming.join("Microsoft\\Windows\\Fonts"));
        }

        if let Some(local) = dirs::cache_dir() {
            self.search_dir(local.join("Microsoft\\Windows\\Fonts"));
        }
    }

    /// Search for all fonts in a directory recursively.
    pub fn search_dir(&mut self, path: impl AsRef<Path>) {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("ttf" | "otf" | "TTF" | "OTF" | "ttc" | "otc" | "TTC" | "OTC"),
            ) {
                self.search_file(path);
            }
        }
    }

    /// Index the fonts in the file at the given path.
    pub fn search_file(&mut self, path: impl AsRef<Path>) {
        let content = read(path.as_ref()).unwrap();
        self.fonts.push((path.as_ref().to_path_buf(), content));
    }
}
