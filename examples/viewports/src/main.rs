use std::sync::Arc;

use eframe::egui;
use egui::{mutex::RwLock, Id, InnerResponse, ViewportBuilder};

#[derive(Default)]
pub struct App {
    show_async_viewport: bool,
    show_sync_viewport: bool,

    async_viewport_state: Arc<RwLock<usize>>,
    sync_viewport_state: usize,

    async_async_viewport_state: Arc<RwLock<usize>>,
    async_sync_viewport_state: Arc<RwLock<usize>>,

    sync_async_viewport_state: Arc<RwLock<usize>>,
    sync_sync_viewport_state: usize,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            generic_ui(ui, "Central Panel");
            {
                let mut force_embedding = ctx.force_embedding();
                ui.checkbox(&mut force_embedding, "Force embedding of new viewprts");
                ctx.set_force_embedding(force_embedding);
            }
            ui.checkbox(&mut self.show_async_viewport, "Show Async Viewport");
            ui.checkbox(&mut self.show_sync_viewport, "Show Sync Viewport");

            let ctx = ui.ctx();

            // Showing Async Viewport
            if self.show_async_viewport {
                let async_state = self.async_async_viewport_state.clone();
                let sync_state = self.async_sync_viewport_state.clone();

                show_async_viewport(
                    ctx,
                    "Async Viewport",
                    self.async_viewport_state.clone(),
                    vec![
                        AsyncViewport::new("Async Viewport in Async Viewport", move |ctx| {
                            show_async_viewport(
                                ctx,
                                "AA Async Viewport in Async Viewport",
                                async_state.clone(),
                                vec![],
                            );
                        }),
                        AsyncViewport::new("Sync Viewport in Async Viewport", move |ctx| {
                            let mut state = sync_state.write();
                            show_sync_viewport(
                                ctx,
                                "AS Sync Viewport in Async Viewport",
                                &mut state,
                                vec![],
                            );
                        }),
                    ],
                );
            }

            // Showing Sync Viewport
            if self.show_sync_viewport {
                let async_state = self.sync_async_viewport_state.clone();
                let sync_state = &mut self.sync_sync_viewport_state;
                show_sync_viewport(
                    ctx,
                    "Sync Viewport",
                    &mut self.sync_viewport_state,
                    vec![
                        SyncViewport::new("Async Viewport in Sync Viewport", move |ctx| {
                            show_async_viewport(
                                ctx,
                                "SA Async Viewport in Sync Viewport",
                                async_state.clone(),
                                vec![],
                            );
                        }),
                        SyncViewport::new("Sync Viewport in Sync Viewport", move |ctx| {
                            show_sync_viewport(
                                ctx,
                                "SS Sync Viewport in Sync Viewport",
                                sync_state,
                                vec![],
                            );
                        }),
                    ],
                );
            }
        });
    }
}

#[derive(Default, Clone)]
struct State {
    active: Vec<bool>,
}

#[derive(Clone)]
struct AsyncViewport {
    name: String,
    init: Arc<Box<dyn Fn(&egui::Context) + Sync + Send>>,
}

impl AsyncViewport {
    fn new(name: impl Into<String>, init: impl Fn(&egui::Context) + Sync + Send + 'static) -> Self {
        Self {
            name: name.into(),
            init: Arc::new(Box::new(init)),
        }
    }
}

struct SyncViewport<'a> {
    name: String,
    init: Box<dyn FnMut(&egui::Context) + 'a>,
}

impl<'a> SyncViewport<'a> {
    fn new(name: impl Into<String>, init: impl FnMut(&egui::Context) + 'a) -> Self {
        Self {
            name: name.into(),
            init: Box::new(init),
        }
    }
}

fn show_async_viewport(
    ctx: &egui::Context,
    name: impl Into<String>,
    count: Arc<RwLock<usize>>,
    viewports: Vec<AsyncViewport>,
) {
    let name: String = name.into();

    ctx.create_viewport(
        ViewportBuilder::new(name.clone())
            .with_title(name.as_str())
            .with_inner_size(Some(egui::vec2(450.0, 350.0))),
        move |ctx| {
            let mut count = count.write();
            let viewports = viewports.clone();

            let n = name.clone();
            let name = n.clone();

            let content = move |ui: &mut egui::Ui| {
                generic_ui(ui, name.clone());

                let ctx = ui.ctx().clone();
                let mut state = ctx.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_default::<State>(name.clone().into())
                        .clone()
                });

                state.active.resize(viewports.len(), false);

                for (i, viewport) in viewports.iter().enumerate() {
                    ui.checkbox(&mut state.active[i], &viewport.name);
                }

                ui.add(egui::DragValue::new(&mut *count).prefix("Count: "));

                for (i, viewport) in viewports.iter().enumerate() {
                    if state.active[i] {
                        (viewport.init)(&ctx);
                    }
                }

                ctx.memory_mut(move |mem| {
                    *mem.data.get_temp_mut_or_default::<State>(name.into()) = state;
                });
            };

            show_as_popup(ctx, &n, content);
        },
    );
}

fn show_sync_viewport(
    ctx: &egui::Context,
    name: impl Into<String>,
    count: &mut usize,
    mut viewports: Vec<SyncViewport<'_>>,
) {
    let name: String = name.into();

    ctx.create_viewport_sync(
        ViewportBuilder::new(name.clone())
            .with_title(name.as_str())
            .with_inner_size(Some(egui::vec2(450.0, 350.0))),
        move |ctx| {
            let n = name.clone();

            let content = |ui: &mut egui::Ui| {
                generic_ui(ui, name.clone());

                let ctx = ui.ctx().clone();
                let mut state = ctx.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_default::<State>(name.clone().into())
                        .clone()
                });

                state.active.resize(viewports.len(), false);

                for (i, viewport) in viewports.iter().enumerate() {
                    ui.checkbox(&mut state.active[i], &viewport.name);
                }

                ui.add(egui::DragValue::new(count).prefix("Count: "));

                for (i, viewport) in viewports.iter_mut().enumerate() {
                    if state.active[i] {
                        (viewport.init)(&ctx);
                    }
                }

                ctx.memory_mut(move |mem| {
                    *mem.data.get_temp_mut_or_default::<State>(name.into()) = state;
                });
            };

            show_as_popup(ctx, &n, content);
        },
    );
}

// This is taken from crates/egui_demo_lib/src/debo/drag_and_drop.rs
fn drag_source<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> InnerResponse<R> {
    let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

    if !is_being_dragged {
        let res = ui.scope(body);

        // Check for drags:
        let response = ui.interact(res.response.rect, id, egui::Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        res
    } else {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

        // Paint the body to a new layer:
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let res = ui.with_layer_id(layer_id, body);

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - res.response.rect.center();
            ui.ctx().translate_layer(layer_id, delta);
        }

        res
    }
}

// This is taken from crates/egui_demo_lib/src/debo/drag_and_drop.rs
fn drop_target<R>(
    ui: &mut egui::Ui,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

    let margin = egui::Vec2::splat(ui.visuals().clip_rect_margin); // 3.0

    let background_id = ui.painter().add(egui::Shape::Noop);

    let available_rect = ui.available_rect_before_wrap();
    let inner_rect = available_rect.shrink2(margin);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
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
        egui::epaint::RectShape::new(rect, style.rounding, fill, stroke),
    );

    egui::InnerResponse::new(ret, response)
}

/// This will make the content as a popup if cannot has his own native window
fn show_as_popup(ctx: &egui::Context, name: &str, content: impl FnOnce(&mut egui::Ui)) {
    if ctx.viewport_id() == ctx.parent_viewport_id() {
        egui::Window::new(name).show(ctx, content);
    } else {
        egui::CentralPanel::default().show(ctx, content);
    }
}

fn generic_ui(ui: &mut egui::Ui, container_id: impl Into<Id>) {
    let container_id = container_id.into();

    let ctx = ui.ctx().clone();
    ui.label(format!(
        "Frame nr: {} (this increases when this viewport is being rendered)",
        ctx.frame_nr()
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

    ui.label(format!("Viewport Id: {}", ctx.viewport_id()));
    ui.label(format!("Parent Viewport Id: {}", ctx.parent_viewport_id()));

    ui.add_space(8.0);

    let inner_rect = ctx.inner_rect();
    ui.label(format!(
        "Inner Rect: Pos: {:?}, Size: {:?}",
        inner_rect.min,
        inner_rect.size()
    ));
    let outer_rect = ctx.outer_rect();
    ui.label(format!(
        "Outer Rect: Pos: {:?}, Size: {:?}",
        outer_rect.min,
        outer_rect.size()
    ));

    let tmp_pixels_per_point = ctx.pixels_per_point();
    let mut pixels_per_point = ui.data_mut(|data| {
        *data.get_temp_mut_or(container_id.with("pixels_per_point"), tmp_pixels_per_point)
    });
    let res = ui.add(
        egui::DragValue::new(&mut pixels_per_point)
            .prefix("Pixels per Point: ")
            .speed(0.1)
            .clamp_range(0.5..=4.0),
    );
    if res.drag_released() {
        ctx.set_pixels_per_point(pixels_per_point);
    }
    if res.dragged() {
        ui.data_mut(|data| {
            data.insert_temp(container_id.with("pixels_per_point"), pixels_per_point);
        });
    } else {
        ui.data_mut(|data| {
            data.insert_temp(container_id.with("pixels_per_point"), tmp_pixels_per_point);
        });
    }
    egui::gui_zoom::zoom_with_keyboard_shortcuts(&ctx, None);

    if ctx.viewport_id() != ctx.parent_viewport_id() {
        let parent = ctx.parent_viewport_id();
        if ui.button("Set parent pos 0,0").clicked() {
            ctx.viewport_command_for(
                parent,
                egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)),
            );
        }
    }

    use std::collections::HashMap;
    use std::sync::OnceLock;

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
            assert!(col <= COLS, "The coll should be less then: {COLS}");

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
            assert!(col <= COLS, "The col should be less then: {COLS}");
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
            assert!(col <= COLS, "The col should be less then: {COLS}");

            // Should be a better way to do this!
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
                        if ui.memory(|mem| mem.is_being_dragged(id)) {
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
    ui.separator();
}

fn main() {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let _ = eframe::run_native(
        "Viewports",
        eframe::NativeOptions {
            #[cfg(feature = "wgpu")]
            renderer: eframe::Renderer::Wgpu,

            initial_window_size: Some(egui::Vec2::new(450.0, 320.0)),
            ..Default::default()
        },
        Box::new(|_| Box::<App>::default()),
    );
}
