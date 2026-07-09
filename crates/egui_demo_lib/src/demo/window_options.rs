use egui::{UiKind, Vec2b, WindowDrag};

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowOptions {
    title: String,
    title_bar: bool,
    closable: bool,
    collapsible: bool,
    movable: bool,
    resizable: bool,
    constrain: bool,
    scroll2: Vec2b,
    disabled_time: f64,

    anchored: bool,
    anchor: egui::Align2,
    anchor_offset: egui::Vec2,

    drag_area: WindowDrag,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "🗖 Window Options".to_owned(),
            title_bar: true,
            closable: true,
            collapsible: true,
            movable: true,
            resizable: true,
            constrain: true,
            scroll2: Vec2b::TRUE,
            disabled_time: f64::NEG_INFINITY,
            anchored: false,
            anchor: egui::Align2::RIGHT_TOP,
            anchor_offset: egui::Vec2::ZERO,
            drag_area: WindowDrag::default(),
        }
    }
}

impl crate::Demo for WindowOptions {
    fn name(&self) -> &'static str {
        "🗖 Window Options"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        let Self {
            title,
            title_bar,
            closable,
            collapsible,
            movable,
            resizable,
            constrain,
            scroll2,
            disabled_time,
            anchored,
            anchor,
            anchor_offset,
            drag_area,
        } = self.clone();

        let enabled = ui.input(|i| i.time) - disabled_time > 2.0;
        if !enabled {
            ui.request_repaint();
        }

        use crate::View as _;
        let mut window = egui::Window::new(title)
            .id(egui::Id::new("demo_window_options")) // required since we change the title
            .resizable(resizable)
            .constrain(constrain)
            .collapsible(collapsible)
            .movable(movable)
            .title_bar(title_bar)
            .drag_area(drag_area)
            .scroll(scroll2)
            .constrain_to(ui.available_rect_before_wrap())
            .enabled(enabled);
        if closable {
            window = window.open(open);
        }
        if anchored {
            window = window.anchor(anchor, anchor_offset);
        }
        window.show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for WindowOptions {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            title,
            title_bar,
            closable,
            collapsible,
            movable,
            resizable,
            constrain,
            scroll2,
            disabled_time: _,
            anchored,
            anchor,
            anchor_offset,
            drag_area,
        } = self;
        ui.horizontal(|ui| {
            ui.label("title:");
            ui.text_edit_singleline(title);
        });

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.checkbox(title_bar, "title_bar");
                    ui.checkbox(closable, "closable");
                    ui.checkbox(collapsible, "collapsible");
                    ui.checkbox(movable, "movable")
                        .on_hover_text("Can the window be moved by dragging?");
                    ui.checkbox(resizable, "resizable");
                    ui.checkbox(constrain, "constrain")
                        .on_hover_text("Constrain window to the screen");
                    ui.checkbox(&mut scroll2[0], "hscroll");
                    ui.checkbox(&mut scroll2[1], "vscroll");
                });
            });
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.checkbox(anchored, "anchored");
                    if !*anchored {
                        ui.disable();
                    }
                    ui.horizontal(|ui| {
                        ui.label("x:");
                        ui.selectable_value(&mut anchor[0], egui::Align::LEFT, "Left");
                        ui.selectable_value(&mut anchor[0], egui::Align::Center, "Center");
                        ui.selectable_value(&mut anchor[0], egui::Align::RIGHT, "Right");
                    });
                    ui.horizontal(|ui| {
                        ui.label("y:");
                        ui.selectable_value(&mut anchor[1], egui::Align::TOP, "Top");
                        ui.selectable_value(&mut anchor[1], egui::Align::Center, "Center");
                        ui.selectable_value(&mut anchor[1], egui::Align::BOTTOM, "Bottom");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Offset:");
                        ui.add(egui::DragValue::new(&mut anchor_offset.x));
                        ui.add(egui::DragValue::new(&mut anchor_offset.y));
                    });
                });
            });
        });

        ui.horizontal(|ui| {
            ui.label("Drag to move:")
                .on_hover_text("Where the user can grab the window to move it");
            ui.selectable_value(drag_area, WindowDrag::Off, "Off")
                .on_hover_text("The window cannot be dragged to move it (same as movable = false)");
            ui.selectable_value(drag_area, WindowDrag::OnTouch, "OnTouch")
                .on_hover_text("Anywhere on touch screens, title-bar only otherwise (default)");
            ui.selectable_value(drag_area, WindowDrag::TitleBar, "TitleBar")
                .on_hover_text("Only the title bar moves the window");
            ui.selectable_value(drag_area, WindowDrag::Anywhere, "Anywhere")
                .on_hover_text("Drag anywhere on the window to move it");
        });

        ui.separator();
        let on_top = Some(ui.layer_id()) == ui.ctx().top_layer_id();
        ui.label(format!("This window is on top: {on_top}."));

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Disable for 2 seconds").clicked() {
                self.disabled_time = ui.input(|i| i.time);
            }
            egui::reset_button(ui, self, "Reset");
            if ui
                .button("Close")
                .on_hover_text("You can collapse / close Windows via Ui::close")
                .clicked()
            {
                // Calling close would close the collapsible within the window
                // ui.close();
                // Instead, we close the window itself
                ui.close_kind(UiKind::Window);
            }
            ui.add(crate::egui_github_link_file!());
        });
    }
}
