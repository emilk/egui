use egui::{
    Button, Color32, Context, DragAndDrop, Event, Frame, Modifiers, PointerButton, RawInput, Rect,
    Stroke, Ui, epaint::ClippedShape, pos2, vec2,
};

#[derive(Debug, PartialEq)]
struct Output {
    shapes: Vec<ClippedShape>,
    content: Rect,
    zone: Rect,
    sibling: Rect,
    payload: Option<u32>,
}

fn show(ctx: &Context, frame: Frame, drop_zone: bool, events: Vec<Event>) -> Output {
    let mut result = Output {
        shapes: Vec::new(),
        content: Rect::NOTHING,
        zone: Rect::NOTHING,
        sibling: Rect::NOTHING,
        payload: None,
    };
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))),
            events,
            ..Default::default()
        },
        |ui| {
            let contents = |ui: &mut Ui| {
                ui.add_sized(vec2(40.0, 20.0), Button::new(""));
                ui.min_rect()
            };
            let response = if drop_zone {
                let (response, payload) = ui.dnd_drop_zone::<u32, _>(frame, contents);
                result.payload = payload.as_deref().copied();
                response
            } else {
                frame.show(ui, contents)
            };
            result.content = response.inner;
            result.zone = response.response.rect;
            result.sibling = ui.add_sized(vec2(40.0, 20.0), Button::new("")).rect;
        },
    );
    result.shapes = output.shapes.clone();
    output.drop_without_applying_deltas();
    result
}

fn custom_frame() -> Frame {
    Frame::NONE
        .inner_margin(8)
        .outer_margin(3)
        .fill(Color32::RED)
        .stroke(Stroke::new(3.0, Color32::GREEN))
        .corner_radius(7)
        .shadow(egui::epaint::Shadow {
            offset: [2, 3],
            blur: 4,
            spread: 1,
            color: Color32::BLACK,
        })
}

#[test]
fn idle_drop_zone_preserves_frame_and_contents() {
    for frame in [Frame::NONE, custom_frame()] {
        let plain = Context::default();
        let drop_zone = Context::default();
        for _ in 0..3 {
            assert_eq!(
                show(&plain, frame, false, vec![]),
                show(&drop_zone, frame, true, vec![]),
            );
        }
    }
}

#[test]
fn drop_zone_drag_feedback_and_release() {
    for accepted in [true, false] {
        let ctx = Context::default();
        let frame = custom_frame();
        let idle = show(&ctx, frame, true, vec![]);
        let inside = idle.zone.min + vec2(4.0, 4.0);
        let outside = pos2(300.0, 200.0);
        if accepted {
            DragAndDrop::set_payload(&ctx, 42_u32);
        } else {
            DragAndDrop::set_payload(&ctx, "wrong type");
        }
        for pointer in [outside, inside, outside, inside] {
            let output = show(&ctx, frame, true, vec![Event::PointerMoved(pointer)]);
            let visuals = ctx.style_of(ctx.theme()).visuals.clone();
            let style = if accepted && pointer == inside {
                visuals.widgets.active
            } else {
                visuals.widgets.inactive
            };
            let mut expected = frame.fill(style.bg_fill).stroke(style.bg_stroke);
            if !accepted {
                expected.fill = visuals.disable(expected.fill);
                expected.stroke.color = visuals.disable(expected.stroke.color);
            }
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| s.shape == expected.paint(output.content))
            );
            assert_eq!(output.content, idle.content);
            assert_eq!(output.zone, idle.zone);
            assert_eq!(output.sibling, idle.sibling);
            assert_eq!(output.payload, None);
        }
        let release = show(
            &ctx,
            frame,
            true,
            vec![Event::PointerButton {
                pos: inside,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert_eq!(release.payload, accepted.then_some(42));
        assert!(!DragAndDrop::has_any_payload(&ctx));
        let restored = show(&ctx, frame, true, vec![Event::PointerGone]);
        assert_eq!(restored, idle);
    }
}
