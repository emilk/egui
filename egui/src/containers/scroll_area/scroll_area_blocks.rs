use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct State {
    pub s: scroll_area::State,

    first_to_show: usize,
    first_shown_block_size: Vec2,
    
    #[cfg_attr(feature = "serde", serde(skip))]
    block_count: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            s: Default::default(),
            first_to_show: 0,
            block_count: 0,
            first_shown_block_size: Vec2::ZERO,
        }
    }
}

impl State {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data().get_persisted(id)
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data().insert_persisted(id, self);
    }

    fn block_scroll_out_fraction(&self, size: Vec2, has_bar: &[bool; 2]) -> Vec2 {
        let fraction = |d| {
            if self.block_count == 0 {
                0.
            } else {
                let scroll_block_size = size[d] / (self.block_count as f32);
                if self.first_to_show != ((self.s.offset[d] / scroll_block_size) as usize) {
                    // TODO figure out handling for this case, to work with drag-scrolling
                    // this causes 1 frame jump whenever we scroll up (left?) to new item
                    // would it be worth always drawing item -1 / +1 to have their sizes?
                    // drawing -1 / +1 actually looks even worse, when these are recalculated
                    // can we do rendering to drop buffer, just to get the size? then render normally for a second time?
                    0.
                } else {
                    let a = self.s.offset[d] - self.first_to_show as f32 * scroll_block_size;
                    self.first_shown_block_size[d] * a / scroll_block_size
                }
            }
        };
        if has_bar[0] {
            vec2(fraction(0), 0.)
        } else if has_bar[1] {
            vec2(0., fraction(1))
        } else {
            vec2(0., 0.)
        }
    }
}

pub struct ScrollAreaBlocksOutput {
    /// [`Id`] of the [`ScrollArea`].
    pub id: Id,

    /// The current state of the scroll area.
    pub state: State,

    /// Where on the screen the content is (excludes scroll bars).
    pub inner_rect: Rect,
}

impl ScrollArea {
    /// Efficiently show only the visible part of a large number of rows.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let total_rows = 1_000;
    /// egui::ScrollArea::vertical().show_blocks(total_rows, ui, |ui, item_index| {
    ///     let text = format!("block {}/{}", item_index + 1, total_rows);
    ///     ui.label(text);
    ///     if item_index & 1 == 0 { // show that items can have significantly different sizes
    ///         ui.indent(row, |ui|{
    ///             for i in 0..item_index {
    ///                 ui.label(format!("item {}/{}", i + 1, item_index));
    ///             }
    ///         });
    ///     }
    /// });
    /// # });
    /// ```
    pub fn show_blocks(
        self,
        total_block_count: usize,
        ui: &mut Ui,
        mut add_contents: impl FnMut(&mut Ui, usize),
    ) -> ScrollAreaBlocksOutput {
        let id = self.id(ui);
        if self.has_bar[0] && self.has_bar[1] {
            let pos = ui.available_rect_before_wrap().min;
            let painter = ui.ctx().debug_painter();
            painter.error(pos, "show_blocks can not used with both scroll axis enabled at same time");
            return ScrollAreaBlocksOutput { id, state: Default::default(), inner_rect: Rect::NAN }
        }
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();
        let mut prepared = self.begin(&mut state, id, ui);
        let inner_rect = prepared.inner_rect;
        state.first_to_show = if prepared.has_bar[0] {
            let block_width_on_scroll = inner_rect.size()[0] / (total_block_count as f32);
            (state.s.offset[0] / block_width_on_scroll) as usize
        } else if prepared.has_bar[1] {
            let block_height_on_scroll = inner_rect.size()[1] / (total_block_count as f32);
            (state.s.offset[1] / block_height_on_scroll) as usize
        } else {
            0
        }.min(total_block_count - 1);

        prepared.content_ui.push_id(state.first_to_show, |ui|{
            add_contents(ui, state.first_to_show);
        });
        state.first_shown_block_size = prepared.content_ui.min_size();

        for i in (state.first_to_show + 1)..total_block_count {
            if (prepared.has_bar[0]
                && !inner_rect.x_range().contains(&prepared.content_ui.next_widget_position().x))
                || (prepared.has_bar[1]
                && !inner_rect.y_range().contains(&prepared.content_ui.next_widget_position().y))
            {
                break;
            }
            prepared.content_ui.push_id(i, |ui|{
                add_contents(ui, i);
            });
        }

        // TODO is there an better way? (to prevent shrinking when last block is smaller)
        prepared.auto_shrink =
            if state.first_to_show > 0 {
                if prepared.has_bar[0] { [false, prepared.auto_shrink[1]] } else { [prepared.auto_shrink[0], false] }
            } else { prepared.auto_shrink };

        if state.block_count != total_block_count {
            for d in 0..2 {
                if prepared.has_bar[d] {
                    if state.block_count != 0 {
                        if total_block_count != 0 {
                            let old_block_size = inner_rect.size()[d] / (state.block_count as f32);
                            let new_block_size = inner_rect.size()[d] / (total_block_count as f32);
                            state.s.offset[d] -= state.first_to_show as f32 * old_block_size;
                            state.s.offset[d] += state.first_to_show as f32 * new_block_size;
                        } else {
                            state.s.offset[d] = 0.;
                        }
                    } else {
                        state.s.offset[d] = 0.;
                    }
                } else {
                    state.s.offset[d] = 0.;
                }
            }
        }
        state.block_count = total_block_count;
        prepared.end(&mut state, id, ui);
        ScrollAreaBlocksOutput {
            id,
            state,
            inner_rect,
        }
    }
}

impl scroll_area::ScrollState for State {
    fn s(&mut self) -> &mut scroll_area::State {
        &mut self.s
    }
    fn content_max_rect(&self, inner_rect: &Rect, content_max_size: Vec2, has_bar: &[bool; 2]) -> Rect {
        let fraction = self.block_scroll_out_fraction(inner_rect.size(), has_bar);
        Rect::from_min_size(inner_rect.min - fraction, content_max_size)
    }
    fn content_is_too_large(&self, content_size: Vec2, inner_rect: &Rect, has_bar: &[bool;2]) -> [bool;2] {
        [
            content_size.x > inner_rect.width() || (has_bar[0] && self.first_to_show != 0),
            content_size.y > inner_rect.height() || (has_bar[1] && self.first_to_show != 0),
        ]
    }
    fn max_offset(&self, _content_size: Vec2, inner_size: Vec2) -> Vec2 {
        inner_size
    }
    fn handle_rect(&self, d: usize, _content_size: Vec2, inner_rect: &Rect, bounds: &scroll_area::HandleBounds) -> Rect {
        let scroll_block_size = inner_rect.size()[d] / (self.block_count as f32);
        if d == 0 {
            Rect::from_min_max(
                pos2(bounds.min_main + self.s.offset.x, bounds.min_cross),
                pos2(bounds.min_main + self.s.offset.x + scroll_block_size, bounds.max_cross),
            )
        } else {
            Rect::from_min_max(
                pos2(bounds.min_cross, bounds.min_main + self.s.offset.y),
                pos2(bounds.max_cross, bounds.min_main + self.s.offset.y + scroll_block_size),
            )
        }
    }
    fn remap_offset(&mut self, d: usize, new_handle_top: f32, min_main: f32, _max_main: f32, _content_size: Vec2) {
        self.s.offset[d] = new_handle_top - min_main;
    }
    fn max_offset_final(&mut self, _content_size: Vec2, inner_size: Vec2) -> Vec2{
        let mut max_size = inner_size;
        for d in 0..2 {
            let scroll_block_size = inner_size[d] / (self.block_count as f32);
            if self.first_shown_block_size[d] <= inner_size[d] {
                max_size[d] -= scroll_block_size;
            } else {
                max_size[d] -= scroll_block_size / (self.first_shown_block_size[d] / inner_size[d]);
            }
        }
        self.s.offset = self.s.offset.min(max_size);
        self.s.offset = self.s.offset.max(Vec2::ZERO);
        max_size
    }
    fn store_state(&self, id: Id, ui: &mut Ui) {
        self.store(ui.ctx(), id);
    }

    fn dragged(&mut self, inner_rect: &Rect, delta: Vec2, velocity: Vec2, has_bar: &[bool;2]) {
        let delta = SCROLL_MULTIPLIER * delta / self.first_shown_block_size;
        let velocity = SCROLL_MULTIPLIER * velocity / self.first_shown_block_size;
        self.s.dragged(inner_rect, delta, velocity, has_bar)
    }
    fn slide_after_drag(&mut self, ui: &mut Ui) {
        self.s.slide_after_drag(ui)
    }
    fn scroll(&mut self, d: usize, inner_rect: &Rect, scroll_delta: Vec2) {
        let scroll_delta = SCROLL_MULTIPLIER * scroll_delta / self.first_shown_block_size;
        self.s.scroll(d, inner_rect, scroll_delta)
    }
}

// random number chosen to make scrolling not feel too slow
const SCROLL_MULTIPLIER : f32 = 2.5;

impl AsMut<scroll_area::State> for State {
    fn as_mut(&mut self) -> &mut scroll_area::State {
        &mut self.s
    }
}
