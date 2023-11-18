use eframe::{egui, CreationContext, NativeOptions};
use egui::{Button, CentralPanel, Context, UserAttentionType};

use std::time::{Duration, SystemTime};

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400., 200.]),
        ..Default::default()
    };
    eframe::run_native(
        "User attention test",
        native_options,
        Box::new(|cc| Box::new(Application::new(cc))),
    )
}

fn repr(attention: UserAttentionType) -> String {
    format!("{attention:?}")
}

struct Application {
    attention: UserAttentionType,
    request_at: Option<SystemTime>,

    auto_reset: bool,
    reset_at: Option<SystemTime>,
}

impl Application {
    fn new(_cc: &CreationContext<'_>) -> Self {
        Self {
            attention: UserAttentionType::Informational,
            request_at: None,
            auto_reset: false,
            reset_at: None,
        }
    }

    fn attention_reset_timeout() -> Duration {
        Duration::from_secs(3)
    }

    fn attention_request_timeout() -> Duration {
        Duration::from_secs(2)
    }

    fn repaint_max_timeout() -> Duration {
        Duration::from_secs(1)
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(request_at) = self.request_at {
            if request_at < SystemTime::now() {
                self.request_at = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(self.attention));
                if self.auto_reset {
                    self.auto_reset = false;
                    self.reset_at = Some(SystemTime::now() + Self::attention_reset_timeout());
                }
            }
        }

        if let Some(reset_at) = self.reset_at {
            if reset_at < SystemTime::now() {
                self.reset_at = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    UserAttentionType::Reset,
                ));
            }
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Attention type:");
                    egui::ComboBox::new("attention", "")
                        .selected_text(repr(self.attention))
                        .show_ui(ui, |ui| {
                            for kind in [
                                UserAttentionType::Informational,
                                UserAttentionType::Critical,
                            ] {
                                ui.selectable_value(&mut self.attention, kind, repr(kind));
                            }
                        })
                });

                let button_enabled = self.request_at.is_none() && self.reset_at.is_none();
                let button_text = if button_enabled {
                    format!(
                        "Request in {} seconds",
                        Self::attention_request_timeout().as_secs()
                    )
                } else {
                    match self.reset_at {
                        None => "Unfocus the window, fast!".to_owned(),
                        Some(t) => {
                            if let Ok(elapsed) = t.duration_since(SystemTime::now()) {
                                format!("Resetting attention in {} s...", elapsed.as_secs())
                            } else {
                                "Resetting attention...".to_owned()
                            }
                        }
                    }
                };

                let resp = ui
                    .add_enabled(button_enabled, Button::new(button_text))
                    .on_hover_text_at_pointer(
                        "After clicking, unfocus the application's window to see the effect",
                    );

                ui.checkbox(
                    &mut self.auto_reset,
                    format!(
                        "Reset after {} seconds",
                        Self::attention_reset_timeout().as_secs()
                    ),
                );

                if resp.clicked() {
                    self.request_at = Some(SystemTime::now() + Self::attention_request_timeout());
                }
            });
        });

        ctx.request_repaint_after(Self::repaint_max_timeout());
    }
}
