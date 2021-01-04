use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

// ----------------------------------------------------------------------------

/// A key-value store backed by a JSON file on disk.
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
            kv: read_json(&path).unwrap_or_default(),
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
            serde_json::to_writer(std::fs::File::create(&self.path).unwrap(), &self.kv).unwrap();
            self.dirty = false;
        }
    }
}

// ----------------------------------------------------------------------------

pub fn read_json<T>(memory_json_path: impl AsRef<Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    match std::fs::File::open(memory_json_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    eprintln!("ERROR: Failed to parse json: {}", err);
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
pub fn read_memory(ctx: &egui::Context, memory_json_path: impl AsRef<std::path::Path>) {
    let memory: Option<egui::Memory> = read_json(memory_json_path);
    if let Some(memory) = memory {
        *ctx.memory() = memory;
    }
}

/// Alternative to `FileStorage`
pub fn write_memory(
    ctx: &egui::Context,
    memory_json_path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(std::fs::File::create(memory_json_path)?, &*ctx.memory())?;
    Ok(())
}
