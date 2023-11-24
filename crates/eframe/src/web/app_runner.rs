use egui::TexturesDelta;

use crate::{epi, App};

use super::{now_sec, web_painter::WebPainter, NeedRepaint};

pub struct AppRunner {
    web_options: crate::WebOptions,
    pub(crate) frame: epi::Frame,
    egui_ctx: egui::Context,
    painter: super::ActiveWebPainter,
    pub(crate) input: super::WebInput,
    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: std::sync::Arc<NeedRepaint>,
    last_save_time: f64,
    screen_reader: super::screen_reader::ScreenReader,
    pub(crate) text_cursor_pos: Option<egui::Pos2>,
    pub(crate) mutable_text_under_cursor: bool,

    // Output for the last run:
    textures_delta: TexturesDelta,
    clipped_primitives: Option<Vec<egui::ClippedPrimitive>>,
}

impl Drop for AppRunner {
    fn drop(&mut self) {
        log::debug!("AppRunner has fully dropped");
    }
}

impl AppRunner {
    /// # Errors
    /// Failure to initialize WebGL renderer.
    pub async fn new(
        canvas_id: &str,
        web_options: crate::WebOptions,
        app_creator: epi::AppCreator,
    ) -> Result<Self, String> {
        let painter = super::ActiveWebPainter::new(canvas_id, &web_options).await?;

        let system_theme = if web_options.follow_system_theme {
            super::system_theme()
        } else {
            None
        };

        let info = epi::IntegrationInfo {
            web_info: epi::WebInfo {
                user_agent: super::user_agent().unwrap_or_default(),
                location: super::web_location(),
            },
            system_theme,
            cpu_usage: None,
        };
        let storage = LocalStorage::default();

        let egui_ctx = egui::Context::default();
        egui_ctx.set_os(egui::os::OperatingSystem::from_user_agent(
            &super::user_agent().unwrap_or_default(),
        ));
        super::storage::load_memory(&egui_ctx);

        egui_ctx.options_mut(|o| {
            // On web, the browser controls the zoom factor:
            o.zoom_with_keyboard = false;
            o.zoom_factor = 1.0;
        });

        let theme = system_theme.unwrap_or(web_options.default_theme);
        egui_ctx.set_visuals(theme.egui_visuals());

        let app = app_creator(&epi::CreationContext {
            egui_ctx: egui_ctx.clone(),
            integration_info: info.clone(),
            storage: Some(&storage),

            #[cfg(feature = "glow")]
            gl: Some(painter.gl().clone()),

            #[cfg(all(feature = "wgpu", not(feature = "glow")))]
            wgpu_render_state: painter.render_state(),
            #[cfg(all(feature = "wgpu", feature = "glow"))]
            wgpu_render_state: None,
        });

        let frame = epi::Frame {
            info,
            storage: Some(Box::new(storage)),

            #[cfg(feature = "glow")]
            gl: Some(painter.gl().clone()),

            #[cfg(all(feature = "wgpu", not(feature = "glow")))]
            wgpu_render_state: painter.render_state(),
            #[cfg(all(feature = "wgpu", feature = "glow"))]
            wgpu_render_state: None,
        };

        let needs_repaint: std::sync::Arc<NeedRepaint> = Default::default();
        {
            let needs_repaint = needs_repaint.clone();
            egui_ctx.set_request_repaint_callback(move |info| {
                needs_repaint.repaint_after(info.delay.as_secs_f64());
            });
        }

        let mut runner = Self {
            web_options,
            frame,
            egui_ctx,
            painter,
            input: Default::default(),
            app,
            needs_repaint,
            last_save_time: now_sec(),
            screen_reader: Default::default(),
            text_cursor_pos: None,
            mutable_text_under_cursor: false,
            textures_delta: Default::default(),
            clipped_primitives: None,
        };

        runner.input.raw.max_texture_side = Some(runner.painter.max_texture_side());
        runner
            .input
            .raw
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(super::native_pixels_per_point());

        Ok(runner)
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Get mutable access to the concrete [`App`] we enclose.
    ///
    /// This will panic if your app does not implement [`App::as_any_mut`].
    pub fn app_mut<ConcreteApp: 'static + App>(&mut self) -> &mut ConcreteApp {
        self.app
            .as_any_mut()
            .expect("Your app must implement `as_any_mut`, but it doesn't")
            .downcast_mut::<ConcreteApp>()
            .expect("app_mut got the wrong type of App")
    }

    pub fn auto_save_if_needed(&mut self) {
        let time_since_last_save = now_sec() - self.last_save_time;
        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            self.save();
        }
    }

    pub fn save(&mut self) {
        if self.app.persist_egui_memory() {
            super::storage::save_memory(&self.egui_ctx);
        }
        if let Some(storage) = self.frame.storage_mut() {
            self.app.save(storage);
        }
        self.last_save_time = now_sec();
    }

    pub fn canvas_id(&self) -> &str {
        self.painter.canvas_id()
    }

    pub fn destroy(mut self) {
        log::debug!("Destroying AppRunner");
        self.painter.destroy();
    }

    /// Runs the user code and paints the UI.
    ///
    /// If there is already an outstanding frame of output,
    /// that is painted instead.
    pub fn run_and_paint(&mut self) {
        if self.clipped_primitives.is_none() {
            // Run user code, and paint the results:
            self.logic();
            self.paint();
        } else {
            // We have already run the logic, e.g. in an on-click event,
            // so let's only present the results:
            self.paint();
        }
    }

    /// Runs the logic, but doesn't paint the result.
    ///
    /// The result can be painted later with a call to [`Self::run_and_paint`] or [`Self::paint`].
    pub fn logic(&mut self) {
        let frame_start = now_sec();

        super::resize_canvas_to_screen_size(self.canvas_id(), self.web_options.max_size_points);
        let canvas_size = super::canvas_size_in_points(self.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);

        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            self.app.update(egui_ctx, &mut self.frame);
        });
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = full_output;

        if viewport_output.len() > 1 {
            log::warn!("Multiple viewports not yet supported on the web");
        }
        for viewport_output in viewport_output.values() {
            for command in &viewport_output.commands {
                // TODO(emilk): handle some of the commands
                log::warn!(
                    "Unhandled egui viewport command: {command:?} - not implemented in web backend"
                );
            }
        }

        self.handle_platform_output(platform_output);
        self.textures_delta.append(textures_delta);
        self.clipped_primitives = Some(self.egui_ctx.tessellate(shapes, pixels_per_point));

        self.frame.info.cpu_usage = Some((now_sec() - frame_start) as f32);
    }

    /// Paint the results of the last call to [`Self::logic`].
    pub fn paint(&mut self) {
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_primitives = std::mem::take(&mut self.clipped_primitives);

        if let Some(clipped_primitives) = clipped_primitives {
            if let Err(err) = self.painter.paint_and_update_textures(
                self.app.clear_color(&self.egui_ctx.style().visuals),
                &clipped_primitives,
                self.egui_ctx.pixels_per_point(),
                &textures_delta,
            ) {
                log::error!("Failed to paint: {}", super::string_from_js_value(&err));
            }
        }
    }

    fn handle_platform_output(&mut self, platform_output: egui::PlatformOutput) {
        if self.egui_ctx.options(|o| o.screen_reader) {
            self.screen_reader
                .speak(&platform_output.events_description());
        }

        let egui::PlatformOutput {
            cursor_icon,
            open_url,
            copied_text,
            events: _, // already handled
            mutable_text_under_cursor,
            text_cursor_pos,
            #[cfg(feature = "accesskit")]
                accesskit_update: _, // not currently implemented
        } = platform_output;

        super::set_cursor_icon(cursor_icon);
        if let Some(open) = open_url {
            super::open_url(&open.url, open.new_tab);
        }

        #[cfg(web_sys_unstable_apis)]
        if !copied_text.is_empty() {
            super::set_clipboard_text(&copied_text);
        }

        #[cfg(not(web_sys_unstable_apis))]
        let _ = copied_text;

        self.mutable_text_under_cursor = mutable_text_under_cursor;

        if self.text_cursor_pos != text_cursor_pos {
            super::text_agent::move_text_cursor(text_cursor_pos, self.canvas_id());
            self.text_cursor_pos = text_cursor_pos;
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct LocalStorage {}

impl epi::Storage for LocalStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        super::storage::local_storage_get(key)
    }

    fn set_string(&mut self, key: &str, value: String) {
        super::storage::local_storage_set(key, &value);
    }

    fn flush(&mut self) {}
}
