use std::sync::Arc;

use egui::{ScreenshotCallback, TexturesDelta, ViewportCommand};

use crate::{App, epi, web::web_painter::WebPainter, web_events::WebEventState};

use super::{NeedRepaint, WebCanvas, WebHost, now_sec, text_agent::TextAgent};

pub struct AppRunner {
    #[allow(clippy::allow_attributes, dead_code)]
    pub(crate) web_options: crate::WebOptions,
    pub(crate) frame: epi::Frame,
    egui_ctx: egui::Context,
    painter: Box<dyn WebPainter>,

    /// The canvas we render to: DOM (`Html`) or worker-owned (`Offscreen`).
    canvas: WebCanvas,

    /// The channel back to the host page. `None` in DOM mode.
    host: Option<WebHost>,

    pub(crate) input: super::WebInput,

    /// Pointer/wheel state for messages from the host page (worker mode only).
    pub(crate) offscreen_events: WebEventState,

    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: Arc<NeedRepaint>,
    last_save_time: f64,

    /// DOM-only hidden `<input>` used for IME. `None` in worker mode.
    pub(crate) text_agent: Option<TextAgent>,

    // If not empty, the painter should capture n frames from now.
    // zero means capture the exact next frame.
    screenshot_commands_with_frame_delay: Vec<(ScreenshotCallback, usize)>,

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
    /// Failure to initialize WebGL renderer, or failure to create app.
    #[cfg_attr(
        not(feature = "wgpu_no_default_features"),
        expect(clippy::unused_async)
    )]
    pub(crate) async fn new(
        canvas: WebCanvas,
        web_options: crate::WebOptions,
        app_creator: epi::AppCreator<'static>,
        host: Option<WebHost>,
        text_agent: Option<TextAgent>,
    ) -> Result<Self, String> {
        let egui_ctx = egui::Context::default();
        egui_ctx.add_glyph_rasterizer(super::canvas_glyphs::glyph_rasterizer());

        #[allow(clippy::allow_attributes, unused_assignments)]
        #[cfg(feature = "glow")]
        let mut gl = None;

        #[allow(clippy::allow_attributes, unused_assignments)]
        #[cfg(feature = "wgpu_no_default_features")]
        let mut wgpu_render_state = None;

        let painter = match web_options.renderer {
            #[cfg(feature = "glow")]
            epi::Renderer::Glow => {
                log::debug!("Using the glow renderer");
                let painter = super::web_painter_glow::WebPainterGlow::new(
                    egui_ctx.clone(),
                    canvas.clone(),
                    &web_options,
                )?;
                gl = Some(Arc::clone(painter.gl()));
                Box::new(painter) as Box<dyn WebPainter>
            }

            #[cfg(feature = "wgpu_no_default_features")]
            epi::Renderer::Wgpu => {
                log::debug!("Using the wgpu renderer");
                let painter = super::web_painter_wgpu::WebPainterWgpu::new(
                    egui_ctx.clone(),
                    canvas.clone(),
                    &web_options,
                )
                .await?;
                wgpu_render_state = painter.render_state();
                Box::new(painter) as Box<dyn WebPainter>
            }
        };

        // `user_agent` and `web_location` read from `window`, which a worker
        // does not have.
        let user_agent = super::user_agent().unwrap_or_default();
        let location = if web_sys::window().is_some() {
            super::web_location()
        } else {
            Default::default()
        };

        egui_ctx.set_os(egui::os::OperatingSystem::from_user_agent(&user_agent));

        let info = epi::IntegrationInfo {
            web_info: epi::WebInfo {
                user_agent,
                location,
                offscreen_canvas: canvas.as_offscreen().is_some(),
            },
            cpu_usage: None,
        };
        let storage = LocalStorage::default();

        super::storage::load_memory(&egui_ctx);

        egui_ctx.options_mut(|o| {
            // On web by default egui follows the zoom factor of the browser,
            // and lets the browser handle the zoom shortcuts.
            // A user can still zoom egui separately by calling [`egui::Context::set_zoom_factor`].
            o.zoom_with_keyboard = false;
            o.zoom_factor = 1.0;
        });

        // Tell egui right away about native_pixels_per_point
        // so that the app knows about it during app creation:
        egui_ctx.input_mut(|i| {
            let viewport_info = i.raw.viewports.entry(egui::ViewportId::ROOT).or_default();
            viewport_info.native_pixels_per_point = Some(super::native_pixels_per_point());
            i.pixels_per_point = super::native_pixels_per_point();
        });

        let cc = epi::CreationContext {
            egui_ctx: egui_ctx.clone(),
            integration_info: info.clone(),
            storage: Some(&storage),

            #[cfg(feature = "glow")]
            gl: gl.clone(),

            #[cfg(feature = "glow")]
            get_proc_address: None,

            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_render_state: wgpu_render_state.clone(),
        };
        let app = app_creator(&cc).map_err(|err| err.to_string())?;

        let frame = epi::Frame {
            info,
            storage: Some(Box::new(storage)),

            #[cfg(feature = "glow")]
            gl,

            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_render_state,
        };

        let needs_repaint: Arc<NeedRepaint> = Arc::new(NeedRepaint::new(web_options.max_fps));
        {
            let needs_repaint = Arc::clone(&needs_repaint);
            egui_ctx.set_request_repaint_callback(move |info| {
                needs_repaint.repaint_after(info.delay.as_secs_f64());
            });
        }

        let mut runner = Self {
            web_options,
            frame,
            egui_ctx,
            painter,
            canvas,
            host,
            input: Default::default(),
            offscreen_events: Default::default(),
            app,
            needs_repaint,
            last_save_time: now_sec(),
            text_agent,
            screenshot_commands_with_frame_delay: vec![],
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
        runner.input.raw.system_theme = super::system_theme();

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

    pub fn html_canvas(&self) -> &web_sys::HtmlCanvasElement {
        self.canvas.expect_html()
    }

    pub fn canvas(&self) -> &web_sys::HtmlCanvasElement {
        self.html_canvas()
    }

    pub fn destroy(mut self) {
        log::debug!("Destroying AppRunner");
        self.egui_ctx.on_exit();
        self.painter.destroy();
    }

    pub fn has_outstanding_paint_data(&self) -> bool {
        self.clipped_primitives.is_some()
    }

    /// Does the eframe app have focus?
    ///
    /// Technically: does either the canvas or the [`TextAgent`] have focus?
    pub fn has_focus(&self) -> bool {
        if self.host.is_some() {
            return self.input.focused;
        }

        let focused = super::has_focus(self.html_canvas());

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        if document.hidden() {
            return false;
        }

        focused || self.text_agent.as_ref().is_some_and(|t| t.has_focus())
    }

    /// Called from the host page's `resize` message.
    pub(crate) fn on_offscreen_resize(&self, width: u32, height: u32) {
        self.canvas.set_size(width, height);
        self.needs_repaint.repaint_asap();
    }

    pub fn update_focus(&mut self) {
        let has_focus = self.has_focus();
        if self.input.raw.focused != has_focus {
            log::trace!("{} Focus changed to {has_focus}", self.canvas.id());
            self.input.set_focus(has_focus);

            if !has_focus {
                // We lost focus - good idea to save
                self.save();
            }
            self.egui_ctx().request_repaint();
        }
    }

    /// Runs the logic, but doesn't paint the result.
    ///
    /// The result can be painted later with a call to [`Self::run_and_paint`] or [`Self::paint`].
    pub fn logic(&mut self) {
        // We sometimes miss blur/focus events due to the text agent, so let's just poll each frame:
        self.update_focus();

        let canvas_size = super::canvas_size_in_points(&self.canvas, self.egui_ctx());
        let mut raw_input = self.input.new_frame(canvas_size);

        if super::DEBUG_RESIZE {
            log::info!(
                "egui running at canvas size: {}x{}, DPR: {}, zoom_factor: {}. egui size: {}x{} points",
                self.canvas.width(),
                self.canvas.height(),
                super::native_pixels_per_point(),
                self.egui_ctx.zoom_factor(),
                canvas_size.x,
                canvas_size.y,
            );
        }

        self.app.raw_input_hook(&self.egui_ctx, &mut raw_input);

        let is_visible = raw_input
            .viewports
            .get(&egui::ViewportId::ROOT)
            .and_then(|v| v.visible())
            .unwrap_or(true);

        if is_visible {
            let full_output = self.egui_ctx.run_ui(raw_input, |ui| {
                self.app.logic(ui.ctx(), &mut self.frame);
                self.app.ui(ui, &mut self.frame);
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
            self.handle_viewport_commands(
                viewport_output
                    .into_values()
                    .flat_map(|viewport_output| viewport_output.commands),
            );

            self.handle_platform_output(platform_output);
            self.textures_delta.append(textures_delta);
            self.clipped_primitives = Some(self.egui_ctx.tessellate(shapes, pixels_per_point));
        } else {
            // The tab is hidden, so we run no egui pass at all.
            // That way all ui state is left untouched, and is still there
            // when the tab is shown again.

            let egui::LogicOutput {
                platform_output,
                viewport_commands,
            } = self.egui_ctx.run_logic(&raw_input, |ctx| {
                self.app.logic(ctx, &mut self.frame);
            });

            // No pass consumed the input, so save it for the next one:
            self.input.raw.append(raw_input);

            self.handle_viewport_commands(viewport_commands.into_values().flatten());
            self.handle_platform_output(platform_output);
        }
    }

    fn handle_viewport_commands(&mut self, commands: impl Iterator<Item = ViewportCommand>) {
        for command in commands {
            match command {
                ViewportCommand::Screenshot(callback) => {
                    self.screenshot_commands_with_frame_delay
                        .push((callback, 1));
                }
                _ => {
                    // TODO(emilk): handle some of the commands
                    log::warn!(
                        "Unhandled egui viewport command: {command:?} - not implemented in web backend"
                    );
                }
            }
        }
    }

    /// Paint the results of the last call to [`Self::logic`].
    pub fn paint(&mut self) {
        let clipped_primitives = core::mem::take(&mut self.clipped_primitives);

        if let Some(clipped_primitives) = clipped_primitives {
            let mut screenshot_commands = vec![];
            self.screenshot_commands_with_frame_delay
                .retain_mut(|(callback, frame_delay)| {
                    if *frame_delay == 0 {
                        screenshot_commands.push(callback.clone());
                        false
                    } else {
                        *frame_delay -= 1;
                        true
                    }
                });
            if !self.screenshot_commands_with_frame_delay.is_empty() {
                self.egui_ctx().request_repaint();
            }

            if let Err(err) = self.painter.paint_and_update_textures(
                self.app.clear_color(&self.egui_ctx.global_style().visuals),
                &clipped_primitives,
                self.egui_ctx.pixels_per_point(),
                &mut self.textures_delta,
                screenshot_commands,
            ) {
                log::error!("Failed to paint: {}", super::string_from_js_value(&err));
            }
        }
    }

    pub fn report_frame_time(&mut self, cpu_usage_seconds: f32) {
        self.frame.info.cpu_usage = Some(cpu_usage_seconds);
    }

    fn handle_platform_output(&self, platform_output: egui::PlatformOutput) {
        #[cfg(feature = "web_screen_reader")]
        if self.egui_ctx.options(|o| o.screen_reader) {
            super::screen_reader::speak(&platform_output.events_description());
        }

        let egui::PlatformOutput {
            commands,
            cursor_icon,
            cursor_image: _, // TODO(alextournai): support custom bitmap cursors on the web (via CSS `url(...)`)
            events: _,       // already handled
            mutable_text_under_cursor: _, // TODO(#4569): https://github.com/emilk/egui/issues/4569
            ime,
            accesskit_update: _,        // not currently implemented
            num_completed_passes: _,    // handled by `Context::run`
            request_discard_reasons: _, // handled by `Context::run`
        } = platform_output;

        for command in commands {
            match command {
                egui::OutputCommand::CopyText(text) => {
                    super::set_clipboard_text(&text);
                }
                egui::OutputCommand::CopyImage(image) => {
                    super::set_clipboard_image(&image);
                }
                egui::OutputCommand::OpenUrl(open_url) => {
                    super::open_url(&open_url.url, open_url.new_tab);
                }
            }
        }

        if let Some(host) = &self.host {
            host.send_cursor_icon(cursor_icon);
        } else {
            super::set_cursor_icon(self.html_canvas(), cursor_icon);
        }

        if self.has_focus() {
            // The eframe app has focus.
            if let Some(ime) = ime
                && let Some(text_agent) = &self.text_agent
            {
                if ime.should_interrupt_composition {
                    text_agent.interrupt_ime_composition();
                }
                // We are editing text: give the focus to the text agent.
                text_agent.focus();
            } else {
                if let Some(text_agent) = &self.text_agent {
                    // We are not editing text - give the focus to the canvas.
                    text_agent.blur();
                }
                if let Some(canvas) = self.canvas.as_html() {
                    super::focus_without_scroll(canvas).ok();
                }
            }
        }

        if let Some(text_agent) = &self.text_agent
            && let Err(err) =
                text_agent.update(ime, self.html_canvas(), self.egui_ctx.zoom_factor())
        {
            log::error!(
                "failed to update text agent position: {}",
                super::string_from_js_value(&err)
            );
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

    fn remove_string(&mut self, key: &str) {
        super::storage::local_storage_remove(key);
    }

    fn flush(&mut self) {}
}
