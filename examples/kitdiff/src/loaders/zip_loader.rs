use crate::snapshot::{FileReference, Snapshot};
use eframe::egui::{Context, ImageSource};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use zip::ZipArchive;

#[derive(Debug)]
pub enum ZipError {
    NetworkError(String),
    IoError(std::io::Error),
    ZipError(zip::result::ZipError),
    TempDirError,
}

impl From<std::io::Error> for ZipError {
    fn from(err: std::io::Error) -> Self {
        ZipError::IoError(err)
    }
}

impl From<zip::result::ZipError> for ZipError {
    fn from(err: zip::result::ZipError) -> Self {
        ZipError::ZipError(err)
    }
}

pub fn extract_and_discover_zip(
    zip_data: Vec<u8>,
    sender: mpsc::Sender<Snapshot>,
    ctx: Context,
) -> Result<(), ZipError> {
    if let Err(e) = run_zip_discovery(zip_data, sender, ctx) {
        eprintln!("Zip discovery error: {:?}", e);
        panic!("Zip discovery failed: {:?}", e);
    }
    Ok(())
}

fn run_zip_discovery(
    zip_data: Vec<u8>,
    sender: mpsc::Sender<Snapshot>,
    ctx: Context,
) -> Result<(), ZipError> {
    // Extract all files into memory (similar to tar loader)
    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor)?;

    let mut files: HashMap<PathBuf, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue, // Skip files with invalid names
        };

        // Only process PNG files
        if file_path.extension().and_then(|s| s.to_str()) == Some("png") {
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            files.insert(file_path, data);
        }
    }

    // Process the extracted files to create snapshots
    let mut processed_files = std::collections::HashSet::new();

    for (png_path, _) in &files {
        if processed_files.contains(png_path) {
            continue;
        }

        if let Some(snapshot) = try_create_zip_snapshot(png_path, &files) {
            // Mark related files as processed
            processed_files.insert(png_path.clone());
            if let Some(old_path) = get_variant_path(png_path, "old") {
                processed_files.insert(old_path);
            }
            if let Some(new_path) = get_variant_path(png_path, "new") {
                processed_files.insert(new_path);
            }
            if let Some(diff_path) = get_variant_path(png_path, "diff") {
                processed_files.insert(diff_path);
            }

            // Include bytes in egui context for loading
            match &snapshot.old {
                FileReference::Source(ImageSource::Bytes { uri, bytes }) => {
                    ctx.include_bytes(uri.clone(), bytes.clone());
                }
                _ => {}
            }
            match &snapshot.new {
                FileReference::Source(ImageSource::Bytes { uri, bytes }) => {
                    ctx.include_bytes(uri.clone(), bytes.clone());
                }
                _ => {}
            }

            if sender.send(snapshot).is_ok() {
                ctx.request_repaint();
            }
        }
    }

    Ok(())
}

fn try_create_zip_snapshot(png_path: &Path, files: &HashMap<PathBuf, Vec<u8>>) -> Option<Snapshot> {
    let file_name = png_path.file_name()?.to_str()?;

    // Skip files that are already variants (.old.png, .new.png, .diff.png)
    if file_name.ends_with(".old.png")
        || file_name.ends_with(".new.png")
        || file_name.ends_with(".diff.png")
    {
        return None;
    }

    // Get variant paths
    let old_path = get_variant_path(png_path, "old")?;
    let new_path = get_variant_path(png_path, "new")?;
    let diff_path = get_variant_path(png_path, "diff")?;

    // Check if diff exists (required for a valid snapshot)
    if !files.contains_key(&diff_path) {
        return None;
    }

    let base_data = files.get(png_path)?;

    if files.contains_key(&old_path) {
        // old.png exists, use original as new and old.png as old
        let old_data = files.get(&old_path)?;
        Some(Snapshot {
            path: png_path.to_path_buf(),
            old: FileReference::Source(ImageSource::Bytes {
                uri: Cow::Owned(format!("zip://{}", old_path.display())),
                bytes: eframe::egui::load::Bytes::Shared(old_data.clone().into()),
            }),
            new: FileReference::Source(ImageSource::Bytes {
                uri: Cow::Owned(format!("zip://{}", png_path.display())),
                bytes: eframe::egui::load::Bytes::Shared(base_data.clone().into()),
            }),
            diff: None, // We'll handle diff separately if needed
        })
    } else if files.contains_key(&new_path) {
        // new.png exists, use original as old and new.png as new
        let new_data = files.get(&new_path)?;
        Some(Snapshot {
            path: png_path.to_path_buf(),
            old: FileReference::Source(ImageSource::Bytes {
                uri: Cow::Owned(format!("zip://{}", png_path.display())),
                bytes: eframe::egui::load::Bytes::Shared(base_data.clone().into()),
            }),
            new: FileReference::Source(ImageSource::Bytes {
                uri: Cow::Owned(format!("zip://{}", new_path.display())),
                bytes: eframe::egui::load::Bytes::Shared(new_data.clone().into()),
            }),
            diff: None, // We'll handle diff separately if needed
        })
    } else {
        // No old or new variant, skip this snapshot
        None
    }
}

fn get_variant_path(base_path: &Path, variant: &str) -> Option<PathBuf> {
    let stem = base_path.file_stem()?.to_str()?;
    let parent = base_path.parent().unwrap_or(Path::new(""));
    Some(parent.join(format!("{}.{}.png", stem, variant)))
}
