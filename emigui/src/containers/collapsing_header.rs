use crate::{layout::Direction, *};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct State {
    open: bool,

    #[serde(skip)] // Times are relative, and we don't want to continue animations anyway
    toggle_time: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            toggle_time: -f64::INFINITY,
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
            region.direction() == Direction::Vertical,
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
                text_size.y + 2.0 * region.style().button_padding.y,
            ),
            Some(id),
        );

        let state = {
            let mut memory = region.memory();
            let mut state = memory.collapsing_headers.entry(id).or_insert(State {
                open: default_open,
                ..Default::default()
            });
            if interact.clicked {
                state.open = !state.open;
                state.toggle_time = region.input().time;
            }
            *state
        };

        region.add_paint_cmd(PaintCmd::Rect {
            corner_radius: region.style().interact_corner_radius(&interact),
            fill_color: region.style().interact_fill_color(&interact),
            outline: region.style().interact_outline(&interact),
            rect: interact.rect,
        });

        paint_icon(region, &state, &interact);

        region.add_text(
            pos2(
                interact.rect.left() + region.style().indent,
                interact.rect.center().y - text_size.y / 2.0,
            ),
            text_style,
            title,
            Some(region.style().interact_stroke_color(&interact)),
        );

        let animation_time = region.style().animation_time;
        let time_since_toggle = (region.input().time - state.toggle_time) as f32;
        let animate = time_since_toggle < animation_time;
        if animate {
            region.indent(id, |child_region| {
                let max_height = if state.open {
                    remap(
                        time_since_toggle,
                        0.0..=animation_time,
                        // Get instant feedback, and we don't expect to get bigger than this
                        100.0..=1500.0,
                    )
                } else {
                    remap_clamp(
                        time_since_toggle,
                        0.0..=animation_time,
                        // TODO: state.open_height
                        50.0..=0.0,
                    )
                };

                let mut clip_rect = child_region.clip_rect();
                clip_rect.max.y = clip_rect.max.y.min(child_region.cursor().y + max_height);
                child_region.set_clip_rect(clip_rect);

                let top_left = child_region.top_left();
                add_contents(child_region);
                // Pretend children took up less space than they did:
                child_region.child_bounds.max.y =
                    child_region.child_bounds.max.y.min(top_left.y + max_height);
            });
        } else if state.open {
            region.indent(id, add_contents);
        }

        region.response(interact)
    }
}

fn paint_icon(region: &mut Region, state: &State, interact: &InteractInfo) {
    let stroke_color = region.style().interact_stroke_color(&interact);
    let stroke_width = region.style().interact_stroke_width(&interact);

    let (mut small_icon_rect, _) = region.style().icon_rectangles(&interact.rect);
    small_icon_rect.set_center(pos2(
        interact.rect.left() + region.style().indent / 2.0,
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
