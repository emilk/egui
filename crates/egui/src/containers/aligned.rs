use emath::{Align2, Rect, Vec2};

use crate::{AsIdSalt, IdSalt, InnerResponse, Ui, UiBuilder};

/// Align some widgets of unknown size within the available space of a [`Ui`].
///
/// For instance, this can put a group of widgets in the bottom-right corner of the parent [`Ui`].
///
/// egui is an immediate mode GUI, so the size of the contents is not known until they have been added.
/// This container remembers the size from the previous pass, and uses that to place the contents.
/// On the first pass (when no size is known yet) the contents are added invisibly,
/// and a new pass is requested with [`crate::Context::request_discard`], so there is no visible flicker.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Aligned::new(egui::Align2::RIGHT_BOTTOM).show(ui, |ui| {
///     ui.label("Bottom right");
///     let _ = ui.button("Click me");
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
#[derive(Clone, Copy, Debug)]
pub struct Aligned {
    align2: Align2,
    id_salt: Option<IdSalt>,
}

impl Aligned {
    /// Align the contents within the available space of the parent [`Ui`].
    pub fn new(align2: Align2) -> Self {
        Self {
            align2,
            id_salt: None,
        }
    }

    /// A source for the unique [`crate::Id`], e.g. `.id_salt("second_aligned")` or `.id_salt(loop_index)`.
    ///
    /// Needed if you have several [`Aligned`] in the same [`Ui`].
    #[inline]
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// Show the contents.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let Self { align2, id_salt } = self;

        let id_salt = id_salt.unwrap_or_else(|| IdSalt::new("aligned"));
        let id = ui.make_persistent_id(id_salt);
        let size_id = id.with("size");

        let available_rect = ui.available_rect_before_wrap();
        let last_size: Option<Vec2> = ui.data(|d| d.get_temp(size_id));

        let content_rect = if let Some(size) = last_size {
            let left_top = align2
                .align_size_within_rect(size, available_rect)
                .left_top();

            // Never place the contents above or to the left of the available space,
            // even if the contents are larger than the available space:
            let left_top = left_top.max(available_rect.left_top());

            // Keep the full available size, in case the contents grow this pass:
            Rect::from_min_size(left_top, available_rect.size())
        } else {
            available_rect
        };

        let mut builder = UiBuilder::new().id(id).max_rect(content_rect);
        if last_size.is_none() {
            // We don't know the size yet, so we can't place the contents correctly.
            // Add them invisibly to measure them, and then try again.
            builder = builder.invisible();
        }

        let response = ui.scope_builder(builder, add_contents);

        let size = response.response.rect.size();
        if last_size != Some(size) {
            ui.ctx().request_discard("Aligned: size changed");
        }
        ui.data_mut(|d| d.insert_temp(size_id, size));

        response
    }
}

#[cfg(test)]
mod tests {
    use emath::vec2;

    use crate::{Align2, Aligned, Context, FontDefinitions, Rect};

    /// Run one frame of a single [`Aligned`] and return the rect of its contents, and the rect of the parent.
    fn run_frame(ctx: &Context) -> (Rect, Rect) {
        let mut rects = None;
        let output = ctx.run_ui(Default::default(), |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let response = Aligned::new(Align2::RIGHT_BOTTOM).show(ui, |ui| {
                ui.allocate_space(vec2(30.0, 20.0));
            });
            rects = Some((response.response.rect, available_rect));
        });
        output.drop_without_applying_deltas();
        rects.expect("The closure was not called")
    }

    #[test]
    fn contents_end_up_aligned() {
        let ctx = Context::default();
        ctx.set_fonts(FontDefinitions::empty());

        // First frame measures, second frame places:
        run_frame(&ctx);
        let (content_rect, available_rect) = run_frame(&ctx);

        assert_eq!(content_rect.size(), vec2(30.0, 20.0));
        assert!((content_rect.right() - available_rect.right()).abs() < 0.01);
        assert!((content_rect.bottom() - available_rect.bottom()).abs() < 0.01);
    }
}
