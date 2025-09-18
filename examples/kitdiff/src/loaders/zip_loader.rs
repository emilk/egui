use crate::snapshot::Snapshot;
use eframe::egui::Context;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::mpsc;
use tempfile::TempDir;
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
    zip_source: String,
    temp_dir: &TempDir,
    sender: mpsc::Sender<Snapshot>,
    ctx: Context,
) -> Result<(), ZipError> {
    let extract_path = temp_dir.path().to_path_buf();
    std::thread::spawn(move || {
        if let Err(e) = run_zip_extraction_and_discovery(zip_source, &extract_path, sender, ctx) {
            eprintln!("Zip discovery error: {:?}", e);
        }
    });
    Ok(())
}

fn run_zip_extraction_and_discovery(
    zip_source: String,
    extract_path: &Path,
    sender: mpsc::Sender<Snapshot>,
    ctx: Context,
) -> Result<(), ZipError> {
    // // Download or read the zip file
    // let zip_data = if zip_source.starts_with("http://") || zip_source.starts_with("https://") {
    //     download_zip(&zip_source)?
    // } else {
    //     // Read from filesystem
    //     std::fs::read(&zip_source)?
    // };
    //
    // // Extract zip to temp directory
    // extract_zip(zip_data, extract_path)?;
    //
    // // Run file discovery on the extracted directory
    // crate::native_loaders::file_diff::file_discovery(extract_path, sender, ctx);

    todo!();

    Ok(())
}

fn download_zip(url: &str) -> Result<Vec<u8>, ZipError> {
    // let request = ehttp::Request::get(url);
    // let response =
    //     ehttp::fetch_blocking(&request).map_err(|e| ZipError::NetworkError(e.to_string()))?;
    //
    // if !response.ok {
    //     return Err(ZipError::NetworkError(format!(
    //         "HTTP {}: {}",
    //         response.status, response.status_text
    //     )));
    // }
    //
    // Ok(response.bytes)
    todo!()
}

fn extract_zip(zip_data: Vec<u8>, extract_path: &Path) -> Result<(), ZipError> {
    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_path.join(path),
            None => continue, // Skip files with invalid names
        };

        if file.name().ends_with('/') {
            // Directory
            std::fs::create_dir_all(&outpath)?;
        } else {
            // File
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Set permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}
