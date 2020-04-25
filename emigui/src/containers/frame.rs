//! Frame container

use crate::*;

#[derive(Clone, Debug, Default)]
pub struct Frame {}

impl Frame {
    pub fn show(self, region: &mut Region, add_contents: impl FnOnce(&mut Region)) {
        let style = region.style();
        let margin = style.window_padding;

        let outer_pos = region.cursor();
        let inner_rect =
            Rect::from_min_size(outer_pos + margin, region.available_space() - 2.0 * margin);
        let where_to_put_background = region.paint_list_len();

        let mut child_region = region.child_region(inner_rect);
        add_contents(&mut child_region);

        // TODO: handle the last item_spacing in a nicer way
        let inner_size = child_region.bounding_size();
        let inner_size = inner_size.ceil(); // TODO: round to pixel

        let outer_rect = Rect::from_min_size(outer_pos, margin + inner_size + margin);

        let corner_radius = style.window.corner_radius;
        let fill_color = style.background_fill_color();
        region.insert_paint_cmd(
            where_to_put_background,
            PaintCmd::Rect {
                corner_radius,
                fill_color: Some(fill_color),
                outline: Some(Outline::new(1.0, color::WHITE)),
                rect: outer_rect,
            },
        );

        // TODO: move up corsor?
        region
            .child_bounds
            .extend_with(child_region.child_bounds.max + margin);
    }
}
