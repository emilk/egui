#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(clippy::unwrap_used, rustdoc::missing_crate_level_docs)] // it's a test

use std::sync::Arc;

use eframe::egui;
use egui::{Id, InnerResponse, UiBuilder, ViewportBuilder, ViewportId, mutex::RwLock};

// Drag-and-drop between windows is not yet implemented, but if you wanna work on it, enable this:
pub const DRAG_AND_DROP_TEST: bool = false;

fn main() {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let _ = eframe::run_native(
        "Viewports",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([450.0, 400.0]),

            #[cfg(feature = "wgpu")]
            renderer: eframe::Renderer::Wgpu,

            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::<App>::default())),
    );
}

pub struct ViewportState {
    pub id: ViewportId,
    pub visible: bool,
    pub immediate: bool,
    pub title: String,
    pub children: Vec<Arc<RwLock<Self>>>,
}

impl ViewportState {
    pub fn new_deferred(
        title: &'static str,
        children: Vec<Arc<RwLock<Self>>>,
    ) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            id: ViewportId::from_hash_of(title),
            visible: false,
            immediate: false,
            title: title.into(),
            children,
        }))
    }

    pub fn new_immediate(
        title: &'static str,
        children: Vec<Arc<RwLock<Self>>>,
    ) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            id: ViewportId::from_hash_of(title),
            visible: false,
            immediate: true,
            title: title.into(),
            children,
        }))
    }

    pub fn show(vp_state: Arc<RwLock<Self>>, ctx: &egui::Context, close_button: bool) {
        if !vp_state.read().visible {
            return;
        }
        let vp_id = vp_state.read().id;
        let immediate = vp_state.read().immediate;
        let title = vp_state.read().title.clone();

        let viewport = ViewportBuilder::default()
            .with_title(&title)
            .with_close_button(close_button)
            .with_inner_size([500.0, 500.0]);

        if immediate {
            let mut vp_state = vp_state.write();
            ctx.show_viewport_immediate(vp_id, viewport, move |ui, class| {
                if ui.input(|i| i.viewport().close_requested()) {
                    vp_state.visible = false;
                }
                show_as_popup(ui, class, |ui: &mut egui::Ui| {
                    generic_child_ui(ui, &mut vp_state, close_button);
                });
            });
        } else {
            let count = Arc::new(RwLock::new(0));
            ctx.show_viewport_deferred(vp_id, viewport, move |ui, class| {
                let mut vp_state = vp_state.write();
                if ui.input(|i| i.viewport().close_requested()) {
                    vp_state.visible = false;
                }
                let count = Arc::clone(&count);
                show_as_popup(ui, class, move |ui: &mut egui::Ui| {
                    let current_count = *count.read();
                    ui.label(format!("Callback has been reused {current_count} times"));
                    *count.write() += 1;

                    generic_child_ui(ui, &mut vp_state, close_button);
                });
            });
        }
    }

    pub fn set_visible_recursive(&mut self, visible: bool) {
        self.visible = visible;
        for child in &self.children {
            child.write().set_visible_recursive(true);
        }
    }
}

pub struct App {
    top: Vec<Arc<RwLock<ViewportState>>>,
    close_button: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            top: vec![
                ViewportState::new_deferred(
                    "Top Deferred Viewport",
                    vec![
                        ViewportState::new_deferred(
                            "DD: Deferred Viewport in Deferred Viewport",
                            vec![],
                        ),
                        ViewportState::new_immediate(
                            "DS: Immediate Viewport in Deferred Viewport",
                            vec![],
                        ),
                    ],
                ),
                ViewportState::new_immediate(
                    "Top Immediate Viewport",
                    vec![
                        ViewportState::new_deferred(
                            "SD: Deferred Viewport in Immediate Viewport",
                            vec![],
                        ),
                        ViewportState::new_immediate(
                            "SS: Immediate Viewport in Immediate Viewport",
                            vec![],
                        ),
                    ],
                ),
            ],
            close_button: true,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Root viewport");
            {
                let mut embed_viewports = ui.embed_viewports();
                ui.checkbox(&mut embed_viewports, "Embed all viewports");
                if ui.button("Open all viewports").clicked() {
                    for viewport in &self.top {
                        viewport.write().set_visible_recursive(true);
                    }
                }
                ui.set_embed_viewports(embed_viewports);
            }
            ui.checkbox(&mut self.close_button, "with close button");
            generic_ui(ui, &self.top, self.close_button);
        });
    }
}

/// This will make the content as a popup if cannot has his own native window
fn show_as_popup(
    ui: &mut egui::Ui,
    class: egui::ViewportClass,
    content: impl FnOnce(&mut egui::Ui),
) {
    if class == egui::ViewportClass::EmbeddedWindow {
        // Not a real viewport - already has a frame
        content(ui);
    } else {
        egui::CentralPanel::default().show_inside(ui, content);
    }
}

fn generic_child_ui(ui: &mut egui::Ui, vp_state: &mut ViewportState, close_button: bool) {
    ui.horizontal(|ui| {
        ui.label("Title:");
        if ui.text_edit_singleline(&mut vp_state.title).changed() {
            // Title changes
            ui.send_viewport_cmd_to(
                vp_state.id,
                egui::ViewportCommand::Title(vp_state.title.clone()),
            );
        }
    });

    generic_ui(ui, &vp_state.children, close_button);
}

fn generic_ui(ui: &mut egui::Ui, children: &[Arc<RwLock<ViewportState>>], close_button: bool) {
    let container_id = ui.id();

    let ctx = ui.ctx().clone();
    ui.label(format!(
        "Frame nr: {} (this increases when this viewport is being rendered)",
        ctx.cumulative_pass_nr()
    ));
    ui.horizontal(|ui| {
        let mut show_spinner =
            ui.data_mut(|data| *data.get_temp_mut_or(container_id.with("show_spinner"), false));
        ui.checkbox(&mut show_spinner, "Show Spinner (forces repaint)");
        if show_spinner {
            ui.spinner();
        }
        ui.data_mut(|data| data.insert_temp(container_id.with("show_spinner"), show_spinner));
    });

    ui.add_space(8.0);

    ui.label(format!("Viewport Id: {:?}", ctx.viewport_id()));
    ui.label(format!(
        "Parent Viewport Id: {:?}",
        ctx.parent_viewport_id()
    ));

    ui.collapsing("Info", |ui| {
        ui.label(format!("zoom_factor: {}", ctx.zoom_factor()));
        ui.label(format!("pixels_per_point: {}", ctx.pixels_per_point()));

        if let Some(native_pixels_per_point) = ctx.input(|i| i.viewport().native_pixels_per_point) {
            ui.label(format!(
                "native_pixels_per_point: {native_pixels_per_point:?}"
            ));
        }
        if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
            ui.label(format!("monitor_size: {monitor_size:?} (points)"));
        }
        if let Some(viewport_rect) = ui.input(|i| i.raw.screen_rect) {
            ui.label(format!(
                "Viewport Rect: Pos: {:?}, Size: {:?} (points)",
                viewport_rect.min,
                viewport_rect.size()
            ));
        }
        if let Some(inner_rect) = ctx.input(|i| i.viewport().inner_rect) {
            ui.label(format!(
                "Inner Rect: Pos: {:?}, Size: {:?} (points)",
                inner_rect.min,
                inner_rect.size()
            ));
        }
        if let Some(outer_rect) = ctx.input(|i| i.viewport().outer_rect) {
            ui.label(format!(
                "Outer Rect: Pos: {:?}, Size: {:?} (points)",
                outer_rect.min,
                outer_rect.size()
            ));
        }
    });

    if ctx.viewport_id() != ctx.parent_viewport_id() {
        let parent = ctx.parent_viewport_id();
        if ui.button("Set parent pos 0,0").clicked() {
            ctx.send_viewport_cmd_to(
                parent,
                egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)),
            );
        }
    }

    if DRAG_AND_DROP_TEST {
        drag_and_drop_test(ui);
    }

    if !children.is_empty() {
        ui.separator();

        ui.heading("Children:");

        for child in children {
            let visible = {
                let mut child_lock = child.write();
                let ViewportState { visible, title, .. } = &mut *child_lock;
                ui.checkbox(visible, title.as_str());
                *visible
            };
            if visible {
                ViewportState::show(Arc::clone(child), &ctx, close_button);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Drag-and-drop between windows is not yet implemented, but there is some test code for it here:

fn drag_and_drop_test(ui: &mut egui::Ui) {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    let container_id = ui.id();

    const COLS: usize = 2;
    static DATA: OnceLock<RwLock<DragAndDrop>> = OnceLock::new();
    let data = DATA.get_or_init(Default::default);
    data.write().init(container_id);

    #[derive(Default)]
    struct DragAndDrop {
        containers_data: HashMap<Id, Vec<Vec<Id>>>,
        data: HashMap<Id, String>,
        counter: usize,
        is_dragged: Option<Id>,
    }

    impl DragAndDrop {
        fn init(&mut self, container: Id) {
            if !self.containers_data.contains_key(&container) {
                for i in 0..COLS {
                    self.insert(
                        container,
                        i,
                        format!("From: {container:?}, and is: {}", self.counter),
                    );
                }
            }
        }

        fn insert(&mut self, container: Id, col: usize, value: impl Into<String>) {
            assert!(col < COLS, "The coll should be less than: {COLS}");

            let value: String = value.into();
            let id = Id::new(format!("%{}% {}", self.counter, &value));
            self.data.insert(id, value);
            let viewport_data = self.containers_data.entry(container).or_insert_with(|| {
                let mut res = Vec::new();
                res.resize_with(COLS, Default::default);
                res
            });
            self.counter += 1;

            viewport_data[col].push(id);
        }

        fn cols(&self, container: Id, col: usize) -> Vec<(Id, String)> {
            assert!(col < COLS, "The col should be less than: {COLS}");
            let container_data = &self.containers_data[&container];
            container_data[col]
                .iter()
                .map(|id| (*id, self.data[id].clone()))
                .collect()
        }

        /// Move element ID to Viewport and col
        fn mov(&mut self, to: Id, col: usize) {
            let Some(id) = self.is_dragged.take() else {
                return;
            };
            assert!(col < COLS, "The col should be less than: {COLS}");

            // Should be a better way to do this!
            #[expect(clippy::iter_over_hash_type)]
            for container_data in self.containers_data.values_mut() {
                for ids in container_data {
                    ids.retain(|i| *i != id);
                }
            }

            if let Some(container_data) = self.containers_data.get_mut(&to) {
                container_data[col].push(id);
            }
        }

        fn dragging(&mut self, id: Id) {
            self.is_dragged = Some(id);
        }
    }

    ui.separator();
    ui.label("Drag and drop:");
    ui.columns(COLS, |ui| {
        for col in 0..COLS {
            let data = DATA.get().unwrap();
            let ui = &mut ui[col];
            let mut is_dragged = None;
            let res = drop_target(ui, |ui| {
                ui.set_min_height(60.0);
                for (id, value) in data.read().cols(container_id, col) {
                    drag_source(ui, id, |ui| {
                        ui.add(egui::Label::new(value).sense(egui::Sense::click()));
                        if ui.ctx().is_being_dragged(id) {
                            is_dragged = Some(id);
                        }
                    });
                }
            });
            if let Some(id) = is_dragged {
                data.write().dragging(id);
            }
            if res.response.hovered() && ui.input(|i| i.pointer.any_released()) {
                data.write().mov(container_id, col);
            }
        }
    });
}

// This is taken from crates/egui_demo_lib/src/debo/drag_and_drop.rs
fn drag_source<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> InnerResponse<R> {
    let is_being_dragged = ui.ctx().is_being_dragged(id);

    if !is_being_dragged {
        let res = ui.scope(body);

        // Check for drags:
        let response = ui.interact(res.response.rect, id, egui::Sense::drag());
        if response.hovered() {
            ui.set_cursor_icon(egui::CursorIcon::Grab);
        }
        res
    } else {
        ui.set_cursor_icon(egui::CursorIcon::Grabbing);

        // Paint the body to a new layer:
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let res = ui.scope_builder(UiBuilder::new().layer_id(layer_id), body);

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - res.response.rect.center();
            ui.ctx().set_transform_layer(
                layer_id,
                eframe::emath::TSTransform::from_translation(delta),
            );
        }

        res
    }
}

// TODO(emilk): Update to be more like `crates/egui_demo_lib/src/debo/drag_and_drop.rs`
fn drop_target<R>(
    ui: &mut egui::Ui,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let is_being_dragged = ui.ctx().dragged_id().is_some();

    let margin = egui::Vec2::splat(ui.visuals().clip_rect_margin); // 3.0

    let background_id = ui.painter().add(egui::Shape::Noop);

    let available_rect = ui.available_rect_before_wrap();
    let inner_rect = available_rect.shrink2(margin);
    let mut content_ui = ui.new_child(UiBuilder::new().max_rect(inner_rect));
    let ret = body(&mut content_ui);

    let outer_rect =
        egui::Rect::from_min_max(available_rect.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), egui::Sense::hover());

    let style = if is_being_dragged && response.hovered() {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };

    let fill = style.bg_fill;
    let stroke = style.bg_stroke;

    ui.painter().set(
        background_id,
        egui::epaint::RectShape::new(
            rect,
            style.corner_radius,
            fill,
            stroke,
            egui::StrokeKind::Inside,
        ),
    );

    egui::InnerResponse::new(ret, response)
}
