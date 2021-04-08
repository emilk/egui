use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

// ----------------------------------------------------------------------------

/// A key-value store backed by a [RON](https://github.com/ron-rs/ron) file on disk.
/// Used to restore egui state, glium window position/size and app state.
pub struct FileStorage {
    path: PathBuf,
    kv: HashMap<String, String>,
    dirty: bool,
}

impl FileStorage {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path: PathBuf = path.into();
        Self {
            kv: read_ron(&path).unwrap_or_default(),
            path,
            dirty: false,
        }
    }
}

impl epi::Storage for FileStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.kv.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        if self.kv.get(key) != Some(&value) {
            self.kv.insert(key.to_owned(), value);
            self.dirty = true;
        }
    }

    fn flush(&mut self) {
        if self.dirty {
            // eprintln!("Persisted to {}", self.path.display());
            let file = std::fs::File::create(&self.path).unwrap();
            let config = Default::default();
            ron::ser::to_writer_pretty(file, &self.kv, config).unwrap();
            self.dirty = false;
        }
    }
}

// ----------------------------------------------------------------------------

pub fn read_ron<T>(ron_path: impl AsRef<Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    match std::fs::File::open(ron_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match ron::de::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    eprintln!("ERROR: Failed to parse RON: {}", err);
                    None
                }
            }
        }
        Err(_err) => {
            // File probably doesn't exist. That's fine.
            None
        }
    }
}
// ----------------------------------------------------------------------------

/// Alternative to `FileStorage`
pub fn read_memory(ctx: &egui::Context, memory_file_path: impl AsRef<std::path::Path>) {
    let memory: Option<egui::Memory> = read_ron(memory_file_path);
    if let Some(memory) = memory {
        *ctx.memory() = memory;
    }
}

/// Alternative to `FileStorage`
pub fn write_memory(
    ctx: &egui::Context,
    memory_file_path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(memory_file_path)?;
    let ron_config = Default::default();
    ron::ser::to_writer_pretty(file, &*ctx.memory(), ron_config)?;
    Ok(())
}
