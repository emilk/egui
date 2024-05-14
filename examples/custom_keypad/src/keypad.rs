use eframe::egui::{self, pos2, vec2, Button, Ui, Vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Transition {
    #[default]
    None,
    CloseOnNextFrame,
    CloseImmediately,
}

#[derive(Clone, Debug)]
struct State {
    open: bool,
    closable: bool,
    close_on_next_frame: bool,
    start_pos: egui::Pos2,
    focus: Option<egui::Id>,
    events: Option<Vec<egui::Event>>,
}

impl State {
    fn new() -> Self {
        Self {
            open: false,
            closable: false,
            close_on_next_frame: false,
            start_pos: pos2(100.0, 100.0),
            focus: None,
            events: None,
        }
    }

    fn queue_char(&mut self, c: char) {
        let events = self.events.get_or_insert(vec![]);
        if let Some(key) = egui::Key::from_name(&c.to_string()) {
            events.push(egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            });
        }
        events.push(egui::Event::Text(c.to_string()));
    }

    fn queue_key(&mut self, key: egui::Key) {
        let events = self.events.get_or_insert(vec![]);
        events.push(egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        });
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple keypad widget.
pub struct Keypad {
    id: egui::Id,
}

impl Keypad {
    pub fn new() -> Self {
        Self {
            id: egui::Id::new("keypad"),
        }
    }

    pub fn bump_events(&self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let events = ctx.memory_mut(|m| {
            m.data
                .get_temp_mut_or_default::<State>(self.id)
                .events
                .take()
        });
        if let Some(mut events) = events {
            events.append(&mut raw_input.events);
            raw_input.events = events;
        }
    }

    fn buttons(ui: &mut Ui, state: &mut State) -> Transition {
        let mut trans = Transition::None;
        ui.vertical(|ui| {
            let window_margin = ui.spacing().window_margin;
            let size_1x1 = vec2(32.0, 26.0);
            let _size_1x2 = vec2(32.0, 52.0 + window_margin.top);
            let _size_2x1 = vec2(64.0 + window_margin.left, 26.0);

            ui.spacing_mut().item_spacing = Vec2::splat(window_margin.left);

            ui.horizontal(|ui| {
                if ui.add_sized(size_1x1, Button::new("1")).clicked() {
                    state.queue_char('1');
                }
                if ui.add_sized(size_1x1, Button::new("2")).clicked() {
                    state.queue_char('2');
                }
                if ui.add_sized(size_1x1, Button::new("3")).clicked() {
                    state.queue_char('3');
                }
                if ui.add_sized(size_1x1, Button::new("⏮")).clicked() {
                    state.queue_key(egui::Key::Home);
                }
                if ui.add_sized(size_1x1, Button::new("🔙")).clicked() {
                    state.queue_key(egui::Key::Backspace);
                }
            });
            ui.horizontal(|ui| {
                if ui.add_sized(size_1x1, Button::new("4")).clicked() {
                    state.queue_char('4');
                }
                if ui.add_sized(size_1x1, Button::new("5")).clicked() {
                    state.queue_char('5');
                }
                if ui.add_sized(size_1x1, Button::new("6")).clicked() {
                    state.queue_char('6');
                }
                if ui.add_sized(size_1x1, Button::new("⏭")).clicked() {
                    state.queue_key(egui::Key::End);
                }
                if ui.add_sized(size_1x1, Button::new("⎆")).clicked() {
                    state.queue_key(egui::Key::Enter);
                    trans = Transition::CloseOnNextFrame;
                }
            });
            ui.horizontal(|ui| {
                if ui.add_sized(size_1x1, Button::new("7")).clicked() {
                    state.queue_char('7');
                }
                if ui.add_sized(size_1x1, Button::new("8")).clicked() {
                    state.queue_char('8');
                }
                if ui.add_sized(size_1x1, Button::new("9")).clicked() {
                    state.queue_char('9');
                }
                if ui.add_sized(size_1x1, Button::new("⏶")).clicked() {
                    state.queue_key(egui::Key::ArrowUp);
                }
                if ui.add_sized(size_1x1, Button::new("⌨")).clicked() {
                    trans = Transition::CloseImmediately;
                }
            });
            ui.horizontal(|ui| {
                if ui.add_sized(size_1x1, Button::new("0")).clicked() {
                    state.queue_char('0');
                }
                if ui.add_sized(size_1x1, Button::new(".")).clicked() {
                    state.queue_char('.');
                }
                if ui.add_sized(size_1x1, Button::new("⏴")).clicked() {
                    state.queue_key(egui::Key::ArrowLeft);
                }
                if ui.add_sized(size_1x1, Button::new("⏷")).clicked() {
                    state.queue_key(egui::Key::ArrowDown);
                }
                if ui.add_sized(size_1x1, Button::new("⏵")).clicked() {
                    state.queue_key(egui::Key::ArrowRight);
                }
            });
        });

        trans
    }

    pub fn show(&self, ctx: &egui::Context) {
        let (focus, mut state) = ctx.memory(|m| {
            (
                m.focused(),
                m.data.get_temp::<State>(self.id).unwrap_or_default(),
            )
        });

        let mut is_first_show = false;
        if ctx.wants_keyboard_input() && state.focus != focus {
            let y = ctx.style().spacing.interact_size.y * 1.25;
            state.open = true;
            state.start_pos = ctx.input(|i| {
                i.pointer
                    .hover_pos()
                    .map_or(pos2(100.0, 100.0), |p| p + vec2(0.0, y))
            });
            state.focus = focus;
            is_first_show = true;
        }

        if state.close_on_next_frame {
            state.open = false;
            state.close_on_next_frame = false;
            state.focus = None;
        }

        let mut open = state.open;

        let win = egui::Window::new("⌨ Keypad");
        let win = if is_first_show {
            win.current_pos(state.start_pos)
        } else {
            win.default_pos(state.start_pos)
        };
        let resp = win
            .movable(true)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| Self::buttons(ui, &mut state));

        state.open = open;

        if let Some(resp) = resp {
            match resp.inner {
                Some(Transition::CloseOnNextFrame) => {
                    state.close_on_next_frame = true;
                }
                Some(Transition::CloseImmediately) => {
                    state.open = false;
                    state.focus = None;
                }
                _ => {}
            }
            if !state.closable && resp.response.hovered() {
                state.closable = true;
            }
            if state.closable && resp.response.clicked_elsewhere() {
                state.open = false;
                state.closable = false;
                state.focus = None;
            }
            if is_first_show {
                ctx.move_to_top(resp.response.layer_id);
            }
        }

        if let (true, Some(focus)) = (state.open, state.focus) {
            ctx.memory_mut(|m| {
                m.request_focus(focus);
            });
        }

        ctx.memory_mut(|m| m.data.insert_temp(self.id, state));
    }
}

impl Default for Keypad {
    fn default() -> Self {
        Self::new()
    }
}
