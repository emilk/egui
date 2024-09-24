pub fn image_snapshot(current: image::RgbaImage, name: &str) {
    let current =
        dify_image::RgbaImage::from_raw(current.width(), current.height(), current.into_raw())
            .unwrap();

    let path = format!("tests/snapshots/{name}.png");
    let diff_path = format!("tests/snapshots/{name}.diff.webp");
    let current_path = format!("tests/snapshots/{name}.new.webp");

    std::fs::create_dir_all("tests/snapshots").ok();

    current.save(&current_path).unwrap();

    let previous = match dify_image::open(&path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            println!("Error opening image: {err}");
            println!("Saving current image as {path}");
            current.save(&path).unwrap();

            current.clone()
        }
    };

    let result = dify::diff::get_results(previous, current.clone(), 0.1, true, None, &None, &None);

    if let Some((diff, result_image)) = result {
        result_image.save(diff_path.clone()).unwrap();

        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            current.save(&path).unwrap();
            println!("Updated snapshot: {path}");
        } else {
            panic!(
                "Image did not match snapshot. Diff: {diff}, {diff_path}"
            );
        }
    } else {
        // Delete old diff if it exists
        std::fs::remove_file(diff_path).ok();
    }
}
