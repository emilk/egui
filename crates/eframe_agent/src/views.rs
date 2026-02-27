use std::sync::Arc;

use crate::{runtime::AgentRuntime, state::AgentState};
use eframe::egui;

/// Context shared with each view invocation.
pub struct ViewContext<'a> {
    /// Root UI handle for the frame.
    pub ui: &'a mut egui::Ui,

    /// Frame handle for integration operations.
    pub frame: &'a mut eframe::Frame,

    /// The active runtime.
    pub runtime: &'a Arc<dyn AgentRuntime>,
}

/// Trait implemented by all agent UI views.
pub trait AgentView {
    /// Stable identifier for debugging purposes.
    fn id(&self) -> &'static str;

    /// Render the view.
    fn show(&mut self, ctx: &mut ViewContext<'_>, state: &mut AgentState);
}

/// Holds a collection of [`AgentView`] implementations.
pub struct AgentViewRegistry {
    views: Vec<Box<dyn AgentView>>,
}

impl AgentViewRegistry {
    /// Create an empty registry.
    #[inline]
    pub fn new() -> Self {
        Self { views: Vec::new() }
    }

    /// Add a view to the registry.
    pub fn push<V>(&mut self, view: V)
    where
        V: AgentView + 'static,
    {
        self.views.push(Box::new(view));
    }

    /// Builder style helper.
    #[inline]
    pub fn with_view<V>(mut self, view: V) -> Self
    where
        V: AgentView + 'static,
    {
        self.push(view);
        self
    }

    /// Show all registered views.
    pub fn show_all(&mut self, ctx: &mut ViewContext<'_>, state: &mut AgentState) {
        for view in &mut self.views {
            view.show(ctx, state);
        }
    }
}

impl Default for AgentViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

mod tool_log_view;
pub use tool_log_view::ToolLogView;
