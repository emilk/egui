use crate::diff_image_loader;
use crate::diff_image_loader::DiffOptions;
use std::path::PathBuf;
use eframe::egui::ImageSource;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub path: PathBuf,
    pub old: FileReference,
    pub new: FileReference,
    pub diff: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum FileReference {
    Path(PathBuf),
    Source(ImageSource<'static>),
}

impl FileReference {
    pub fn to_uri(&self) -> String {
        match self {
            FileReference::Path(path) => format!("file://{}", path.display()),
            FileReference::Source(source) => match source {
                ImageSource::Uri(uri) => uri.to_string(),
                ImageSource::Bytes { uri, .. } => uri.to_string(),
                _ => "unknown://unknown".to_string(),
            },
        }
    }
}


impl Snapshot {
    pub fn old_uri(&self) -> String {
        self.old.to_uri()
    }

    pub fn new_uri(&self) -> String {
        self.new.to_uri()
    }

    pub fn file_diff_uri(&self) -> Option<String> {
        self.diff
            .as_ref()
            .map(|p| format!("file://{}", p.display()))
    }

    pub fn diff_uri(&self, use_file_if_available: bool, options: DiffOptions) -> String {
        use_file_if_available
            .then(|| self.file_diff_uri())
            .flatten()
            .unwrap_or_else(|| {
                diff_image_loader::DiffUri {
                    old: self.old_uri(),
                    new: self.new_uri(),
                    options,
                }
                .to_uri()
            })
    }
}
