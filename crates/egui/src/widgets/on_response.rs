use crate::{Popup, Response, Ui, Widget, WidgetText};

type ResponseCallbacks<'a> = smallvec::SmallVec<[Box<dyn FnOnce(Response) -> Response + 'a>; 1]>;

/// A [`Widget`] wrapped with callbacks that act on its [`Response`].
///
/// Created with the methods of [`OnResponseExt`], which is implemented for every [`Widget`].
///
/// ```
/// # use egui::OnResponseExt as _;
/// # egui::__run_test_ui(|ui| {
/// let mut count = 0;
/// ui.add(
///     egui::Button::new("Click me")
///         .on_hover_text("This is a button")
///         .on_click(|| count += 1),
/// );
/// # });
/// ```
pub struct OnResponse<'a, T> {
    inner: T,
    on_response: ResponseCallbacks<'a>,
    enabled: bool,
}

/// Generates the builder methods of [`OnResponseExt`] and [`OnResponse`].
///
/// The methods are needed on both, so that a call on an [`OnResponse`] returns an
/// `OnResponse<T>` rather than a nested `OnResponse<OnResponse<T>>`.
/// A blanket `impl OnResponseExt for T: Widget` would also cover `OnResponse`,
/// so `OnResponse` can't get a separate trait impl - hence the macro.
macro_rules! on_response_methods {
    ($target:path, $($pub:tt)*) => {
        /// Enable or disable the widget.
        ///
        /// A disabled widget is grayed out and can't be interacted with.
        #[inline]
        $($pub)* fn enabled(self, enabled: bool) -> OnResponse<'a, $target> {
            let mut on_response = self.into_on_response();
            on_response.enabled = enabled;
            on_response
        }

        /// Add a callback that is called with the [`Response`] of the widget, once it has been added.
        ///
        /// The callback returns the (possibly modified) [`Response`], which is then
        /// what [`Ui::add`] returns.
        #[inline]
        $($pub)* fn on_response(
            self,
            on_response: impl FnOnce(Response) -> Response + 'a,
        ) -> OnResponse<'a, $target> {
            let mut wrapped = self.into_on_response();
            wrapped.on_response.push(Box::new(on_response));
            wrapped
        }

        /// Add a callback that is called when the widget is clicked.
        #[inline]
        $($pub)* fn on_click(self, on_click: impl FnOnce() + 'a) -> OnResponse<'a, $target> {
            self.on_response(move |response| {
                if response.clicked() {
                    on_click();
                }
                response
            })
        }

        /// Show some ui in a tooltip when the widget is hovered.
        ///
        /// See [`Response::on_hover_ui`].
        #[inline]
        $($pub)* fn on_hover_ui(
            self,
            on_hover_ui: impl FnOnce(&mut Ui) + 'a,
        ) -> OnResponse<'a, $target> {
            self.on_response(move |response| response.on_hover_ui(on_hover_ui))
        }

        /// Show some ui in a tooltip when the disabled widget is hovered.
        ///
        /// See [`Response::on_disabled_hover_ui`].
        #[inline]
        $($pub)* fn on_disabled_hover_ui(
            self,
            on_hover_ui: impl FnOnce(&mut Ui) + 'a,
        ) -> OnResponse<'a, $target> {
            self.on_response(move |response| response.on_disabled_hover_ui(on_hover_ui))
        }

        /// Show some text in a tooltip when the widget is hovered.
        ///
        /// See [`Response::on_hover_text`].
        #[inline]
        $($pub)* fn on_hover_text(self, hover_text: impl Into<WidgetText> + 'a) -> OnResponse<'a, $target> {
            let hover_text = hover_text.into();
            self.on_response(move |response| response.on_hover_text(hover_text))
        }

        /// Show some text in a tooltip when the disabled widget is hovered.
        ///
        /// See [`Response::on_disabled_hover_text`].
        #[inline]
        $($pub)* fn on_disabled_hover_text(
            self,
            hover_text: impl Into<WidgetText> + 'a,
        ) -> OnResponse<'a, $target> {
            let hover_text = hover_text.into();
            self.on_response(move |response| response.on_disabled_hover_text(hover_text))
        }

        /// Show a menu when the widget is clicked.
        ///
        /// See [`Popup::menu`].
        #[inline]
        $($pub)* fn on_menu(self, add_contents: impl FnOnce(&mut Ui) + 'a) -> OnResponse<'a, $target> {
            self.on_custom_menu(|popup| popup, add_contents)
        }

        /// Show a menu when the widget is clicked, with a chance to customize the [`Popup`] first.
        ///
        /// See [`Popup::menu`].
        #[inline]
        $($pub)* fn on_custom_menu(
            self,
            customize: impl FnOnce(Popup<'_>) -> Popup<'_> + 'a,
            add_contents: impl FnOnce(&mut Ui) + 'a,
        ) -> OnResponse<'a, $target> {
            self.on_response(move |response| {
                customize(Popup::menu(&response)).show(add_contents);
                response
            })
        }
    };
}

impl<'a, T> OnResponse<'a, T> {
    #[inline]
    fn into_on_response(self) -> Self {
        self
    }

    on_response_methods!(T, pub);
}

/// Extension trait for adding callbacks to any [`Widget`], acting on its [`Response`].
///
/// This lets you e.g. add a tooltip or a click handler to a widget before adding it to a [`Ui`],
/// which is convenient when passing widgets around, or when building a widget in a builder chain.
///
/// ```
/// # use egui::OnResponseExt as _;
/// # egui::__run_test_ui(|ui| {
/// let mut count = 0;
/// ui.add(
///     egui::Button::new("Click me")
///         .on_hover_text("This is a button")
///         .on_click(|| count += 1),
/// );
///
/// ui.add(
///     egui::Button::new("Menu").on_menu(|ui| {
///         let _ = ui.button("Item");
///     }),
/// );
/// # });
/// ```
pub trait OnResponseExt<'a>: Sized {
    /// The wrapped widget.
    type Target;

    /// Wrap this widget in an [`OnResponse`].
    fn into_on_response(self) -> OnResponse<'a, Self::Target>;

    on_response_methods!(Self::Target,);
}

impl<'a, T: Widget> OnResponseExt<'a> for T {
    type Target = T;

    #[inline]
    fn into_on_response(self) -> OnResponse<'a, Self::Target> {
        OnResponse {
            inner: self,
            on_response: smallvec::SmallVec::new(),
            enabled: true,
        }
    }
}

impl<T: Widget> Widget for OnResponse<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            inner,
            on_response,
            enabled,
        } = self;

        let mut response = ui.add_enabled(enabled, inner);

        for on_response in on_response {
            response = on_response(response);
        }

        response
    }
}

impl<T> core::ops::Deref for OnResponse<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> core::ops::DerefMut for OnResponse<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use core::cell::{Cell, RefCell};

    use super::OnResponseExt as _;
    use crate::{Button, Context, FontDefinitions};

    #[test]
    fn callbacks_run_in_order_and_disabled_is_respected() {
        let ctx = Context::default();
        ctx.set_fonts(FontDefinitions::empty());

        let order = RefCell::new(Vec::new());
        let enabled = Cell::new(None);

        let output = ctx.run_ui(Default::default(), |ui| {
            ui.add(
                Button::new("Button")
                    .on_response(|response| {
                        order.borrow_mut().push(1);
                        response
                    })
                    .on_response(|response| {
                        order.borrow_mut().push(2);
                        response
                    })
                    .enabled(false)
                    .on_response(|response| {
                        enabled.set(Some(response.enabled()));
                        response
                    }),
            );
        });
        output.drop_without_applying_deltas();

        assert_eq!(order.into_inner(), [1, 2]);
        assert_eq!(enabled.get(), Some(false));
    }
}
