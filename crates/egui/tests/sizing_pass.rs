//! Sizing mode defaults, inheritance, and layout behavior.

use egui::{Align, Context, Id, Layout, Rect, Ui, UiBuilder, Vec2};

#[test]
fn root_sizing_mode_defaults_to_false() {
    let ctx = Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {
        for (name, builder, expected) in [
            ("default", UiBuilder::new(), false),
            ("opt_out", UiBuilder::new().no_sizing_pass(), false),
            ("sizing", UiBuilder::new().sizing_pass(), true),
        ] {
            let root = Ui::new(ctx.clone(), Id::unique(name), builder);
            assert_eq!(root.is_sizing_pass(), expected, "{name}");
        }
    });
    output.textures_delta.clear();
}

#[test]
fn children_inherit_and_override_sizing_mode() {
    let ctx = Context::default();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        assert!(!ui.new_child(UiBuilder::new()).is_sizing_pass());

        let mut sizing = ui.new_child(UiBuilder::new().sizing_pass());
        assert!(sizing.is_sizing_pass());
        assert!(sizing.new_child(UiBuilder::new()).is_sizing_pass());

        let mut normal = sizing.new_child(UiBuilder::new().no_sizing_pass());
        assert!(!normal.is_sizing_pass());
        assert!(!normal.new_child(UiBuilder::new()).is_sizing_pass());

        // A descendant can enable sizing again after an ancestor opted out.
        assert!(
            normal
                .new_child(UiBuilder::new().sizing_pass())
                .is_sizing_pass()
        );
    });
    output.textures_delta.clear();
}

#[test]
fn sizing_opt_out_preserves_requested_layout_and_widget_widths() {
    let ctx = Context::default();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(248.0, 90.0));
        let mut parent = ui.new_child(UiBuilder::new().sizing_pass().max_rect(rect));
        let builder = UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Center).with_cross_justify(true));
        let mut inherited = parent.new_child(builder.clone());
        assert!(!inherited.layout().cross_justify);
        assert_eq!(inherited.layout().cross_align, Align::Min);
        assert_eq!(inherited.separator().rect.width(), 0.0);
        let mut child = parent.new_child(builder.no_sizing_pass());
        assert!(child.layout().cross_justify);
        assert_eq!(child.layout().cross_align, Align::Center);
        assert_eq!(child.separator().rect.width(), 248.0);
        // Opting out does not reconstruct a layout already adjusted in an ancestor.
        let descendant = inherited.new_child(UiBuilder::new().no_sizing_pass());
        assert_eq!(descendant.layout(), inherited.layout());
    });
    output.textures_delta.clear();
}
