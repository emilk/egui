use std::sync::Arc;

use eframe::egui::{self, CentralPanel, Color32, Frame as EguiFrame};
use eframe_agent::views::ViewContext;
use eframe_agent::{
    AgentCommand, AgentRuntime, AgentState, AgentTaskState, AgentView, MessageRole, TaskStatus,
};

/// Conversation-centric view used by the agent demo.
#[derive(Default)]
pub struct ConversationView;

impl AgentView for ConversationView {
    fn id(&self) -> &'static str {
        "conversation_view"
    }

    fn show(&mut self, ctx: &mut ViewContext<'_>, state: &mut AgentState) {
        let runtime = Arc::clone(ctx.runtime);

        CentralPanel::default()
            .frame(EguiFrame::default())
            .show_inside(ctx.ui, |ui| {
                ui.heading("GUI Agent");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("agent_messages")
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for message in &state.messages {
                            let color = match message.role {
                                MessageRole::User => Color32::from_rgb(0x8a, 0xc2, 0xff),
                                MessageRole::Agent => Color32::from_rgb(0xc1, 0xff, 0xc1),
                                MessageRole::System => Color32::from_rgb(0xff, 0xe0, 0x84),
                            };

                            ui.colored_label(
                                color,
                                format!("{}: {}", label_for_role(message.role), message.text),
                            );
                        }
                    });

                ui.separator();
                ui.heading("Tasks");
                ui.horizontal_wrapped(|ui| {
                    for task in &state.tasks {
                        draw_task_chip(ui, task);
                    }
                    if state.tasks.is_empty() {
                        ui.weak("No tasks yet");
                    }
                });

                if !state.command_palette_open {
                    ui.separator();
                    ui.heading("Prompt");
                    ui.horizontal(|ui| {
                        ui.label("Prompt:");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut state.draft_prompt)
                                .id(egui::Id::new("prompt_input"))
                                .hint_text("Type a message..."),
                        );
                        set_author_id(ui.ctx(), &response, "prompt_input");
                        let send_button = ui.button("Send");
                        set_author_id(ui.ctx(), &send_button, "prompt_send");
                        let send_clicked = send_button.clicked();
                        let submitted = response.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                            && !state.draft_prompt.trim().is_empty();

                        if submitted || send_clicked {
                            submit_prompt_and_reset(&runtime, state);
                        }
                    });
                }

                if state.command_palette_open {
                    let mut palette_open = state.command_palette_open;
                    egui::Window::new("Command palette")
                        .open(&mut palette_open)
                        .id(egui::Id::new("agent_command_palette"))
                        .show(ui.ctx(), |ui| {
                            ui.label("Enter prompt:");
                            let text_edit = egui::TextEdit::singleline(&mut state.draft_prompt)
                                .hint_text("Ask the agent...");
                            let response = ui.add(text_edit);
                            set_author_id(ui.ctx(), &response, "command_palette_input");

                            let submitted = response.lost_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter))
                                && !state.draft_prompt.trim().is_empty();

                            let send_button = ui.button("Send");
                            set_author_id(ui.ctx(), &send_button, "command_palette_send");
                            if submitted || send_button.clicked() {
                                submit_prompt_and_reset(&runtime, state);
                            }
                        });
                    state.command_palette_open = palette_open;
                } else {
                    ui.horizontal(|ui| {
                        let command_palette = ui.button("Command Palette (Cmd+K)");
                        set_author_id(ui.ctx(), &command_palette, "command_palette_toggle");
                        if command_palette.clicked() {
                            state.command_palette_open = true;
                        }
                        let clear_button = ui.button("Clear");
                        set_author_id(ui.ctx(), &clear_button, "clear_history");
                        if clear_button.clicked() {
                            state.reset();
                            runtime.submit_command(AgentCommand::ClearHistory);
                        }
                        let cancel_button = ui.button("Cancel");
                        set_author_id(ui.ctx(), &cancel_button, "cancel_task");
                        if cancel_button.clicked() {
                            runtime.submit_command(AgentCommand::CancelActiveTask);
                        }
                    });
                }
            });
    }
}

fn set_author_id(ctx: &egui::Context, response: &egui::Response, author_id: &str) {
    ctx.accesskit_node_builder(response.id, |builder| {
        builder.set_author_id(author_id);
    });
}

fn draw_task_chip(ui: &mut egui::Ui, task: &AgentTaskState) {
    let color = match task.status {
        TaskStatus::Pending => Color32::from_rgb(0xd8, 0xd8, 0xd8),
        TaskStatus::Running => Color32::from_rgb(0xff, 0xc1, 0x5f),
        TaskStatus::Completed => Color32::from_rgb(0x9a, 0xe5, 0xa8),
        TaskStatus::Failed => Color32::from_rgb(0xff, 0x82, 0x82),
    };

    ui.group(|ui| {
        ui.colored_label(color, format!("#{:<3} {}", task.id, task.label));
        ui.weak(format!("{:?}", task.status));
    });
}

fn label_for_role(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "You",
        MessageRole::Agent => "Agent",
        MessageRole::System => "System",
    }
}

fn submit_prompt_and_reset(runtime: &Arc<dyn AgentRuntime>, state: &mut AgentState) {
    if state.draft_prompt.trim().is_empty() {
        return;
    }

    let prompt = std::mem::take(&mut state.draft_prompt);
    runtime.submit_command(AgentCommand::SubmitPrompt(prompt));
    state.command_palette_open = false;
}
