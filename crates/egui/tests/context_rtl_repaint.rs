//! Direction and theme preference changes must repaint all viewports, and must
//! stay silent when nothing changes: an idle app (no input events) learns about
//! a direction or theme flip only through a repaint request, and apps that call
//! these setters every frame must not create a repaint loop.

use std::sync::Arc;

use egui::{
    Context, FullOutput, RawInput, Rect, RequestRepaintInfo, Theme, ThemePreference, Ui, Vec2,
    mutex::Mutex,
};

fn input(time: f64) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(
            Default::default(),
            Vec2::new(800.0, 600.0),
        )),
        time: Some(time),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, time: f64, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(time), run_ui);
    output.textures_delta.clear();
    output
}

type Log = Arc<Mutex<Vec<RequestRepaintInfo>>>;

fn watch(ctx: &Context) -> Log {
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    ctx.set_request_repaint_callback(move |info| {
        sink.lock().push(info);
    });
    log
}

fn recorded(log: &Log) -> Vec<RequestRepaintInfo> {
    log.lock().clone()
}

fn clear(log: &Log) {
    log.lock().clear();
}

/// A direction change outside any pass must wake up the integration: the
/// callback sees an immediate repaint request.
#[test]
fn set_right_to_left_requests_an_immediate_repaint() {
    let ctx = Context::default();
    let log = watch(&ctx);

    clear(&log);
    ctx.set_right_to_left(true);
    let logged = recorded(&log);
    assert!(
        logged.iter().any(|info| info.delay.is_zero()),
        "a direction change outside any pass must request an immediate repaint: {logged:?}"
    );
}

/// The actual change repaints; setting the same value again requests nothing.
#[test]
fn set_right_to_left_with_the_same_value_requests_no_repaint() {
    let ctx = Context::default();
    let log = watch(&ctx);

    ctx.set_right_to_left(true); // the actual change
    let logged = recorded(&log);
    assert!(
        !logged.is_empty(),
        "the change itself must repaint: {logged:?}"
    );

    clear(&log);
    ctx.set_right_to_left(true); // no change
    assert!(
        recorded(&log).is_empty(),
        "setting the same direction again must not repaint"
    );
}

/// A theme preference change outside any pass must wake up the integration.
#[test]
fn set_theme_requests_a_repaint_when_the_preference_changes() {
    let ctx = Context::default();
    let log = watch(&ctx);

    clear(&log);
    ctx.set_theme(Theme::Light);
    let logged = recorded(&log);
    assert!(
        logged.iter().any(|info| info.delay.is_zero()),
        "a theme preference change outside any pass must request an immediate repaint: \
         {logged:?}"
    );
}

/// Setting the same theme preference again (in either form) requests nothing.
#[test]
fn set_theme_with_the_same_value_requests_no_repaint() {
    let ctx = Context::default();
    ctx.set_theme(Theme::Light);
    let log = watch(&ctx);

    clear(&log);
    ctx.set_theme(Theme::Light); // no change
    assert!(
        recorded(&log).is_empty(),
        "setting the same theme preference again must not repaint"
    );

    clear(&log);
    ctx.set_theme(ThemePreference::Light); // same preference, other form
    assert!(
        recorded(&log).is_empty(),
        "setting the same theme preference via Into must not repaint"
    );
}

/// Per-frame callers (the i18n demo-app pattern) write the same value every
/// frame: after the one actual change, no frame may ever see a repaint request
/// from the setter, and the recorded preference stays exactly as set.
#[test]
fn set_right_to_left_called_every_frame_is_a_no_op() {
    let ctx = Context::default();
    let log = watch(&ctx);

    ctx.set_right_to_left(true); // the one actual change
    assert!(!recorded(&log).is_empty());

    for frame in 0..10u64 {
        clear(&log);
        ctx.set_right_to_left(true); // same value, every frame
        assert!(
            recorded(&log).is_empty(),
            "frame {frame}: a same-value set_right_to_left must not request a repaint"
        );
        assert_eq!(
            ctx.direction_preference(),
            egui::DirectionPreference::Rtl,
            "frame {frame}: the explicit preference must stay exactly as set"
        );
        run(&ctx, frame as f64, |_ui| {});
    }
}
