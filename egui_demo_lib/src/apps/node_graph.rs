use egui::{containers::*, *};

// pub use petgraph::graph::{EdgeIndex as EdgeId, NodeIndex as NodeId};
slotmap::new_key_type! { pub struct EdgeId; }
slotmap::new_key_type! { pub struct NodeId; }

pub type Nodes = slotmap::SlotMap<NodeId, Node>;
pub type Edges = slotmap::SlotMap<EdgeId, Edge>;

#[derive(Clone, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct NodeGraph {
    // graph: petgraph::Graph<Node, Edge>,
    nodes: Nodes,
    edges: Edges,
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Edge {
    nodes: [NodeId; 2],
}

impl Edge {
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self { nodes: [from, to] }
    }
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Node {
    /// PERSISTED MODE: position is relative to parent!
    rect: Rect,
    title: String,
    description: String,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            rect: Rect::from_center_size(Pos2::ZERO, Vec2::splat(200.0)),
            title: "unnamed".to_owned(),
            description: Default::default(),
        }
    }
}

impl Node {
    pub fn new(title: impl Into<String>, pos: impl Into<Pos2>) -> Self {
        Self {
            title: title.into(),
            rect: Rect::from_center_size(pos.into(), Vec2::splat(100.0)),
            ..Default::default()
        }
    }

    pub fn window(&mut self, id: NodeId, ui: &mut Ui) {
        let translation = ui.min_rect().min - Pos2::ZERO;

        // TODO: set to use same layer as parent `ui`
        let response = Window::new(self.title.clone())
            .id(egui::Id::new(id))
            .collapsible(false)
            .scroll(false)
            .resizable(true)
            .default_size(self.rect.size())
            .title_bar(false)
            .current_pos(self.rect.min + translation)
            //.show_inside(ui, |ui|{ // TODO
            .show(ui.ctx(), |ui| {
                self.ui(ui);
            });
        let response = response.expect("Window can't be closed");
        self.rect = response.rect.translate(-translation);
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let Self {
            rect: _,
            title,
            description,
        } = self;

        // Manual titlebar with editable title:
        ui.vertical_centered(|ui| {
            ui.add(
                TextEdit::singleline(title)
                    .desired_width(32.0)
                    .text_style(TextStyle::Heading)
                    .frame(false),
            );
        });
        ui.separator();

        // Body:
        ui.horizontal(|ui| {
            ui.label("Description:");
            ui.text_edit_singleline(description);
        });
    }
}

impl Edge {
    fn ui(&self, ui: &Ui, nodes: &Nodes) {
        let translation = ui.min_rect().min - Pos2::ZERO;

        if let (Some(node0), Some(node1)) = (nodes.get(self.nodes[0]), nodes.get(self.nodes[1])) {
            let rects = [
                node0.rect.translate(translation),
                node1.rect.translate(translation),
            ];

            let x = lerp(rects[0].center().x..=rects[1].center().x, 0.5);
            let y = lerp(rects[0].center().y..=rects[1].center().y, 0.5);

            let p0 = rects[0].clamp(pos2(x, y));
            let p1 = rects[1].clamp(pos2(x, y));

            let stroke = Stroke::new(2.0, Color32::from_gray(200));
            ui.painter().arrow(p0, p1 - p0, stroke);
        }
    }
}

impl NodeGraph {
    fn egiu_deps() -> Self {
        let mut graph = Self::default();
        let emath = graph.add_node(Node::new("emath", [200., 500.]));
        let epaint = graph.add_node(Node::new("epaint", [200., 400.]));
        let egui = graph.add_node(Node::new("egui", [200., 300.]));

        graph.add_edge(Edge::new(egui, emath));
        graph.add_edge(Edge::new(egui, epaint));

        graph
    }

    fn add_node(&mut self, node: Node) -> NodeId {
        self.nodes.insert(node)
    }

    fn add_edge(&mut self, edge: Edge) -> EdgeId {
        self.edges.insert(edge)
    }
}

impl epi::App for NodeGraph {
    fn name(&self) -> &str {
        "Node Graph"
    }

    #[cfg(feature = "persistence")]
    fn load(&mut self, storage: &dyn epi::Storage) {
        *self = epi::get_value(storage, "egui_demo_lib/apps/node_graph").unwrap_or_default()
    }

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, "egui_demo_lib/apps/node_graph", self);
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        // TODO: side panel with "add windows" and whatnot

        egui::SidePanel::left("control_ui", 100.0).show(ctx, |ui| self.control_ui(ui));

        egui::CentralPanel::default()
            .frame(Frame::dark_canvas(&ctx.style()))
            .show(ctx, |ui| self.contents_ui(ui));
    }
}

impl NodeGraph {
    pub fn control_ui(&mut self, ui: &mut Ui) {
        // egui::reset_button_with(ui, self, Self::egiu_deps());
        if ui.button("Reset").clicked() {
            *self = Self::egiu_deps();
        }
    }

    pub fn contents_ui(&mut self, ui: &mut Ui) {
        for (node_id, node) in &mut self.nodes {
            node.window(node_id, ui);
        }
        for (_edge_id, edge) in &mut self.edges {
            edge.ui(ui, &mut self.nodes);
        }
    }
}
