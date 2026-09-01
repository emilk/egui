use raw_window_handle::RawDisplayHandle;

/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works within the same app is used instead.
pub struct Clipboard {
    #[cfg(all(
        not(any(target_os = "android", target_os = "ios")),
        feature = "arboard",
    ))]
    arboard: Option<arboard::Clipboard>,

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        feature = "smithay-clipboard"
    ))]
    smithay: Option<smithay_clipboard::Clipboard>,

    /// Fallback manual clipboard.
    clipboard: String,
}

impl Clipboard {
    /// Construct a new instance
    pub fn new(_raw_display_handle: Option<RawDisplayHandle>) -> Self {
        Self {
            #[cfg(all(
                not(any(target_os = "android", target_os = "ios")),
                feature = "arboard",
            ))]
            arboard: init_arboard(),

            #[cfg(all(
                any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ),
                feature = "smithay-clipboard"
            ))]
            smithay: init_smithay_clipboard(_raw_display_handle),

            clipboard: Default::default(),
        }
    }

    pub fn get(&mut self) -> Option<String> {
        #[cfg(all(
            any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ),
            feature = "smithay-clipboard"
        ))]
        if let Some(clipboard) = &mut self.smithay {
            match clipboard.load() {
                Ok(text) => return Some(text),
                Err(err) => {
                    log::error!("smithay paste error: {err}");
                }
            }
        }

        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            return match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(err) => {
                    // Expected whenever the clipboard holds something other than text (e.g.
                    // an image copied with a screenshot tool) — the caller falls back to
                    // `Self::get_image` in that case, so this is not an error worth
                    // alarming the user/log about.
                    if !is_expected_content_absence(&err) {
                        log::error!("arboard paste error: {err}");
                    }
                    None
                }
            };
        }

        Some(self.clipboard.clone())
    }

    pub fn set_text(&mut self, text: String) {
        #[cfg(all(
            any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ),
            feature = "smithay-clipboard"
        ))]
        if let Some(clipboard) = &mut self.smithay {
            clipboard.store(text);
            return;
        }

        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_text(text) {
                log::error!("arboard copy/cut error: {err}");
            }
            return;
        }

        self.clipboard = text;
    }

    /// Get an image from the clipboard, if there is one and the platform backend supports it.
    ///
    /// This mirrors [`Self::set_image`] for the opposite direction, so that a Ctrl+V/Cmd+V
    /// paste can carry an image (e.g. a screenshot or a copied image) instead of text — see
    /// [`egui::Event::PasteImage`].
    pub fn get_image(&mut self) -> Option<egui::ColorImage> {
        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            return match clipboard.get_image() {
                Ok(image) => Some(color_image_from_arboard(&image)),
                Err(err) => {
                    // Expected whenever the clipboard holds neither text nor an image (e.g.
                    // it's simply empty) — `Self::get` was already tried first and came up
                    // empty too, so this is the mundane "nothing to paste" case, not an error.
                    if !is_expected_content_absence(&err) {
                        log::error!("arboard paste-image error: {err}");
                    }
                    None
                }
            };
        }

        None
    }

    pub fn set_image(&mut self, image: &egui::ColorImage) {
        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_image(arboard::ImageData {
                width: image.width(),
                height: image.height(),
                bytes: std::borrow::Cow::Borrowed(bytemuck::cast_slice(&image.pixels)),
            }) {
                log::error!("arboard copy/cut error: {err}");
            }
            log::debug!("Copied image to clipboard");
            return;
        }

        log::error!(
            "Copying images is not supported. Enable the 'clipboard' feature of `egui-winit` to enable it."
        );
        _ = image;
    }
}

/// Whether an `arboard::Error` from reading the clipboard is the expected, mundane outcome
/// of the clipboard simply not holding the requested content type (e.g. text was asked for
/// but the clipboard holds an image, or vice versa, or it's just empty) — as opposed to a
/// genuine failure (permissions, a locked clipboard, a conversion error) worth an `error!` log.
///
/// Pulled out as its own pure function (rather than inlined in the two `match`es above) so it
/// can be unit-tested without touching the real OS clipboard, which CI can't rely on.
#[cfg(all(
    not(any(target_os = "android", target_os = "ios")),
    feature = "arboard",
))]
fn is_expected_content_absence(err: &arboard::Error) -> bool {
    matches!(err, arboard::Error::ContentNotAvailable)
}

#[cfg(all(
    not(any(target_os = "android", target_os = "ios")),
    feature = "arboard",
))]
fn color_image_from_arboard(image: &arboard::ImageData<'_>) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.bytes)
}

#[cfg(all(
    not(any(target_os = "android", target_os = "ios")),
    feature = "arboard",
))]
fn init_arboard() -> Option<arboard::Clipboard> {
    profiling::function_scope!();

    log::trace!("Initializing arboard clipboard…");
    match arboard::Clipboard::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            log::warn!("Failed to initialize arboard clipboard: {err}");
            None
        }
    }
}

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "smithay-clipboard"
))]
fn init_smithay_clipboard(
    raw_display_handle: Option<RawDisplayHandle>,
) -> Option<smithay_clipboard::Clipboard> {
    #![expect(clippy::undocumented_unsafe_blocks)]

    profiling::function_scope!();

    if let Some(RawDisplayHandle::Wayland(display)) = raw_display_handle {
        log::trace!("Initializing smithay clipboard…");
        #[expect(unsafe_code)]
        Some(unsafe { smithay_clipboard::Clipboard::new(display.display.as_ptr()) })
    } else {
        #[cfg(feature = "wayland")]
        log::debug!("Cannot init smithay clipboard without a Wayland display handle");
        #[cfg(not(feature = "wayland"))]
        log::debug!(
            "Cannot init smithay clipboard: the 'wayland' feature of 'egui-winit' is not enabled"
        );
        None
    }
}

#[cfg(all(
    not(any(target_os = "android", target_os = "ios")),
    feature = "arboard",
))]
#[cfg(test)]
mod tests {
    use super::{color_image_from_arboard, is_expected_content_absence};

    /// Regression test for the spurious `error!`-level log a maintainer caught by manually
    /// testing an image paste (nothing had exercised this distinction before): only
    /// `ContentNotAvailable` — clipboard simply doesn't hold the requested content type — is
    /// expected and should stay silent; every other `arboard::Error` variant is a real failure
    /// and must still be logged.
    #[test]
    fn only_content_not_available_is_treated_as_expected() {
        assert!(is_expected_content_absence(
            &arboard::Error::ContentNotAvailable
        ));

        assert!(!is_expected_content_absence(
            &arboard::Error::ClipboardNotSupported
        ));
        assert!(!is_expected_content_absence(
            &arboard::Error::ClipboardOccupied
        ));
        assert!(!is_expected_content_absence(
            &arboard::Error::ConversionFailure
        ));
        assert!(!is_expected_content_absence(&arboard::Error::Unknown {
            description: "anything".to_owned(),
        }));
    }

    #[test]
    fn color_image_from_arboard_converts_straight_to_premultiplied_alpha() {
        // 2x1 image: opaque red, then half-transparent white — straight (unmultiplied) alpha,
        // as arboard/the OS clipboard would hand it to us.
        let image = arboard::ImageData {
            width: 2,
            height: 1,
            bytes: std::borrow::Cow::Borrowed(&[255, 0, 0, 255, 255, 255, 255, 128]),
        };
        let color_image = color_image_from_arboard(&image);
        assert_eq!(color_image.size, [2, 1]);
        assert_eq!(
            color_image.pixels[0],
            egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255)
        );
        assert_eq!(
            color_image.pixels[1],
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128)
        );
    }
}
