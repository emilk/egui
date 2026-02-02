use eframe::egui;
use eframe_agent::views::ViewContext;
use eframe_agent::{AgentState, AgentView};

#[derive(Default)]
pub struct CounterView {
    count: i32,
}

impl AgentView for CounterView {
    fn id(&self) -> &'static str {
        "counter_view"
    }

    fn show(&mut self, ctx: &mut ViewContext<'_>, _state: &mut AgentState) {
        egui::Panel::top("counter_panel").show_inside(ctx.ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Counter");
                let decrement = ui.button("Decrement");
                set_author_id(ui.ctx(), &decrement, "counter_decrement");
                if decrement.clicked() {
                    self.count = self.count.saturating_sub(1);
                }
                let reset = ui.button("Reset");
                set_author_id(ui.ctx(), &reset, "counter_reset");
                if reset.clicked() {
                    self.count = 0;
                }
                let increment = ui.button("Increment");
                set_author_id(ui.ctx(), &increment, "counter_increment");
                if increment.clicked() {
                    self.count = self.count.saturating_add(1);
                }
                let increment_plus_two = ui.button("Increment+2");
                set_author_id(ui.ctx(), &increment_plus_two, "counter_increment_plus_two");
                if increment_plus_two.clicked() {
                    self.count = self.count.saturating_add(2);
                }
                ui.separator();
                let counter_label = ui.label(format!("Counter: {}", self.count));
                set_author_id(ui.ctx(), &counter_label, "counter_value");
            });
        });
    }
}

fn set_author_id(ctx: &egui::Context, response: &egui::Response, author_id: &str) {
    ctx.accesskit_node_builder(response.id, |builder| {
        builder.set_author_id(author_id);
    });
}
