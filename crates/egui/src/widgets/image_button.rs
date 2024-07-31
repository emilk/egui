use crate::*;

/// A clickable image within a frame.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Debug)]
pub struct ImageButton<'a> {
    pub(crate) image: Image<'a>,
    sense: Sense,
    frame: bool,
    selected: bool,
}

impl<'a> ImageButton<'a> {
    pub fn new(image: impl Into<Image<'a>>) -> Self {
        Self {
            image: image.into(),
            sense: Sense::click(),
            frame: true,
            selected: false,
        }
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    #[inline]
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image = self.image.uv(uv);
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    #[inline]
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.image = self.image.tint(tint);
        self
    }

    /// If `true`, mark this button as "selected".
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Turn off the frame
    #[inline]
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set rounding for the `ImageButton`.
    /// If the underlying image already has rounding, this
    /// will override that value.
    #[inline]
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.image = self.image.rounding(rounding.into());
        self
    }
}

impl<'a> Widget for ImageButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let padding = if self.frame {
            // so we can see that it is a button:
            Vec2::splat(ui.spacing().button_padding.x)
        } else {
            Vec2::ZERO
        };

        let available_size_for_image = ui.available_size() - 2.0 * padding;
        let tlr = self.image.load_for_size(ui.ctx(), available_size_for_image);
        let original_image_size = tlr.as_ref().ok().and_then(|t| t.size());
        let image_size = self
            .image
            .calc_size(available_size_for_image, original_image_size);

        let padded_size = image_size + 2.0 * padding;
        let (rect, response) = ui.allocate_exact_size(padded_size, self.sense);
        response.widget_info(|| WidgetInfo::new(WidgetType::ImageButton));

        if ui.is_rect_visible(rect) {
            let (expansion, rounding, fill, stroke) = if self.selected {
                let selection = ui.visuals().selection;
                (
                    Vec2::ZERO,
                    self.image.image_options().rounding,
                    selection.bg_fill,
                    selection.stroke,
                )
            } else if self.frame {
                let visuals = ui.style().interact(&response);
                let expansion = Vec2::splat(visuals.expansion);
                (
                    expansion,
                    self.image.image_options().rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                )
            } else {
                Default::default()
            };

            // Draw frame background (for transparent images):
            ui.painter()
                .rect_filled(rect.expand2(expansion), rounding, fill);

            let image_rect = ui
                .layout()
                .align_size_within_rect(image_size, rect.shrink2(padding));
            // let image_rect = image_rect.expand2(expansion); // can make it blurry, so let's not
            let image_options = self.image.image_options().clone();

            widgets::image::paint_texture_load_result(ui, &tlr, image_rect, None, &image_options);

            // Draw frame outline:
            ui.painter()
                .rect_stroke(rect.expand2(expansion), rounding, stroke);
        }

        widgets::image::texture_load_result_response(&self.image.source(ui.ctx()), &tlr, response)
    }
}
