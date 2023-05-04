#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self, Style};
use egui::Color32;

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
    title: String,
    color: Color32,
}

impl View {
    pub fn with_nr(i: usize) -> Self {
        Self {
            title: format!("View {i}"),
            color: egui::epaint::Hsva::new(0.1 * i as f32, 0.5, 0.5, 1.0).into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> dock::UiResponse {
        ui.painter().rect_filled(ui.max_rect(), 0.0, self.color);
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
}

impl Default for DockBehavior {
    fn default() -> Self {
        Self {
            simplification_options: Default::default(),
            tab_bar_height: 20.0,
            gap_width: 2.0,
        }
    }
}

impl DockBehavior {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            simplification_options,
            tab_bar_height,
            gap_width,
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

    fn tab_text_for_leaf(&mut self, view: &View) -> egui::WidgetText {
        view.title.clone().into()
    }

    // ---
    // Settings:

    fn tab_bar_height(&self, _style: &Style) -> f32 {
        self.tab_bar_height
    }

    fn gap_width(&self, _style: &Style) -> f32 {
        self.gap_width
    }

    fn simplification_options(&self) -> dock::SimplificationOptions {
        self.simplification_options
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct MyApp {
    dock: dock::Dock<View>,

    #[serde(skip, default)]
    behavior: DockBehavior,
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
        tabs.push(nodes.insert_leaf(gen_view()));
        tabs.push({
            let a = nodes.insert_leaf(gen_view());
            let b = nodes.insert_leaf(gen_view());
            let c = nodes.insert_leaf(gen_view());
            let d = nodes.insert_leaf(gen_view());
            let e = nodes.insert_leaf(gen_view());
            nodes.insert_tab_node(vec![a, b, c, d, e])
        });
        tabs.push({
            let a = nodes.insert_leaf(gen_view());
            let b = nodes.insert_leaf(gen_view());
            let c = nodes.insert_leaf(gen_view());
            let d = nodes.insert_leaf(gen_view());
            let e = nodes.insert_leaf(gen_view());
            nodes.insert_horizontal_node(vec![a, b, c, d, e])
        });
        tabs.push({
            let a = nodes.insert_leaf(gen_view());
            let b = nodes.insert_leaf(gen_view());
            let c = nodes.insert_leaf(gen_view());
            let d = nodes.insert_leaf(gen_view());
            let e = nodes.insert_leaf(gen_view());
            nodes.insert_vertical_node(vec![a, b, c, d, e])
        });
        tabs.push({
            let mut cells = vec![];
            for _ in 0..12 {
                cells.push(nodes.insert_leaf(gen_view()));
            }
            nodes.insert_grid_node(cells)
        });

        let root = nodes.insert_tab_node(tabs);

        let dock = dock::Dock::new(root, nodes);

        Self {
            dock,
            behavior: Default::default(),
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
    let text = format!(
        "{} - {node_id:?}",
        behavior.tab_text_for_node(nodes, node_id).text()
    );

    let Some(mut node) = nodes.nodes.remove(&node_id) else { return; };

    egui::CollapsingHeader::new(text)
        .id_source((node_id, "tree"))
        .default_open(true)
        .show(ui, |ui| match &mut node {
            dock::Node::Leaf(_) => {}
            dock::Node::Branch(branch) => {
                let mut layout = branch.get_layout();
                egui::ComboBox::from_label("Layout")
                    .selected_text(format!("{:?}", layout))
                    .show_ui(ui, |ui| {
                        for typ in dock::Layout::ALL {
                            ui.selectable_value(&mut layout, typ, format!("{:?}", typ))
                                .clicked();
                        }
                    });
                if layout != branch.get_layout() {
                    branch.set_layout(layout);
                }

                for &child in branch.children() {
                    tree_ui(ui, behavior, nodes, child);
                }
            }
        });

    nodes.nodes.insert(node_id, node);
}
