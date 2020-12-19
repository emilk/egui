use crate::*;

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Debug)]
pub struct ImageButton {
    image: widgets::Image,
    sense: Sense,
    frame: bool,
    selected: bool,
}

impl ImageButton {
    pub fn new(texture_id: TextureId, desired_size: impl Into<Vec2>) -> Self {
        Self {
            image: widgets::Image::new(texture_id, desired_size),
            sense: Sense::click(),
            frame: true,
            selected: false,
        }
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image = self.image.uv(uv);
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: impl Into<Srgba>) -> Self {
        self.image = self.image.tint(tint);
        self
    }

    /// If `true`, mark this button as "selected".
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Turn off the frame
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl Widget for ImageButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            image,
            sense,
            frame,
            selected,
        } = self;

        let button_padding = ui.style().spacing.button_padding;
        let desired_size = image.desired_size() + 2.0 * button_padding;
        let rect = ui.allocate_space(desired_size);
        let id = ui.make_position_id();
        let response = ui.interact(rect, id, sense);

        if ui.clip_rect().intersects(rect) {
            let visuals = ui.style().interact(&response);

            if selected {
                let selection = ui.style().visuals.selection;
                ui.painter()
                    .rect(response.rect, 0.0, selection.bg_fill, selection.stroke);
            } else if frame {
                ui.painter().rect(
                    response.rect,
                    visuals.corner_radius,
                    visuals.bg_fill,
                    visuals.bg_stroke,
                );
            }

            let image_rect = ui.layout().align_size_within_rect(
                image.desired_size(),
                response.rect.shrink2(button_padding),
            );
            image.paint_at(ui, image_rect);
        }

        response
    }
}
