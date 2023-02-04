use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

// ----------------------------------------------------------------------------

/// A key-value store backed by a [RON](https://github.com/ron-rs/ron) file on disk.
/// Used to restore egui state, glium window position/size and app state.
pub struct FileStorage {
    ron_filepath: PathBuf,
    kv: HashMap<String, String>,
    dirty: bool,
    last_save_join_handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for FileStorage {
    fn drop(&mut self) {
        if let Some(join_handle) = self.last_save_join_handle.take() {
            join_handle.join().ok();
        }
    }
}

impl FileStorage {
    /// Store the state in this .ron file.
    pub fn from_ron_filepath(ron_filepath: impl Into<PathBuf>) -> Self {
        let ron_filepath: PathBuf = ron_filepath.into();
        tracing::debug!("Loading app state from {:?}…", ron_filepath);
        Self {
            kv: read_ron(&ron_filepath).unwrap_or_default(),
            ron_filepath,
            dirty: false,
            last_save_join_handle: None,
        }
    }

    /// Find a good place to put the files that the OS likes.
    pub fn from_app_name(app_name: &str) -> Option<Self> {
        if let Some(proj_dirs) = directories_next::ProjectDirs::from("", "", app_name) {
            let data_dir = proj_dirs.data_dir().to_path_buf();
            if let Err(err) = std::fs::create_dir_all(&data_dir) {
                tracing::warn!(
                    "Saving disabled: Failed to create app path at {:?}: {}",
                    data_dir,
                    err
                );
                None
            } else {
                Some(Self::from_ron_filepath(data_dir.join("app.ron")))
            }
        } else {
            tracing::warn!("Saving disabled: Failed to find path to data_dir.");
            None
        }
    }
}

impl crate::Storage for FileStorage {
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
            self.dirty = false;

            let file_path = self.ron_filepath.clone();
            let kv = self.kv.clone();

            if let Some(join_handle) = self.last_save_join_handle.take() {
                // wait for previous save to complete.
                join_handle.join().ok();
            }

            let join_handle = std::thread::spawn(move || {
                let file = std::fs::File::create(&file_path).unwrap();
                let config = Default::default();
                ron::ser::to_writer_pretty(file, &kv, config).unwrap();
                tracing::trace!("Persisted to {:?}", file_path);
            });

            self.last_save_join_handle = Some(join_handle);
        }
    }
}

// ----------------------------------------------------------------------------

fn read_ron<T>(ron_path: impl AsRef<Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    match std::fs::File::open(ron_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match ron::de::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    tracing::warn!("Failed to parse RON: {}", err);
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
