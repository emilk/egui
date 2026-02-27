use eframe::egui;

use super::{AgentView, ViewContext};
use crate::state::AgentState;

/// Debug view that displays MCP tool calls.
#[derive(Clone, Debug)]
pub struct ToolLogView {
    title: String,
    as_window: bool,
    max_height: f32,
    window_size: egui::Vec2,
}

impl ToolLogView {
    /// Create a new tool log view (windowed by default).
    #[inline]
    pub fn new() -> Self {
        Self {
            title: "Tool Log".to_string(),
            as_window: true,
            max_height: 140.0,
            window_size: egui::vec2(360.0, 180.0),
        }
    }

    /// Show the log inside the current layout instead of a window.
    #[inline]
    pub fn inline(mut self) -> Self {
        self.as_window = false;
        self
    }

    /// Show the log in a floating window.
    #[inline]
    pub fn windowed(mut self) -> Self {
        self.as_window = true;
        self
    }

    /// Set the title used by the header or window.
    #[inline]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the maximum height for the inline scroll area.
    #[inline]
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    /// Set the default size for the floating window.
    #[inline]
    pub fn with_window_size(mut self, size: egui::Vec2) -> Self {
        self.window_size = size;
        self
    }

    fn render_log(&self, ui: &mut egui::Ui, state: &AgentState, max_height: Option<f32>) {
        let mut scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
        if let Some(max_height) = max_height {
            scroll = scroll.max_height(max_height);
        }
        scroll.show(ui, |ui| {
            if state.ui_log.is_empty() {
                ui.weak("No tool calls yet");
            } else {
                for entry in &state.ui_log {
                    ui.monospace(entry);
                }
            }
        });
    }
}

impl Default for ToolLogView {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentView for ToolLogView {
    fn id(&self) -> &'static str {
        "tool_log_view"
    }

    fn show(&mut self, ctx: &mut ViewContext<'_>, state: &mut AgentState) {
        if self.as_window {
            egui::Window::new(self.title.clone())
                .default_size(self.window_size)
                .resizable(true)
                .show(ctx.ui.ctx(), |ui| {
                    self.render_log(ui, state, None);
                });
        } else {
            ctx.ui.separator();
            ctx.ui.heading(&self.title);
            self.render_log(ctx.ui, state, Some(self.max_height));
        }
    }
}
