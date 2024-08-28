use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

/// The folder where `eframe` will store its state.
///
/// The given `app_id` is either the
/// [`egui::ViewportBuilder::app_id`] of [`crate::NativeOptions::viewport`]
/// or the title argument to [`crate::run_native`].
///
/// On native, the path is:
/// * Linux:   `/home/UserName/.local/share/APP_ID`
/// * macOS:   `/Users/UserName/Library/Application Support/APP_ID`
/// * Windows: `C:\Users\UserName\AppData\Roaming\APP_ID\data`
pub fn storage_dir(app_id: &str) -> Option<PathBuf> {
    use egui::os::OperatingSystem as OS;
    use std::env::var_os;
    match OS::from_target_os() {
        OS::Nix => var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home::home_dir().map(|p| p.join(".local").join("share")))
            .map(|p| {
                p.join(
                    app_id
                        .to_lowercase()
                        .replace(|c: char| c.is_ascii_whitespace(), ""),
                )
            }),
        OS::Mac => home::home_dir().map(|p| {
            p.join("Library")
                .join("Application Support")
                .join(app_id.replace(|c: char| c.is_ascii_whitespace(), "-"))
        }),
        OS::Windows => roaming_appdata().map(|p| p.join(app_id).join("data")),
        OS::Unknown | OS::Android | OS::IOS => None,
    }
}

// Adapted from
// https://github.com/rust-lang/cargo/blob/6e11c77384989726bb4f412a0e23b59c27222c34/crates/home/src/windows.rs#L19-L37
#[cfg(all(windows, not(target_vendor = "uwp")))]
#[allow(unsafe_code)]
fn roaming_appdata() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    use std::slice;

    use windows_sys::Win32::Foundation::S_OK;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{
        FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_DONT_VERIFY,
    };

    extern "C" {
        fn wcslen(buf: *const u16) -> usize;
    }
    unsafe {
        let mut path = ptr::null_mut();
        match SHGetKnownFolderPath(
            &FOLDERID_RoamingAppData,
            KF_FLAG_DONT_VERIFY as u32,
            0,
            &mut path,
        ) {
            S_OK => {
                let path_slice = slice::from_raw_parts(path, wcslen(path));
                let s = OsString::from_wide(&path_slice);
                CoTaskMemFree(path.cast());
                Some(PathBuf::from(s))
            }
            _ => {
                // Free any allocated memory even on failure. A null ptr is a no-op for `CoTaskMemFree`.
                CoTaskMemFree(path.cast());
                None
            }
        }
    }
}

#[cfg(any(not(windows), target_vendor = "uwp"))]
fn roaming_appdata() -> Option<PathBuf> {
    None
}

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
            crate::profile_scope!("wait_for_save");
            join_handle.join().ok();
        }
    }
}

impl FileStorage {
    /// Store the state in this .ron file.
    pub(crate) fn from_ron_filepath(ron_filepath: impl Into<PathBuf>) -> Self {
        crate::profile_function!();
        let ron_filepath: PathBuf = ron_filepath.into();
        log::debug!("Loading app state from {:?}…", ron_filepath);
        Self {
            kv: read_ron(&ron_filepath).unwrap_or_default(),
            ron_filepath,
            dirty: false,
            last_save_join_handle: None,
        }
    }

    /// Find a good place to put the files that the OS likes.
    pub fn from_app_id(app_id: &str) -> Option<Self> {
        crate::profile_function!(app_id);
        if let Some(data_dir) = storage_dir(app_id) {
            if let Err(err) = std::fs::create_dir_all(&data_dir) {
                log::warn!(
                    "Saving disabled: Failed to create app path at {:?}: {}",
                    data_dir,
                    err
                );
                None
            } else {
                Some(Self::from_ron_filepath(data_dir.join("app.ron")))
            }
        } else {
            log::warn!("Saving disabled: Failed to find path to data_dir.");
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
            crate::profile_function!();
            self.dirty = false;

            let file_path = self.ron_filepath.clone();
            let kv = self.kv.clone();

            if let Some(join_handle) = self.last_save_join_handle.take() {
                // wait for previous save to complete.
                join_handle.join().ok();
            }

            let result = std::thread::Builder::new()
                .name("eframe_persist".to_owned())
                .spawn(move || {
                    save_to_disk(&file_path, &kv);
                });
            match result {
                Ok(join_handle) => {
                    self.last_save_join_handle = Some(join_handle);
                }
                Err(err) => {
                    log::warn!("Failed to spawn thread to save app state: {err}");
                }
            }
        }
    }
}

fn save_to_disk(file_path: &PathBuf, kv: &HashMap<String, String>) {
    crate::profile_function!();

    if let Some(parent_dir) = file_path.parent() {
        if !parent_dir.exists() {
            if let Err(err) = std::fs::create_dir_all(parent_dir) {
                log::warn!("Failed to create directory {parent_dir:?}: {err}");
            }
        }
    }

    match std::fs::File::create(file_path) {
        Ok(file) => {
            let mut writer = std::io::BufWriter::new(file);
            let config = Default::default();

            crate::profile_scope!("ron::serialize");
            if let Err(err) = ron::ser::to_writer_pretty(&mut writer, &kv, config)
                .and_then(|_| writer.flush().map_err(|err| err.into()))
            {
                log::warn!("Failed to serialize app state: {}", err);
            } else {
                log::trace!("Persisted to {:?}", file_path);
            }
        }
        Err(err) => {
            log::warn!("Failed to create file {file_path:?}: {err}");
        }
    }
}

// ----------------------------------------------------------------------------

fn read_ron<T>(ron_path: impl AsRef<Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    crate::profile_function!();
    match std::fs::File::open(ron_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match ron::de::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    log::warn!("Failed to parse RON: {}", err);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn directories_storage_dir(app_id: &str) -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", app_id)
            .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
    }

    #[test]
    fn storage_path_matches_directories() {
        use super::storage_dir;
        for app_id in [
            "MyApp", "My App", "my_app", "my-app", "My.App", "my/app", "my:app", r"my\app",
        ] {
            assert_eq!(directories_storage_dir(app_id), storage_dir(app_id));
        }
    }
}
