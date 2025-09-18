use crate::snapshot::Snapshot;
use eframe::egui::Context;
use eframe::egui::load::Bytes;
use std::any::Any;
use std::sync::mpsc::Sender;

pub mod app;
pub mod diff_image_loader;
pub mod loaders;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_loaders;
pub mod snapshot;

#[derive(Debug, Clone)]
pub enum DiffSource {
    #[cfg(not(target_arch = "wasm32"))]
    Files,
    #[cfg(not(target_arch = "wasm32"))]
    Git,
    #[cfg(not(target_arch = "wasm32"))]
    Pr(String), // Store the PR URL
    Zip(PathOrBlob),   // Store the zip source (URL or file path)
    TarGz(PathOrBlob), // Tar.gz files loaded via drag and drop
}

impl DiffSource {
    pub fn load(self, tx: Sender<Snapshot>, ctx: Context) -> Option<DropMeLater> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            DiffSource::Files => {
                native_loaders::file_diff::file_discovery(".", tx, ctx);
                None
            }
            #[cfg(not(target_arch = "wasm32"))]
            DiffSource::Git => {
                native_loaders::git_loader::git_discovery(tx, ctx)
                    .expect("Failed to run git discovery");
                None
            }
            #[cfg(not(target_arch = "wasm32"))]
            DiffSource::Pr(url) => {
                native_loaders::git_loader::pr_git_discovery(url, tx, ctx)
                    .expect("Failed to run PR git discovery");
                None
            }
            DiffSource::Zip(data) => {
                // loaders::tar_loader::extract_and_discover_tar_gz(
                //     data.load_bytes()?.to_vec(),
                //     tx,
                //     ctx,
                // )
                // .expect("Failed to run zip discovery");
                // None
                todo!()
            }
            DiffSource::TarGz(data) => {
                loaders::tar_loader::extract_and_discover_tar_gz(
                    data.load_bytes()?.to_vec(),
                    tx,
                    ctx,
                )
                .expect("Failed to run tar.gz discovery");
                None
            }
        }
    }
}

struct DropMeLater(Box<dyn Any>);

#[derive(Debug, Clone)]
pub enum PathOrBlob {
    Path(std::path::PathBuf),
    Blob(Bytes),
}

impl PathOrBlob {
    pub fn load_bytes(&self) -> Option<Bytes> {
        match self {
            PathOrBlob::Path(path) => std::fs::read(path).ok().map(Bytes::from),
            PathOrBlob::Blob(bytes) => Some(bytes.clone()),
        }
    }
}
