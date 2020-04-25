use crate::{layout::Direction, *};

#[derive(Clone, Copy, Debug)]
pub(crate) struct State {
    pub open: bool,
    pub toggle_time: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            toggle_time: -std::f64::INFINITY,
        }
    }
}

pub struct CollapsingHeader {
    title: String,
    default_open: bool,
}

impl CollapsingHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            default_open: false,
        }
    }

    pub fn default_open(mut self) -> Self {
        self.default_open = true;
        self
    }
}

impl CollapsingHeader {
    pub fn show(self, region: &mut Region, add_contents: impl FnOnce(&mut Region)) -> GuiResponse {
        assert!(
            region.dir == Direction::Vertical,
            "Horizontal collapsing is unimplemented"
        );
        let Self {
            title,
            default_open,
        } = self;

        let id = region.make_unique_id(&title);
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (title, text_size) = font.layout_multiline(&title, region.available_width());

        let interact = region.reserve_space(
            vec2(
                region.available_width(),
                text_size.y + 2.0 * region.style.button_padding.y,
            ),
            Some(id),
        );

        let state = {
            let mut memory = region.ctx.memory.lock();
            let mut state = memory.collapsing_headers.entry(id).or_insert(State {
                open: default_open,
                ..Default::default()
            });
            if interact.clicked {
                state.open = !state.open;
                state.toggle_time = region.ctx.input.time;
            }
            *state
        };

        region.add_paint_cmd(PaintCmd::Rect {
            corner_radius: region.style.interact_corner_radius(&interact),
            fill_color: region.style.interact_fill_color(&interact),
            outline: region.style().interact_outline(&interact),
            rect: interact.rect,
        });

        paint_icon(region, &state, &interact);

        region.add_text(
            pos2(
                interact.rect.left() + region.style.indent,
                interact.rect.center().y - text_size.y / 2.0,
            ),
            text_style,
            title,
            Some(region.style.interact_stroke_color(&interact)),
        );

        let animation_time = region.style().animation_time;
        let time_since_toggle = (region.ctx.input.time - state.toggle_time) as f32;
        if time_since_toggle < animation_time {
            region.indent(id, |region| {
                // animation time

                let max_height = if state.open {
                    remap(
                        time_since_toggle,
                        0.0..=animation_time,
                        // Get instant feedback, and we don't expect to get bigger than this
                        50.0..=1500.0,
                    )
                } else {
                    remap_clamp(
                        time_since_toggle,
                        0.0..=animation_time,
                        // TODO: state.open_height
                        50.0..=0.0,
                    )
                };

                region
                    .clip_rect
                    .set_height(region.clip_rect.height().min(max_height));

                add_contents(region);

                region.child_bounds.max.y = region
                    .child_bounds
                    .max
                    .y
                    .min(region.child_bounds.min.y + max_height);
            });
        } else if state.open {
            region.indent(id, add_contents);
        }

        region.response(interact)
    }
}

fn paint_icon(region: &mut Region, state: &State, interact: &InteractInfo) {
    let stroke_color = region.style.interact_stroke_color(&interact);
    let stroke_width = region.style.interact_stroke_width(&interact);

    let (mut small_icon_rect, _) = region.style.icon_rectangles(&interact.rect);
    small_icon_rect.set_center(pos2(
        interact.rect.left() + region.style.indent / 2.0,
        interact.rect.center().y,
    ));

    // Draw a minus:
    region.add_paint_cmd(PaintCmd::Line {
        points: vec![
            pos2(small_icon_rect.left(), small_icon_rect.center().y),
            pos2(small_icon_rect.right(), small_icon_rect.center().y),
        ],
        color: stroke_color,
        width: stroke_width,
    });

    if !state.open {
        // Draw it as a plus:
        region.add_paint_cmd(PaintCmd::Line {
            points: vec![
                pos2(small_icon_rect.center().x, small_icon_rect.top()),
                pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
            ],
            color: stroke_color,
            width: stroke_width,
        });
    }
}
