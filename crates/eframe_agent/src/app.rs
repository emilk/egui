use std::sync::Arc;

use eframe::{App, CreationContext, Frame, Storage, egui};

#[cfg(feature = "jsonl")]
use crate::automation::AutomationBridge;
use crate::{
    bridge::{load_state_from_storage, save_state_to_storage},
    input::{AgentInputAdapter, InputAction},
    runtime::{AgentCommand, AgentRuntime, AgentUpdate},
    state::AgentState,
    views::{AgentViewRegistry, ViewContext},
};

/// Builder for [`AgentApp`].
pub struct AgentAppBuilder {
    runtime: Arc<dyn AgentRuntime>,
    state: Option<AgentState>,
    views: Option<AgentViewRegistry>,
    input_adapter: AgentInputAdapter,
    #[cfg(feature = "jsonl")]
    automation: Option<AutomationBridge>,
}

impl AgentAppBuilder {
    /// Create a builder for `runtime`.
    #[inline]
    pub fn new(runtime: Arc<dyn AgentRuntime>) -> Self {
        Self {
            runtime,
            state: None,
            views: None,
            input_adapter: AgentInputAdapter::default(),
            #[cfg(feature = "jsonl")]
            automation: None,
        }
    }

    /// Populate state using `CreationContext`.
    #[inline]
    pub fn with_creation_context(mut self, cc: &CreationContext<'_>) -> Self {
        self.state = load_state_from_storage(cc.storage).or_else(|| Some(AgentState::default()));
        self
    }

    /// Override state.
    #[inline]
    pub fn with_state(mut self, state: AgentState) -> Self {
        self.state = Some(state);
        self
    }

    /// Override the view registry.
    #[inline]
    pub fn with_views(mut self, views: AgentViewRegistry) -> Self {
        self.views = Some(views);
        self
    }

    /// Override the input adapter.
    #[inline]
    pub fn with_input_adapter(mut self, adapter: AgentInputAdapter) -> Self {
        self.input_adapter = adapter;
        self
    }

    /// Attach a live automation bridge (AccessKit-based).
    #[cfg(feature = "jsonl")]
    #[inline]
    pub fn with_automation_bridge(mut self, automation: AutomationBridge) -> Self {
        self.automation = Some(automation);
        self
    }

    /// Build the [`AgentApp`].
    pub fn build(self) -> AgentApp {
        AgentApp {
            runtime: self.runtime,
            state: self.state.unwrap_or_default(),
            views: self.views.unwrap_or_else(default_views),
            input_adapter: self.input_adapter,
            pending_actions: Vec::new(),
            runtime_updates: Vec::new(),
            #[cfg(feature = "jsonl")]
            automation: self.automation,
        }
    }
}

fn default_views() -> AgentViewRegistry {
    AgentViewRegistry::new()
}

/// Glue struct that implements [`eframe::App`].
pub struct AgentApp {
    runtime: Arc<dyn AgentRuntime>,
    state: AgentState,
    views: AgentViewRegistry,
    input_adapter: AgentInputAdapter,
    pending_actions: Vec<InputAction>,
    runtime_updates: Vec<AgentUpdate>,
    #[cfg(feature = "jsonl")]
    automation: Option<AutomationBridge>,
}

impl AgentApp {
    /// Construct from builder.
    pub fn builder(runtime: Arc<dyn AgentRuntime>) -> AgentAppBuilder {
        AgentAppBuilder::new(runtime)
    }

    /// Access state.
    pub fn state(&self) -> &AgentState {
        &self.state
    }

    /// Mutable state reference.
    pub fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }

    fn apply_input_actions(&mut self) {
        for action in self.pending_actions.drain(..) {
            match action {
                InputAction::ToggleCommandPalette => self.state.toggle_command_palette(),
                InputAction::ClearHistory => {
                    self.state.reset();
                    self.runtime.submit_command(AgentCommand::ClearHistory);
                }
                InputAction::CancelActiveTask => {
                    self.runtime.submit_command(AgentCommand::CancelActiveTask);
                }
            }
        }
    }

    fn poll_runtime_updates(&mut self, ctx: &egui::Context) -> bool {
        #[cfg(feature = "jsonl")]
        if let Some(automation) = &self.automation {
            ctx.enable_accesskit();
            automation.register_accesskit_plugin(ctx);
        }
        self.runtime.poll_updates(&mut self.runtime_updates);
        if self.runtime_updates.is_empty() {
            return false;
        }

        for update in self.runtime_updates.drain(..) {
            match update {
                AgentUpdate::Control { action } => match action {
                    crate::runtime::ControlAction::CloseWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                },
                other => self.state.record_update(other),
            }
        }
        true
    }
}

impl App for AgentApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let _had_updates = self.poll_runtime_updates(ctx);
        self.apply_input_actions();
        #[cfg(feature = "mcp_sse")]
        ctx.request_repaint();
        #[cfg(not(feature = "mcp_sse"))]
        if _had_updates {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame) {
        let mut view_ctx = ViewContext {
            ui,
            frame,
            runtime: &self.runtime,
        };
        self.views.show_all(&mut view_ctx, &mut self.state);
        #[cfg(feature = "jsonl")]
        if let Some(automation) = &self.automation {
            for request_id in automation.drain_screenshot_requests() {
                view_ctx
                    .ui
                    .ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        request_id,
                    )));
            }
            automation.update_from_ui(view_ctx.ui, &self.state);
        }
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        #[cfg(feature = "jsonl")]
        let injected = if let Some(automation) = &self.automation {
            automation.capture_screenshot_events(raw_input);
            automation.inject_raw_input(raw_input)
        } else {
            0
        };
        self.input_adapter.process(raw_input);
        self.input_adapter.drain_actions(&mut self.pending_actions);
        if !self.pending_actions.is_empty() {
            ctx.request_repaint();
        }
        #[cfg(feature = "jsonl")]
        if injected > 0 {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        save_state_to_storage(storage, &self.state);
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
