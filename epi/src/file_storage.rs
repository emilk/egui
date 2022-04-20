use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc,
};

// ----------------------------------------------------------------------------

/// A key-value store backed by a [RON](https://github.com/ron-rs/ron) file on disk.
/// Used to restore egui state, glium window position/size and app state.
pub struct FileStorage {
    kv: HashMap<String, String>,
    dirty: bool,
    flush_handle: Option<std::thread::JoinHandle<()>>,
    sender: mpsc::Sender<Message>,
}

enum Message {
    DoFlush(HashMap<String, String>),
    Terminate,
}

impl Drop for FileStorage {
    fn drop(&mut self) {
        self.sender.send(Message::Terminate).unwrap();
        if let Some(handle) = self.flush_handle.take() {
            handle.join().unwrap();
        }
    }
}

impl FileStorage {
    /// Store the state in this .ron file.
    pub fn from_ron_filepath(ron_filepath: impl Into<PathBuf>) -> Self {
        let ron_filepath: PathBuf = ron_filepath.into();
        let filepath = ron_filepath.clone();
        let (tx, rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();
        let flush_handle = Some(std::thread::spawn(move || loop {
            let message = rx.recv().unwrap();
            match message {
                Message::DoFlush(kv) => {
                    let file = std::fs::File::create(&filepath).unwrap();
                    let config = Default::default();
                    ron::ser::to_writer_pretty(file, &kv, config).unwrap();
                    tracing::trace!("Persisted to {:?}", filepath);
                }
                Message::Terminate => break,
            }
        }));
        Self {
            kv: read_ron(&ron_filepath).unwrap_or_default(),
            dirty: false,
            flush_handle,
            sender: tx,
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

            let kv = self.kv.clone();
            self.sender.send(Message::DoFlush(kv)).unwrap();
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

// ----------------------------------------------------------------------------

/// Alternative to [`FileStorage`]
pub fn read_memory(ctx: &egui::Context, memory_file_path: impl AsRef<std::path::Path>) {
    let memory: Option<egui::Memory> = read_ron(memory_file_path);
    if let Some(memory) = memory {
        *ctx.memory() = memory;
    }
}

/// Alternative to [`FileStorage`]
///
/// # Errors
/// When failing to serialize or create the file.
pub fn write_memory(
    ctx: &egui::Context,
    memory_file_path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(memory_file_path)?;
    let ron_config = Default::default();
    ron::ser::to_writer_pretty(file, &*ctx.memory(), ron_config)?;
    Ok(())
}
