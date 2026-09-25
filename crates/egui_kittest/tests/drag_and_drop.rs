use egui::{
    Color32, CursorIcon, DragAndDrop, Event, Frame, Id, LayerId, Modifiers, Order, PointerButton,
    Pos2, Rect, Vec2,
};
use egui_kittest::Harness;

struct State {
    text: String,
    clicks: usize,
    button: Rect,
    edit: Rect,
    source: Rect,
    focused: bool,
    enabled: bool,
    source_id: Id,
    edit_id: Id,
    contents_layer: LayerId,
    drop_zone: Rect,
    received: Vec<u32>,
}

fn harness() -> Harness<'static, State> {
    Harness::builder().with_step_dt(0.05).build_ui_state(
        |ui, state| {
            if !state.enabled {
                ui.disable();
            }
            let source = ui.dnd_drag_source(Id::unique("source"), 42_u32, |ui| {
                state.contents_layer = ui.layer_id();
                Frame::new().fill(Color32::RED).show(ui, |ui| {
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
            state.source_id = source.response.id;
            ui.add_space(50.0);
            let (zone, payload) = ui.dnd_drop_zone::<u32, _>(Frame::new(), |ui| {
                ui.allocate_space(Vec2::splat(100.0));
            });
            state.drop_zone = zone.response.rect;
            if let Some(payload) = payload {
                state.received.push(*payload);
            }
        },
        State {
            text: String::new(),
            clicks: 0,
            button: Rect::NOTHING,
            edit: Rect::NOTHING,
            source: Rect::NOTHING,
            focused: false,
            enabled: true,
            source_id: Id::NULL,
            edit_id: Id::NULL,
            contents_layer: LayerId::background(),
            drop_zone: Rect::NOTHING,
            received: Vec::new(),
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

#[test]
fn drag_source_delivers_payload_from_empty_space() {
    let mut harness = harness();
    let source = harness.state().source;
    let edit_id = harness.state().edit_id;
    let start = source.center_top() + Vec2::new(0.0, 10.0);
    let end = harness.state().drop_zone.center();
    assert_eq!(harness.state().source_id, Id::unique("source"));
    harness.hover_at(start);
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::Grab
    );
    harness.drag_at(start);
    harness.run();
    harness.hover_at(end);
    harness.run();
    assert!(harness.ctx.is_being_dragged(Id::unique("source")));
    assert_eq!(
        DragAndDrop::payload::<u32>(&harness.ctx).as_deref(),
        Some(&42)
    );
    assert_eq!(
        harness.state().contents_layer,
        LayerId::new(Order::Tooltip, Id::unique("source"))
    );
    assert_eq!(harness.state().source, source);
    assert_eq!(harness.state().edit_id, edit_id);
    let painted_frame = harness.output().shapes.iter().find_map(|shape| {
        if let egui::epaint::Shape::Rect(rect) = &shape.shape
            && rect.fill == Color32::RED
        {
            Some(rect.rect)
        } else {
            None
        }
    });
    let painted_frame = painted_frame.expect("the dragged frame should be painted");
    assert!(painted_frame.center().distance(end) < 0.01);
    assert_eq!(painted_frame.size(), source.size());
    harness.drop_at(end);
    harness.run();
    assert_eq!(harness.state().received, [42]);
    assert!(!DragAndDrop::has_any_payload(&harness.ctx));
    assert_eq!(harness.state().source_id, Id::unique("source"));
    assert_eq!(harness.state().source, source);
}

#[test]
fn disabled_drag_source_does_not_drag() {
    let mut harness = harness();
    harness.state_mut().enabled = false;
    harness.run();
    let start = harness.state().source.center_top() + Vec2::new(0.0, 10.0);
    harness.hover_at(start);
    harness.run();
    harness.drag_at(start);
    harness.run();
    harness.hover_at(start + Vec2::splat(50.0));
    harness.run();
    assert!(!harness.ctx.is_being_dragged(Id::unique("source")));
    assert!(!DragAndDrop::has_any_payload(&harness.ctx));
}

#[test]
fn drag_source_scroll_into_view_uses_final_geometry() {
    let id = Id::unique("source");
    let mut harness = Harness::new_ui_state(
        |ui, state| {
            let area = egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    ui.add_space(300.0);
                    state.0 = ui
                        .dnd_drag_source(id, (), |ui| {
                            ui.allocate_space(Vec2::splat(50.0));
                        })
                        .response
                        .rect;
                });
            state.1 = area.state.offset;
            state.2 = area.inner_rect;
        },
        (Rect::NOTHING, Vec2::ZERO, Rect::NOTHING),
    );
    assert!(!harness.state().2.intersects(harness.state().0));
    harness.event(Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::ScrollIntoView,
            target_node: id.accesskit_id(),
            target_tree: egui::accesskit::TreeId::ROOT,
            data: None,
        },
    ));
    harness.run();
    let (source, offset, viewport) = *harness.state();
    assert!(offset.is_finite());
    assert!(offset.y > 0.0);
    assert!(viewport.contains_rect(source));
}
