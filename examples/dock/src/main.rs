#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::Color32;

use egui_extras::dock;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native("Dock", options, Box::new(|_cc| Box::<MyApp>::default()))
}

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
        ui.label(&self.title);
        let dragged = ui
            .add(egui::Button::new("Drag me to drag view").sense(egui::Sense::drag()))
            .on_hover_cursor(egui::CursorIcon::Grab)
            .dragged();
        if dragged {
            dock::UiResponse::DragStarted
        } else {
            dock::UiResponse::None
        }
    }
}

struct MyApp {
    dock: dock::Dock<View>,
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

        let tab0 = { nodes.insert_leaf(gen_view()) };
        let tab1 = {
            let a = nodes.insert_leaf(gen_view());
            let b = nodes.insert_leaf(gen_view());
            nodes.insert_tab_node(vec![a, b])
        };
        let tab2 = {
            let a = nodes.insert_leaf(gen_view());
            let b = nodes.insert_leaf(gen_view());
            nodes.insert_horizontal_node(vec![a, b])
        };

        let root = nodes.insert_tab_node(vec![tab0, tab1, tab2]);

        let dock = dock::Dock::new(root, nodes);

        Self { dock }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut behavior = DockBehavior {};

        egui::SidePanel::left("tree").show(ctx, |ui| {
            if ui.button("Reset").clicked() {
                *self = Default::default();
            }
            ui.separator();

            tree_ui(ui, &mut behavior, &self.dock.nodes, self.dock.root);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.dock.ui(&mut behavior, ui);
        });
    }
}

fn tree_ui(
    ui: &mut egui::Ui,
    behavior: &mut dyn dock::Behavior<View>,
    nodes: &dock::Nodes<View>,
    node_id: dock::NodeId,
) {
    let Some(node) = nodes.get(node_id) else { return; };

    // if let dock::NodeLayout::Leaf(view) = node {
    //     ui.label(&view.title);
    //     return;
    // }

    egui::CollapsingHeader::new(behavior.tab_text_for_node(nodes, node_id))
        .id_source((node_id, "tree"))
        .default_open(true)
        .show(ui, |ui| match node {
            dock::NodeLayout::Leaf(_) => {}
            dock::NodeLayout::Tabs(tabs) => {
                for &child in &tabs.children {
                    tree_ui(ui, behavior, nodes, child);
                }
            }
            dock::NodeLayout::Horizontal(layout) => {
                for &child in &layout.children {
                    tree_ui(ui, behavior, nodes, child);
                }
            }
            dock::NodeLayout::Vertical(layout) => {
                for &child in &layout.children {
                    tree_ui(ui, behavior, nodes, child);
                }
            }
        });
}

struct DockBehavior {}

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
}
