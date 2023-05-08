#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

use egui_extras::dock;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Dock",
        options,
        Box::new(|cc| {
            let mut app = MyApp::default();
            if let Some(storage) = cc.storage {
                if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
                    app = state;
                }
            }
            Box::new(app)
        }),
    )
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct View {
    nr: usize,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").field("nr", &self.nr).finish()
    }
}

impl View {
    pub fn with_nr(nr: usize) -> Self {
        Self { nr }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> dock::UiResponse {
        let color = egui::epaint::Hsva::new(0.1 * self.nr as f32, 0.5, 0.5, 1.0);
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);
        let dragged = ui
            .allocate_rect(ui.max_rect(), egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::Grab)
            .dragged();
        if dragged {
            dock::UiResponse::DragStarted
        } else {
            dock::UiResponse::None
        }
    }
}

struct DockBehavior {
    simplification_options: dock::SimplificationOptions,
    tab_bar_height: f32,
    gap_width: f32,
    add_child_to: Option<dock::NodeId>,
}

impl Default for DockBehavior {
    fn default() -> Self {
        Self {
            simplification_options: Default::default(),
            tab_bar_height: 24.0,
            gap_width: 2.0,
            add_child_to: None,
        }
    }
}

impl DockBehavior {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            simplification_options,
            tab_bar_height,
            gap_width,
            add_child_to: _,
        } = self;

        egui::Grid::new("behavior_ui")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("All leaves must have tabs:");
                ui.checkbox(&mut simplification_options.all_leaves_must_have_tabs, "");
                ui.end_row();

                ui.label("Tab bar height:");
                ui.add(
                    egui::DragValue::new(tab_bar_height)
                        .clamp_range(0.0..=100.0)
                        .speed(1.0),
                );
                ui.end_row();

                ui.label("Gap width:");
                ui.add(
                    egui::DragValue::new(gap_width)
                        .clamp_range(0.0..=20.0)
                        .speed(1.0),
                );
                ui.end_row();
            });
    }
}

impl dock::Behavior<View> for DockBehavior {
    fn leaf_ui(
        &mut self,
        ui: &mut egui::Ui,
        _node_id: dock::NodeId,
        view: &mut View,
    ) -> dock::UiResponse {
        view.ui(ui)
    }

    fn tab_title_for_leaf(&mut self, view: &View) -> egui::WidgetText {
        format!("View {}", view.nr).into()
    }

    fn top_bar_rtl_ui(&mut self, ui: &mut egui::Ui, node_id: dock::NodeId) {
        if ui.button("➕").clicked() {
            self.add_child_to = Some(node_id);
        }
    }

    // ---
    // Settings:

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        self.tab_bar_height
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        self.gap_width
    }

    fn simplification_options(&self) -> dock::SimplificationOptions {
        self.simplification_options
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct MyApp {
    dock: dock::Dock<View>,

    #[serde(skip)]
    behavior: DockBehavior,

    #[serde(skip)]
    last_dock_debug: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut next_view_nr = 0;
        let mut gen_view = || {
            let view = View::with_nr(next_view_nr);
            next_view_nr += 1;
            view
        };

        let mut nodes = dock::Nodes::default();

        let mut tabs = vec![];
        let tab_node = {
            let children = (0..7).map(|_| nodes.insert_leaf(gen_view())).collect();
            nodes.insert_tab_node(children)
        };
        tabs.push(tab_node);
        tabs.push({
            let children = (0..7).map(|_| nodes.insert_leaf(gen_view())).collect();
            nodes.insert_horizontal_node(children)
        });
        tabs.push({
            let children = (0..7).map(|_| nodes.insert_leaf(gen_view())).collect();
            nodes.insert_vertical_node(children)
        });
        tabs.push({
            let cells = (0..11).map(|_| nodes.insert_leaf(gen_view())).collect();
            nodes.insert_grid_node(cells)
        });
        tabs.push(nodes.insert_leaf(gen_view()));

        let root = nodes.insert_tab_node(tabs);

        let dock = dock::Dock::new(root, nodes);

        Self {
            dock,
            behavior: Default::default(),
            last_dock_debug: Default::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("tree").show(ctx, |ui| {
            if ui.button("Reset").clicked() {
                *self = Default::default();
            }
            self.behavior.ui(ui);
            ui.separator();

            tree_ui(ui, &mut self.behavior, &mut self.dock.nodes, self.dock.root);

            if let Some(parent) = self.behavior.add_child_to.take() {
                let new_child = self.dock.nodes.insert_leaf(View::with_nr(100));
                if let Some(dock::Node::Branch(dock::Branch::Tabs(tabs))) =
                    self.dock.nodes.get_mut(parent)
                {
                    tabs.add_child(new_child);
                    tabs.set_active(new_child);
                }
            }

            ui.separator();
            ui.style_mut().wrap = Some(false);
            let dock_debug = format!("{:#?}", self.dock);
            ui.monospace(&dock_debug);
            if self.last_dock_debug != dock_debug {
                self.last_dock_debug = dock_debug;
                log::debug!("{}", self.last_dock_debug);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.dock.ui(&mut self.behavior, ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self);
    }
}

fn tree_ui(
    ui: &mut egui::Ui,
    behavior: &mut dyn dock::Behavior<View>,
    nodes: &mut dock::Nodes<View>,
    node_id: dock::NodeId,
) {
    // Get the name BEFORE we remove the node below!
    let text = format!(
        "{} - {node_id:?}",
        behavior.tab_title_for_node(nodes, node_id).text()
    );

    let Some(mut node) = nodes.nodes.remove(&node_id) else {
        log::warn!("Missing node {node_id:?}");
        return;
    };

    egui::CollapsingHeader::new(text)
        .id_source((node_id, "tree"))
        .default_open(true)
        .show(ui, |ui| match &mut node {
            dock::Node::Leaf(_) => {}
            dock::Node::Branch(branch) => {
                let mut layout = branch.layout();
                egui::ComboBox::from_label("Layout")
                    .selected_text(format!("{:?}", layout))
                    .show_ui(ui, |ui| {
                        for typ in dock::Layout::ALL {
                            ui.selectable_value(&mut layout, typ, format!("{:?}", typ))
                                .clicked();
                        }
                    });
                if layout != branch.layout() {
                    branch.set_layout(layout);
                }

                for &child in branch.children() {
                    tree_ui(ui, behavior, nodes, child);
                }
            }
        });

    nodes.nodes.insert(node_id, node);
}
