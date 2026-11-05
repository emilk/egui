use egui_demo_lib::{DemoWindows, i18n, is_mobile};

#[cfg(feature = "glow")]
use eframe::glow;

#[cfg(target_arch = "wasm32")]
use core::any::Any;

use crate::DemoApp;

#[cfg(feature = "easymark")]
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct EasyMarkApp {
    editor: egui_demo_lib::easy_mark::EasyMarkEditor,
}

#[cfg(feature = "easymark")]
impl DemoApp for EasyMarkApp {
    fn demo_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.editor.panels(ui);
    }
}

// ----------------------------------------------------------------------------

impl DemoApp for DemoWindows {
    fn demo_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui(ui);
    }

    fn logic(&mut self, ctx: &egui::Context) {
        self.logic(ctx);
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FractalClockApp {
    fractal_clock: crate::apps::FractalClock,
    pub mock_time: Option<f64>,
}

impl DemoApp for FractalClockApp {
    fn demo_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::dark_canvas(ui.style())
            .stroke(egui::Stroke::NONE)
            .corner_radius(0)
            .show(ui, |ui| {
                self.fractal_clock.ui(
                    ui,
                    self.mock_time
                        .or_else(|| Some(crate::seconds_since_midnight())),
                );
            });
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ColorTestApp {
    color_test: egui_demo_lib::ColorTest,
}

impl DemoApp for ColorTestApp {
    fn demo_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            if frame.is_web() {
                ui.label(i18n::t(
                    "NOTE: Some old browsers stuck on WebGL1 without sRGB support will not pass the color test.",
                ));
                ui.separator();
            }
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                self.color_test.ui(ui);
            });
        });
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Anchor {
    #[default]
    Demo,

    EasyMarkEditor,

    #[cfg(feature = "http")]
    Http,

    #[cfg(feature = "image_viewer")]
    ImageViewer,

    Clock,

    #[cfg(any(feature = "glow", feature = "wgpu"))]
    Custom3d,

    /// Rendering test
    Rendering,
}

impl Anchor {
    #[cfg(target_arch = "wasm32")]
    fn all() -> Vec<Self> {
        vec![
            Self::Demo,
            Self::EasyMarkEditor,
            #[cfg(feature = "http")]
            Self::Http,
            Self::Clock,
            #[cfg(any(feature = "glow", feature = "wgpu"))]
            Self::Custom3d,
            Self::Rendering,
        ]
    }

    #[cfg(target_arch = "wasm32")]
    fn from_str_case_insensitive(anchor: &str) -> Option<Self> {
        let anchor = anchor.to_lowercase();
        Self::all().into_iter().find(|x| x.to_string() == anchor)
    }
}

impl core::fmt::Display for Anchor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut name = format!("{self:?}");
        name.make_ascii_lowercase();
        f.write_str(&name)
    }
}

impl From<Anchor> for egui::WidgetText {
    fn from(value: Anchor) -> Self {
        Self::from(value.to_string())
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
#[must_use]
enum Command {
    Nothing,
    ResetEverything,
}

// ----------------------------------------------------------------------------

/// The state that we persist (serialize).
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct State {
    demo: DemoWindows,

    #[cfg(feature = "easymark")]
    easy_mark_editor: EasyMarkApp,

    #[cfg(feature = "http")]
    http: crate::apps::HttpApp,

    #[cfg(feature = "image_viewer")]
    image_viewer: crate::apps::ImageViewer,

    pub clock: FractalClockApp,

    rendering_test: ColorTestApp,

    selected_anchor: Anchor,
    backend_panel: super::backend_panel::BackendPanel,

    /// The UI language (a code from [`egui_demo_lib::i18n::LANGUAGES`]); empty means English.
    pub language: String,
}

/// Wraps many demo/test apps into one.
pub struct WrapApp {
    pub state: State,

    #[cfg(any(feature = "glow", feature = "wgpu"))]
    custom3d: Option<crate::apps::Custom3d>,

    dropped_files: Vec<egui::DroppedFileHandle>,
}

impl WrapApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This gives us image support:
        egui_extras::install_image_loaders(&cc.egui_ctx);

        #[cfg(feature = "accessibility_inspector")]
        cc.egui_ctx
            .add_plugin(crate::accessibility_inspector::AccessibilityInspectorPlugin::default());

        #[allow(clippy::allow_attributes, unused_mut)]
        let mut slf = Self {
            state: State::default(),

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            custom3d: crate::apps::Custom3d::new(cc),

            dropped_files: Default::default(),
        };

        #[cfg(feature = "persistence")]
        if let Some(storage) = cc.storage
            && let Some(state) = eframe::get_value(storage, eframe::APP_KEY)
        {
            slf.state = state;
        }

        slf
    }

    pub fn apps_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&'static str, Anchor, &mut dyn DemoApp)> {
        let mut vec = vec![
            (
                "✨ Demos",
                Anchor::Demo,
                &mut self.state.demo as &mut dyn DemoApp,
            ),
            #[cfg(feature = "easymark")]
            (
                "📝 EasyMark editor",
                Anchor::EasyMarkEditor,
                &mut self.state.easy_mark_editor as &mut dyn DemoApp,
            ),
            #[cfg(feature = "http")]
            (
                "⬇️ HTTP",
                Anchor::Http,
                &mut self.state.http as &mut dyn DemoApp,
            ),
            (
                "🕑 Fractal Clock",
                Anchor::Clock,
                &mut self.state.clock as &mut dyn DemoApp,
            ),
            #[cfg(feature = "image_viewer")]
            (
                "🖼️ Image Viewer",
                Anchor::ImageViewer,
                &mut self.state.image_viewer as &mut dyn DemoApp,
            ),
        ];

        #[cfg(any(feature = "glow", feature = "wgpu"))]
        if let Some(custom3d) = &mut self.custom3d {
            vec.push((
                "🔺 3D painting",
                Anchor::Custom3d,
                custom3d as &mut dyn DemoApp,
            ));
        }

        vec.push((
            "🎨 Rendering test",
            Anchor::Rendering,
            &mut self.state.rendering_test as &mut dyn DemoApp,
        ));

        vec.into_iter()
    }
}

impl eframe::App for WrapApp {
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // Give the area behind the floating windows a different color, because it looks better:
        let color = egui::lerp(
            egui::Rgba::from(visuals.panel_fill)..=egui::Rgba::from(visuals.extreme_bg_color),
            0.5,
        );
        let color = egui::Color32::from(color);
        color.to_normalized_gamma_f32()
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Run background logic for every app, even the ones not currently shown,
        // so they keep working while the app is hidden (e.g. a backgrounded tab).
        for (_name, _anchor, app) in self.apps_iter_mut() {
            app.logic(ctx);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let lang = i18n::Lang::from_code(&self.state.language).unwrap_or(i18n::Lang::EN);
        i18n::set_current(lang);
        // The interface direction follows the language (Arabic reads right-to-left).
        ui.ctx().set_right_to_left(lang.is_rtl());

        #[cfg(target_arch = "wasm32")]
        if let Some(anchor) = frame
            .info()
            .web_info
            .location
            .hash
            .strip_prefix('#')
            .and_then(Anchor::from_str_case_insensitive)
        {
            self.state.selected_anchor = anchor;
        }

        #[cfg(not(target_arch = "wasm32"))]
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ui.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // The native window title follows the language too: `main.rs` seeds it in English
            // before the persisted language is known, so refresh it once it differs.
            let title = i18n::t("egui demo app");
            let current_title = ui.input(|i| i.viewport().title.clone());
            if current_title.as_deref() != Some(title) {
                ui.send_viewport_cmd(egui::ViewportCommand::Title(title.to_owned()));
            }
        }

        let mut cmd = Command::Nothing;
        egui::Panel::top("wrap_app_top_bar")
            .frame(egui::Frame::new().inner_margin(4))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.visuals_mut().button_frame = false;
                    self.bar_contents(ui, frame, &mut cmd);
                });
            });

        self.state.backend_panel.update(ui.ctx(), frame);

        egui::CentralPanel::no_frame().show(ui, |ui| {
            if !is_mobile(ui.ctx()) {
                cmd = self.backend_panel(ui, frame);
            }

            self.show_selected_app(ui, frame);
        });

        self.state.backend_panel.end_of_frame(ui.ctx());

        self.ui_file_drag_and_drop(ui.ctx());

        self.run_cmd(ui.ctx(), cmd);
    }

    #[cfg(feature = "glow")]
    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(custom3d) = &mut self.custom3d {
            custom3d.on_exit(gl);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(&mut *self)
    }
}

impl WrapApp {
    fn backend_panel(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) -> Command {
        // The backend-panel can be toggled on/off.
        // We show a little animation when the user switches it.
        let mut is_open =
            self.state.backend_panel.open || ui.memory(|mem| mem.everything_is_visible());

        let mut cmd = Command::Nothing;

        egui::Panel::start("backend_panel")
            .resizable(true)
            .size_range(280..=400)
            .show_collapsible(ui, &mut is_open, |ui| {
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.heading(i18n::t("💻 Backend"));
                });

                ui.separator();
                self.backend_panel_contents(ui, frame, &mut cmd);
            });

        // Allow drag-to-close to close the backend panel:
        self.state.backend_panel.open = is_open;

        cmd
    }

    fn run_cmd(&mut self, ctx: &egui::Context, cmd: Command) {
        match cmd {
            Command::Nothing => {}
            Command::ResetEverything => {
                let language = core::mem::take(&mut self.state.language);
                self.state = Default::default();
                self.state.language = language;
                ctx.memory_mut(|mem| *mem = Default::default());
            }
        }
    }

    fn backend_panel_contents(
        &mut self,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
        cmd: &mut Command,
    ) {
        self.state.backend_panel.ui(ui, frame);

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .button(i18n::t("Reset egui"))
                .on_hover_text(i18n::t("Forget scroll, positions, sizes etc"))
                .clicked()
            {
                ui.memory_mut(|mem| *mem = Default::default());
                ui.close();
            }

            if ui.button(i18n::t("Reset everything")).clicked() {
                *cmd = Command::ResetEverything;
                ui.close();
            }
        });
    }

    fn show_selected_app(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let selected_anchor = self.state.selected_anchor;
        for (_name, anchor, app) in self.apps_iter_mut() {
            if anchor == selected_anchor || ui.memory(|mem| mem.everything_is_visible()) {
                app.demo_ui(ui, frame);
            }
        }
    }

    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, cmd: &mut Command) {
        ui.add_space(8.0);

        egui::widgets::global_theme_preference_buttons(ui);

        // Demo toggle for the reading direction and UI language (issue #14):
        // flip to Arabic/RTL to exercise right-to-left widgets such as the
        // date picker popup. Fork-side only; not upstreamable.
        //
        // Route through the app state: the per-frame reset at the top of `ui`
        // derives direction and translations from `self.state.language`, so a
        // direct `Context::set_right_to_left`/`set_language` here is
        // overwritten next frame. The binding still reads the direction the
        // reset derived, so the toggle shows active iff Arabic is active.
        let mut rtl = ui.ctx().is_rtl();
        if ui.toggle_value(&mut rtl, "↔ RTL · العربية").changed() {
            // Registry codes from `i18n::LANGUAGES` ("ar"/"en"), like the
            // Language dropdown's `selected.code()`; "ar-EG" is not one.
            self.state.language = if rtl { "ar" } else { "en" }.to_owned();
        }

        ui.separator();

        ui.menu_button(i18n::t("🌐 Language"), |ui| {
            let mut selected =
                i18n::Lang::from_code(&self.state.language).unwrap_or(i18n::Lang::EN);
            for lang in i18n::Lang::all() {
                // Language names are always shown in their own language.
                ui.radio_value(&mut selected, lang, lang.name());
            }
            self.state.language = selected.code().to_owned();
        });

        ui.separator();

        if is_mobile(ui.ctx()) {
            ui.menu_button(i18n::t("💻 Backend"), |ui| {
                ui.set_style(ui.global_style()); // ignore the "menu" style set by `menu_button`.
                self.backend_panel_contents(ui, frame, cmd);
            });
        } else {
            ui.toggle_value(&mut self.state.backend_panel.open, i18n::t("💻 Backend"));
        }

        ui.separator();

        let mut selected_anchor = self.state.selected_anchor;
        for (name, anchor, _app) in self.apps_iter_mut() {
            if ui
                .selectable_label(selected_anchor == anchor, i18n::t(name))
                .clicked()
            {
                selected_anchor = anchor;
                if frame.is_web() {
                    ui.open_url(egui::OpenUrl::same_tab(format!("#{anchor}")));
                }
            }
        }
        self.state.selected_anchor = selected_anchor;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if false {
                // TODO(emilk): fix the overlap on small screens
                if clock_button(ui, crate::seconds_since_midnight()).clicked() {
                    self.state.selected_anchor = Anchor::Clock;
                    if frame.is_web() {
                        ui.open_url(egui::OpenUrl::same_tab("#clock"));
                    }
                }
            }

            egui::warn_if_debug_build(ui);
        });
    }

    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        use core::fmt::Write as _;
        use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};

        // Preview hovering files:
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = i18n::t("Dropping files:\n").to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if file.mime.is_empty() {
                        text += "\n???";
                    } else {
                        write!(text, "\n{}", file.mime).ok();
                    }
                }
                text
            });

            let painter = ctx.layer_painter(LayerId::new(
                Order::Foreground,
                Id::unique("file_drop_target"),
            ));

            let content_rect = ctx.content_rect();
            painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                content_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.global_style()),
                Color32::WHITE,
            );
        }

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files.clone_from(&i.raw.dropped_files);
            }
        });

        // Show dropped files (if any):
        if !self.dropped_files.is_empty() {
            let mut open = true;
            egui::Window::new(i18n::t("Dropped files"))
                .open(&mut open)
                .show(ctx, |ui| {
                    for file in &self.dropped_files {
                        #[cfg(not(target_arch = "wasm32"))]
                        let info = file.path().display().to_string();

                        // The size and mime-type are free to read; the contents are not,
                        // so we never touch them here.
                        #[cfg(target_arch = "wasm32")]
                        let info = {
                            let Some(web_file) = file.web_file() else {
                                continue;
                            };
                            let (name, mime) = (web_file.name(), web_file.type_());
                            if mime.is_empty() {
                                format!("{name} ({} bytes)", web_file.size())
                            } else {
                                format!("{name} ({} bytes, type: {mime})", web_file.size())
                            }
                        };

                        ui.label(info);
                    }
                });
            if !open {
                self.dropped_files.clear();
            }
        }
    }
}

fn clock_button(ui: &mut egui::Ui, seconds_since_midnight: f64) -> egui::Response {
    let time = seconds_since_midnight;
    let time = format!(
        "{:02}:{:02}:{:02}.{:02}",
        (time % (24.0 * 60.0 * 60.0) / 3600.0).floor(),
        (time % (60.0 * 60.0) / 60.0).floor(),
        (time % 60.0).floor(),
        (time % 1.0 * 100.0).floor()
    );

    ui.button(egui::RichText::new(time).monospace())
}
