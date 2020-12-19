use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

// ----------------------------------------------------------------------------

/// A key-value store backed by a JSON file on disk.
/// Used to restore egui state, glium window position/size and app state.
pub struct FileStorage {
    path: PathBuf,
    kv: HashMap<String, String>,
    dirty: bool,
}

impl FileStorage {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path: PathBuf = path.into();
        Self {
            kv: read_json(&path).unwrap_or_default(),
            path,
            dirty: false,
        }
    }
}

impl egui::app::Storage for FileStorage {
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
            serde_json::to_writer(std::fs::File::create(&self.path).unwrap(), &self.kv).unwrap();
            self.dirty = false;
        }
    }
}

// ----------------------------------------------------------------------------

pub fn read_json<T>(memory_json_path: impl AsRef<Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    match std::fs::File::open(memory_json_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    eprintln!("ERROR: Failed to parse json: {}", err);
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

/// Alternative to `FileStorage`
pub fn read_memory(ctx: &egui::Context, memory_json_path: impl AsRef<std::path::Path>) {
    let memory: Option<egui::Memory> = read_json(memory_json_path);
    if let Some(memory) = memory {
        *ctx.memory() = memory;
    }
}

/// Alternative to `FileStorage`
pub fn write_memory(
    ctx: &egui::Context,
    memory_json_path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(std::fs::File::create(memory_json_path)?, &*ctx.memory())?;
    Ok(())
}

// ----------------------------------------------------------------------------

use glium::glutin;

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WindowSettings {
    /// outer position of window in physical pixels
    pos: Option<egui::Pos2>,
    /// Inner size of window in logical pixels
    inner_size_points: Option<egui::Vec2>,
}

impl WindowSettings {
    pub fn from_json_file(
        settings_json_path: impl AsRef<std::path::Path>,
    ) -> Option<WindowSettings> {
        read_json(settings_json_path)
    }

    pub fn from_display(display: &glium::Display) -> Self {
        let scale_factor = display.gl_window().window().scale_factor();
        let inner_size_points = display
            .gl_window()
            .window()
            .inner_size()
            .to_logical::<f32>(scale_factor);

        Self {
            pos: display
                .gl_window()
                .window()
                .outer_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32)),

            inner_size_points: Some(egui::vec2(
                inner_size_points.width as f32,
                inner_size_points.height as f32,
            )),
        }
    }

    pub fn initialize_size(
        &self,
        window: glutin::window::WindowBuilder,
    ) -> glutin::window::WindowBuilder {
        if let Some(inner_size_points) = self.inner_size_points {
            window.with_inner_size(glutin::dpi::LogicalSize {
                width: inner_size_points.x as f64,
                height: inner_size_points.y as f64,
            })
        } else {
            window
        }

        // Not yet available in winit: https://github.com/rust-windowing/winit/issues/1190
        // if let Some(pos) = self.pos {
        //     *window = window.with_outer_pos(glutin::dpi::PhysicalPosition {
        //         x: pos.x as f64,
        //         y: pos.y as f64,
        //     });
        // }
    }

    pub fn restore_positions(&self, display: &glium::Display) {
        // not needed, done by `initialize_size`
        // let size = self.size.unwrap_or_else(|| vec2(1024.0, 800.0));
        // display
        //     .gl_window()
        //     .window()
        //     .set_inner_size(glutin::dpi::PhysicalSize {
        //         width: size.x as f64,
        //         height: size.y as f64,
        //     });

        if let Some(pos) = self.pos {
            display
                .gl_window()
                .window()
                .set_outer_position(glutin::dpi::PhysicalPosition::new(
                    pos.x as f64,
                    pos.y as f64,
                ));
        }
    }
}
