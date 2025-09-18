use crate::snapshot::{FileReference, Snapshot};
use eframe::egui::Context;
use ignore::WalkBuilder;
use ignore::types::TypesBuilder;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub fn file_discovery(
    base_path: impl Into<PathBuf>,
    sender: mpsc::Sender<Snapshot>,
    ctx: Context,
) {
    let path = base_path.into();

    std::thread::spawn(move || {
        // Create type matcher for .png files
        let mut types_builder = TypesBuilder::new();
        types_builder.add("png", "*.png").unwrap();
        types_builder.select("png");
        let types = types_builder.build().unwrap();

        // Build sequential walker for .png files only to maintain order
        for result in WalkBuilder::new(&path).types(types).build() {
            if let Ok(entry) = result {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    if let Some(snapshot) = try_create_snapshot(entry.path(), &path) {
                        if sender.send(snapshot).is_ok() {
                            ctx.request_repaint();
                        }
                    }
                }
            }
        }
    });
}

fn try_create_snapshot(png_path: &Path, base_path: &Path) -> Option<Snapshot> {
    let file_name = png_path.file_name()?.to_str()?;

    // Skip files that are already variants (.old.png, .new.png, .diff.png)
    if file_name.ends_with(".old.png")
        || file_name.ends_with(".new.png")
        || file_name.ends_with(".diff.png")
    {
        return None;
    }

    // Get base path without .png extension
    let file_base_path = png_path.with_extension("");
    let old_path = file_base_path.with_extension("old.png");
    let new_path = file_base_path.with_extension("new.png");
    let diff_path = file_base_path.with_extension("diff.png");

    // Only create snapshot if diff exists
    if !diff_path.exists() {
        return None;
    }

    // Create relative path from the base directory
    let relative_path = png_path.strip_prefix(base_path).unwrap_or(png_path);

    if old_path.exists() {
        // old.png exists, use original as new and old.png as old
        Some(Snapshot {
            path: relative_path.to_path_buf(),
            old: FileReference::Path(old_path),
            new: FileReference::Path(png_path.to_path_buf()),
            diff: Some(diff_path),
        })
    } else if new_path.exists() {
        // new.png exists, use original as old and new.png as new
        Some(Snapshot {
            path: relative_path.to_path_buf(),
            old: FileReference::Path(png_path.to_path_buf()),
            new: FileReference::Path(new_path),
            diff: Some(diff_path),
        })
    } else {
        // No old or new variant, skip this snapshot
        None
    }
}
