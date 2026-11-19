use egui::{DragAndDrop, Event, Frame, Id, LayerId, Modifiers, PointerButton, Pos2, Rect, Vec2};
use egui_kittest::Harness;

struct State {
    text: String,
    clicks: usize,
    button: Rect,
    edit: Rect,
    source: Rect,
    focused: bool,
    edit_id: Id,
    contents_layer: LayerId,
}

fn harness() -> Harness<'static, State> {
    Harness::builder().with_step_dt(0.05).build_ui_state(
        |ui, state| {
            let source = ui.dnd_drag_source(Id::unique("source"), 42_u32, |ui| {
                state.contents_layer = ui.layer_id();
                Frame::new().show(ui, |ui| {
                    ui.add_space(25.0);
                    let edit = ui.text_edit_singleline(&mut state.text);
                    state.edit = edit.rect;
                    state.edit_id = edit.id;
                    state.focused = edit.has_focus();
                    ui.add_space(25.0);
                    let button = ui.button("Click me");
                    state.button = button.rect;
                    state.clicks += usize::from(button.clicked());
                    ui.add_space(25.0);
                });
            });
            state.source = source.response.rect;
        },
        State {
            text: String::new(),
            clicks: 0,
            button: Rect::NOTHING,
            edit: Rect::NOTHING,
            source: Rect::NOTHING,
            focused: false,
            edit_id: Id::NULL,
            contents_layer: LayerId::background(),
        },
    )
}

fn click(harness: &mut Harness<'_, State>, pos: Pos2) {
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.run();
    }
}

#[test]
fn drag_source_allows_button_clicks() {
    let mut harness = harness();
    let pos = harness.state().button.center();
    click(&mut harness, pos);
    assert_eq!(harness.state().clicks, 1);
    assert!(!DragAndDrop::has_any_payload(&harness.ctx));
}

#[test]
fn pressing_child_button_does_not_start_drag() {
    let mut harness = harness();
    let pos = harness.state().button.center();
    let layer = harness.state().contents_layer;
    let source = harness.state().source;
    harness.hover_at(pos);
    harness.run();
    harness.drag_at(pos);
    // Hold for less than the normal click timeout, without moving the pointer.
    harness.run_steps(2);
    assert!(!harness.ctx.is_being_dragged(Id::unique("source")));
    assert!(!DragAndDrop::has_any_payload(&harness.ctx));
    assert_eq!(harness.state().contents_layer, layer);
    assert_eq!(harness.state().source, source);
    harness.drop_at(pos);
    harness.run();
    assert_eq!(harness.state().clicks, 1);
}

#[test]
fn drag_source_allows_text_editing() {
    let mut harness = harness();
    let pos = harness.state().edit.center();
    click(&mut harness, pos);
    assert!(harness.state().focused);
    harness.event(Event::Text("hello".into()));
    harness.run();
    assert_eq!(harness.state().text, "hello");
    assert!(!DragAndDrop::has_any_payload(&harness.ctx));
}

#[test]
fn drag_source_preserves_child_drag_interaction() {
    let mut harness = harness();
    harness.state_mut().text = "select this text".into();
    harness.run();
    let edit = harness.state().edit;
    let start = edit.left_center() + Vec2::new(5.0, 0.0);
    harness.hover_at(start);
    harness.run();
    harness.drag_at(start);
    harness.run();
    harness.hover_at(edit.right_center() - Vec2::new(5.0, 0.0));
    harness.run();
    assert!(harness.ctx.is_being_dragged(harness.state().edit_id));
    assert!(!DragAndDrop::has_any_payload(&harness.ctx));
    harness.drop_at(edit.right_center());
    harness.run();
    harness.event(Event::Text("replacement".into()));
    harness.run();
    assert!(!harness.state().text.contains("select this text"));
    assert!(harness.state().text.contains("replacement"));
}
