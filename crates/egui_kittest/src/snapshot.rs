use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SnapshotError {
    pub diff: i32,
    pub diff_path: PathBuf,
}

/// Image snapshot test.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot.
pub fn try_image_snapshot(current: image::RgbaImage, name: &str) -> Result<(), SnapshotError> {
    let current =
        dify_image::RgbaImage::from_raw(current.width(), current.height(), current.into_raw())
            .unwrap();

    let snapshots_path = Path::new("tests/snapshots");

    let path = snapshots_path.join(format!("{name}.png"));
    std::fs::create_dir_all(path.parent().unwrap()).ok();

    let diff_path = snapshots_path.join(format!("{name}.diff.png"));
    let current_path = snapshots_path.join(format!("{name}.new.png"));

    std::fs::create_dir_all("tests/snapshots").ok();

    current.save(&current_path).unwrap();

    let previous = match dify_image::open(&path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            println!("Error opening image: {err}");
            println!("Saving current image as {path:?}");
            current.save(&path).unwrap();

            current.clone()
        }
    };

    let result = dify::diff::get_results(previous, current.clone(), 0.1, true, None, &None, &None);

    if let Some((diff, result_image)) = result {
        result_image.save(diff_path.clone()).unwrap();

        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            current.save(&path).unwrap();
            println!("Updated snapshot: {path:?}");
        } else {
            return Err(SnapshotError { diff, diff_path });
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
pub fn image_snapshot(current: image::RgbaImage, name: &str) {
    match try_image_snapshot(current, name) {
        Ok(_) => {}
        Err(err) => {
            panic!(
                "{name} failed. Image did not match snapshot. Diff: {}, {:?}",
                err.diff, err.diff_path
            );
        }
    }
}
