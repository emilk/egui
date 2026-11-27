use egui::{TabBar, Ui};

/// The pages of the first tab bar in [`TabsDemo`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Page {
    Inbox,
    Drafts,
    Sent,
    Archive,
}

/// Showcase [`egui::TabBar`] and [`egui::Tab`].
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TabsDemo {
    page: Page,

    /// How many documents are open in the second tab bar.
    num_documents: usize,

    /// Which of the documents is selected.
    document: usize,
}

impl Default for TabsDemo {
    fn default() -> Self {
        Self {
            page: Page::Inbox,
            num_documents: 8,
            document: 0,
        }
    }
}

impl crate::Demo for TabsDemo {
    fn name(&self) -> &'static str {
        "📑 Tabs"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_width(360.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for TabsDemo {
    fn ui(&mut self, ui: &mut Ui) {
        let Self {
            page,
            num_documents,
            document,
        } = self;

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.label("A tab bar only shows the tabs. You decide what goes below it:");

        TabBar::new().id_salt("pages").show(ui, |ui| {
            ui.tab_value(page, Page::Inbox, "📥 Inbox");
            ui.tab_value(page, Page::Drafts, "📝 Drafts");
            ui.tab_value(page, Page::Sent, "📤 Sent");
            ui.add_enabled_ui(false, |ui| {
                ui.tab_value(page, Page::Archive, "📦 Archive");
            });
        });

        ui.add_space(4.0);
        match page {
            Page::Inbox => ui.label("You have no new messages."),
            Page::Drafts => ui.label("Nothing here yet. Start writing!"),
            Page::Sent => ui.label("You have not sent anything."),
            Page::Archive => ui.label("The archive is disabled, so you should not see this."),
        };

        ui.add_space(12.0);
        ui.label(
            "When the tabs do not fit, the bar scrolls. \
             Click a tab and use the arrow keys to walk through them:",
        );

        TabBar::new().id_salt("documents").show(ui, |ui| {
            for i in 0..*num_documents {
                ui.tab_value(document, i, format!("Document {}", i + 1));
            }

            // Any widget can go in the bar:
            if ui
                .small_button("➕")
                .on_hover_text("New document")
                .clicked()
            {
                *document = *num_documents;
                *num_documents += 1;
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(format!("Document {} is empty.", *document + 1));
            if ui
                .add_enabled(1 < *num_documents, egui::Button::new("Close it"))
                .clicked()
            {
                *num_documents -= 1;
                *document = (*document).min(*num_documents - 1);
            }
        });
    }
}
