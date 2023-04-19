use eframe::egui::{Button, CentralPanel, Context, UserAttentionType};
use eframe::{CreationContext, NativeOptions};

use chrono::{DateTime, Duration, Utc};

fn repr(attention: UserAttentionType) -> String {
    format!("{:?}", attention)
}

struct Application {
    attention: UserAttentionType,
    request_at: Option<DateTime<Utc>>,

    auto_reset: bool,
    reset_at: Option<DateTime<Utc>>,
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
        Duration::seconds(3)
    }

    fn attention_request_timeout() -> Duration {
        Duration::seconds(2)
    }

    fn repaint_max_timeout() -> std::time::Duration {
        std::time::Duration::from_secs(1)
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Some(request_at) = self.request_at {
            if request_at < Utc::now() {
                self.request_at = None;
                frame.request_user_attention(self.attention);
                if self.auto_reset {
                    self.auto_reset = false;
                    self.reset_at = Some(Utc::now() + Self::attention_reset_timeout());
                }
            }
        }

        if let Some(reset_at) = self.reset_at {
            if reset_at < Utc::now() {
                self.reset_at = None;
                frame.request_user_attention(UserAttentionType::Reset);
            }
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Attention type:");
                    eframe::egui::ComboBox::new("attention", "")
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

                let enabled = self.request_at.is_none() && self.reset_at.is_none();
                let resp = ui
                    .add_enabled(
                        enabled,
                        Button::new(match enabled {
                            true => format!(
                                "Request in {} seconds",
                                Self::attention_request_timeout().num_seconds()
                            ),
                            false => match self.reset_at {
                                None => "Unfocus the window, fast!".to_owned(),
                                Some(t) => format!(
                                    "Resetting attention in {} s...",
                                    (t - Utc::now()).num_seconds()
                                ),
                            },
                        }),
                    )
                    .on_hover_text_at_pointer(
                        "After clicking, unfocus the application's window to see the effect",
                    );

                ui.checkbox(
                    &mut self.auto_reset,
                    format!(
                        "Reset after {} seconds",
                        Self::attention_reset_timeout().num_seconds()
                    ),
                );

                if resp.clicked() {
                    self.request_at = Some(Utc::now() + Self::attention_request_timeout());
                }
            });
        });

        ctx.request_repaint_after(Self::repaint_max_timeout());
    }
}

fn main() -> eframe::Result<()> {
    let native_options = NativeOptions {
        initial_window_size: Some(eframe::egui::vec2(400., 200.)),
        ..Default::default()
    };
    eframe::run_native(
        "User attention test",
        native_options,
        Box::new(|cc| Box::new(Application::new(cc))),
    )
}
