//! The direction preference chain: an explicit direction persists across a
//! persistence round-trip, a reported system locale bootstraps the direction
//! while the preference is [`egui::DirectionPreference::System`], an explicit
//! preference wins over the locale, and per-frame callers stay stable.

use egui::{
    Context, DirectionPreference, RawInput, Rect, RequestRepaintInfo, Ui, Vec2, mutex::Mutex,
};
use std::sync::Arc;

fn input(time: f64, system_locale: &str) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(
            Default::default(),
            Vec2::new(800.0, 600.0),
        )),
        time: Some(time),
        system_locale: Some(system_locale.to_owned()),
        ..Default::default()
    }
}

/// Run one ui pass with the given reported system locale and discard the
/// texture deltas (these tests never render).
fn run_locale(ctx: &Context, time: f64, system_locale: &str, run_ui: impl FnMut(&mut Ui)) {
    let mut output = ctx.run_ui(input(time, system_locale), run_ui);
    output.textures_delta.clear();
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

/// A reported RTL locale bootstraps the direction while the preference is
/// `System`, an LTR locale resolves left-to-right, and a mid-run locale flip
/// follows on the next frame.
#[test]
fn system_locale_bootstraps_the_direction() {
    let ctx = Context::default();
    assert_eq!(ctx.direction_preference(), DirectionPreference::System);

    run_locale(&ctx, 0.0, "ar-EG", |_ui| {});
    assert!(
        ctx.is_rtl(),
        "an RTL system locale should bootstrap a right-to-left interface"
    );

    run_locale(&ctx, 1.0, "en-US", |_ui| {});
    assert!(
        !ctx.is_rtl(),
        "the direction should follow the locale when it changes"
    );

    run_locale(&ctx, 2.0, "he", |_ui| {});
    assert!(ctx.is_rtl(), "a mid-run flip to Hebrew should be applied");
}

/// The LTR twin of the bootstrap, from a fresh default context.
#[test]
fn an_ltr_locale_leaves_the_interface_ltr() {
    let ctx = Context::default();
    run_locale(&ctx, 0.0, "en-US", |_ui| {});
    assert!(!ctx.is_rtl());
}

/// `set_right_to_left` records an explicit preference, and a reported system
/// locale must not override it.
#[test]
fn set_right_to_left_records_an_explicit_preference() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    assert_eq!(ctx.direction_preference(), DirectionPreference::Rtl);

    run_locale(&ctx, 0.0, "en-US", |_ui| {});
    assert!(
        ctx.is_rtl(),
        "the explicit preference must win over the system locale"
    );
    assert_eq!(ctx.direction_preference(), DirectionPreference::Rtl);

    let ctx = Context::default();
    ctx.set_right_to_left(false);
    assert_eq!(ctx.direction_preference(), DirectionPreference::Ltr);
    run_locale(&ctx, 0.0, "ar-EG", |_ui| {});
    assert!(
        !ctx.is_rtl(),
        "the explicit preference must win over the locale"
    );
}

/// Per-frame callers (the i18n demo-app pattern) write the same preference
/// every frame: the recorded preference never changes and no repaint storm
/// appears, even with a locale being reported every frame.
///
/// The initial change carries a bounded settle echo (each immediate repaint
/// request results in exactly two, by design — frames 0 and 1); from frame 2
/// on, nothing may ever request a repaint again.
#[test]
fn per_frame_set_right_to_left_keeps_the_preference_stable() {
    let ctx = Context::default();
    let log = watch(&ctx);

    ctx.set_right_to_left(true); // the one actual change
    assert!(!log.lock().is_empty());

    for frame in 0..10u64 {
        log.lock().clear();
        ctx.set_right_to_left(true); // same value, every frame
        assert!(
            log.lock().is_empty(),
            "frame {frame}: a same-value set_right_to_left must not request a repaint"
        );
        run_locale(&ctx, frame as f64, "ar-EG", |_ui| {});
        assert_eq!(
            ctx.direction_preference(),
            DirectionPreference::Rtl,
            "frame {frame}: the explicit preference must stay exactly as set"
        );
        if 2 <= frame {
            assert!(
                log.lock().is_empty(),
                "frame {frame}: the per-frame pattern must not repaint-storm"
            );
        }
    }
}

/// An explicit direction survives a persistence round-trip: the saved
/// `Options` carries the preference (the styles themselves are serde-skipped),
/// and the restored context re-applies it — even against a different reported
/// locale.
#[cfg(feature = "persistence")]
#[test]
fn direction_preference_survives_a_persistence_round_trip() {
    let saved = {
        let ctx = Context::default();
        ctx.set_right_to_left(true);
        run_locale(&ctx, 0.0, "en-US", |_ui| {});
        ctx.memory(|memory| ron::to_string(memory).unwrap())
    };

    let ctx = Context::default();
    ctx.memory_mut(|memory| *memory = ron::from_str(&saved).unwrap());
    assert_eq!(
        ctx.direction_preference(),
        DirectionPreference::Rtl,
        "the preference should deserialize from the saved state"
    );

    run_locale(&ctx, 1.0, "en-US", |_ui| {});
    assert!(
        ctx.is_rtl(),
        "the restored preference should be re-applied against the reported locale"
    );
}
