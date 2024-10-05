use std::fmt::Display;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SnapshotError {
    Diff { diff: i32, diff_path: PathBuf },
    MissingSnapshot { path: PathBuf },
}

impl Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diff { diff, diff_path } => {
                write!(
                    f,
                    "Image did not match snapshot. Diff: {diff}, {diff_path:?}"
                )
            }
            Self::MissingSnapshot { path } => {
                write!(f, "Missing snapshot: {path:?}")
            }
        }
    }
}

/// Image snapshot test.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot.
pub fn try_image_snapshot(current: &image::RgbaImage, name: &str) -> Result<(), SnapshotError> {
    let snapshots_path = Path::new("tests/snapshots");

    let path = snapshots_path.join(format!("{name}.png"));
    std::fs::create_dir_all(path.parent().unwrap()).ok();

    let diff_path = snapshots_path.join(format!("{name}.diff.png"));
    let current_path = snapshots_path.join(format!("{name}.new.png"));

    current.save(&current_path).unwrap();

    let previous = match image::open(&path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            println!("Error opening image: {err}");
            println!("Saving current image as {path:?}");
            current.save(&path).unwrap();

            return Err(SnapshotError::MissingSnapshot { path });
        }
    };

    let result = dify::diff::get_results(previous, current.clone(), 0.1, true, None, &None, &None);

    if let Some((diff, result_image)) = result {
        result_image.save(diff_path.clone()).unwrap();

        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            current.save(&path).unwrap();
            println!("Updated snapshot: {path:?}");
        } else {
            return Err(SnapshotError::Diff { diff, diff_path });
        }
    } else {
        // Delete old diff if it exists
        std::fs::remove_file(diff_path).ok();
    }

    Ok(())
}

/// Image snapshot test.
///
/// # Panics
/// Panics if the image does not match the snapshot.
#[track_caller]
pub fn image_snapshot(current: &image::RgbaImage, name: &str) {
    match try_image_snapshot(current, name) {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
}
