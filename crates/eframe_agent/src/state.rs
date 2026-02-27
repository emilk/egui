use serde::{Deserialize, Serialize};

use crate::runtime::{AgentUpdate, MessageRole};

/// Message shown in the conversation timeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AgentMessage {
    /// Author of the message.
    pub role: MessageRole,

    /// Message body.
    pub text: String,

    /// Timestamp in unix seconds.
    pub timestamp_secs: f64,
}

/// Tracks an in-flight task.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AgentTaskState {
    /// Identifier assigned by the runtime.
    pub id: u64,

    /// Friendly description.
    pub label: String,

    /// Current status.
    pub status: TaskStatus,
}

/// Status for a tracked task.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TaskStatus {
    /// Waiting to run or not yet acknowledged.
    #[default]
    Pending,

    /// Currently running.
    Running,

    /// Completed successfully.
    Completed,

    /// Completed with an error or cancellation.
    Failed,
}

/// Reactive state shared between UI and runtime.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentState {
    /// Conversation history.
    pub messages: Vec<AgentMessage>,

    /// Tasks published by the runtime.
    pub tasks: Vec<AgentTaskState>,

    /// Should the command palette be visible.
    pub command_palette_open: bool,

    /// Text currently typed by the user.
    pub draft_prompt: String,

    /// Tool log entries (for debugging/verification).
    #[serde(default)]
    pub ui_log: Vec<String>,
}

impl AgentState {
    const MAX_UI_LOG_ENTRIES: usize = 200;

    /// Toggle the visibility of the command palette.
    pub fn toggle_command_palette(&mut self) {
        self.command_palette_open = !self.command_palette_open;
    }

    /// Handle a runtime update.
    pub fn record_update(&mut self, update: AgentUpdate) {
        match update {
            AgentUpdate::Message { role, text } => self.push_message(role, text),
            AgentUpdate::TaskStarted { id, label } => self.mark_task_running(id, label),
            AgentUpdate::TaskFinished { id, label, success } => {
                self.mark_task_finished(id, label, success)
            }
            AgentUpdate::Reset => self.reset(),
            AgentUpdate::UiLog { text } => self.push_ui_log(text),
            AgentUpdate::Control { .. } => {}
        }
    }

    /// Clear all state.
    pub fn reset(&mut self) {
        self.messages.clear();
        self.tasks.clear();
        self.ui_log.clear();
    }

    fn push_message(&mut self, role: MessageRole, text: String) {
        self.messages.push(AgentMessage {
            role,
            text,
            timestamp_secs: current_time_secs(),
        });
    }

    fn push_ui_log(&mut self, text: String) {
        self.ui_log.push(text);
        if self.ui_log.len() > Self::MAX_UI_LOG_ENTRIES {
            let overflow = self.ui_log.len() - Self::MAX_UI_LOG_ENTRIES;
            self.ui_log.drain(0..overflow);
        }
    }

    fn mark_task_running(&mut self, id: u64, label: String) {
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            task.status = TaskStatus::Running;
            task.label = label;
            return;
        }

        self.tasks.push(AgentTaskState {
            id,
            label,
            status: TaskStatus::Running,
        });
    }

    fn mark_task_finished(&mut self, id: u64, label: String, success: bool) {
        let status = if success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };

        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            task.status = status;
            task.label = label;
            return;
        }

        self.tasks.push(AgentTaskState { id, label, status });
    }
}

fn current_time_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs_f64())
        .unwrap_or_default()
}
