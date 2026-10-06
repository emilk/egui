use raw_window_handle::RawDisplayHandle;

/// The X11/Wayland selection to operate on.
///
/// `CLIPBOARD` is what Ctrl+C and Ctrl+V use. `PRIMARY` is filled in by merely
/// selecting text, and pasted with the middle mouse button.
#[derive(Clone, Copy)]
enum Selection {
    Clipboard,
    Primary,
}

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
    /// Construct a new instance.
    ///
    /// # Safety
    ///
    /// If `raw_display_handle` is `Some`, the display handle must remain valid for the
    /// entire lifetime of the returned `Clipboard` instance.
    #[expect(unsafe_code)]
    pub unsafe fn new(_raw_display_handle: Option<RawDisplayHandle>) -> Self {
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
            // SAFETY: The caller guarantees that the display handle remains valid.
            smithay: unsafe { Self::init_smithay(_raw_display_handle) },

            clipboard: Default::default(),
        }
    }

    pub fn get(&mut self) -> Option<String> {
        // On a smithay read error we fall through to arboard rather than give up.
        if let Ok(Some(text)) = self.smithay_get(Selection::Clipboard) {
            return Some(text);
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
        let Some(text) = self.smithay_set(Selection::Clipboard, text) else {
            return;
        };

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

    /// Read the X11/Wayland PRIMARY selection.
    ///
    /// Returns `None` on platforms without a PRIMARY selection, and when
    /// nothing owns it.
    pub fn get_primary_text(&mut self) -> Option<String> {
        if let Ok(text) = self.smithay_get(Selection::Primary) {
            return text;
        }

        self.arboard_get_primary()
    }

    /// Set the X11/Wayland PRIMARY selection, which is pasted with the middle
    /// mouse button.
    ///
    /// The selection is served from this process for as long as this
    /// [`Clipboard`] is alive, which is the same lifetime every other
    /// application gives PRIMARY.
    ///
    /// Does nothing on platforms without a PRIMARY selection.
    pub fn set_primary_text(&mut self, text: String) {
        if let Some(text) = self.smithay_set(Selection::Primary, text) {
            // Unlike the clipboard there is no in-app fallback worth having:
            // PRIMARY only exists to be read by other processes.
            self.arboard_set_primary(text);
        }
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

/// There is no such backend, in this build or on this platform.
struct Unavailable;

// The backends that only exist on X11 and Wayland, written once for real and
// once as a no-op, so the rest of this file can call them without repeating the
// list of operating systems.
cfg_select! {
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ) => {
        impl Clipboard {
            /// # Safety
            ///
            /// The display handle in `raw_display_handle` must remain valid for the
            /// lifetime of the returned `Clipboard`.
            #[cfg(feature = "smithay-clipboard")]
            #[expect(unsafe_code)]
            unsafe fn init_smithay(
                raw_display_handle: Option<RawDisplayHandle>,
            ) -> Option<smithay_clipboard::Clipboard> {
                profiling::function_scope!();

                if let Some(RawDisplayHandle::Wayland(display)) = raw_display_handle {
                    log::trace!("Initializing smithay clipboard…");
                    // SAFETY: The caller guarantees that the display handle remains valid.
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

            /// `Err` if there is no smithay clipboard; `Ok(None)` if reading failed.
            #[cfg_attr(
                not(feature = "smithay-clipboard"),
                expect(
                    clippy::unused_self,
                    clippy::needless_pass_by_ref_mut,
                    reason = "does nothing without this backend"
                )
            )]
            fn smithay_get(&mut self, _selection: Selection) -> Result<Option<String>, Unavailable> {
                #[cfg(feature = "smithay-clipboard")]
                if let Some(clipboard) = &mut self.smithay {
                    let read = match _selection {
                        Selection::Clipboard => clipboard.load(),
                        Selection::Primary => clipboard.load_primary(),
                    };

                    return Ok(match read {
                        Ok(text) => Some(text),
                        // Smithay uses NotFound when no supported text MIME type is offered.
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                        Err(err) => {
                            // Not fatal: the caller falls back to arboard.
                            log::debug!("smithay paste error: {err}");
                            None
                        }
                    });
                }

                Err(Unavailable)
            }

            /// Returns the text back if there is no smithay clipboard to take it.
            #[cfg_attr(
                not(feature = "smithay-clipboard"),
                expect(
                    clippy::unused_self,
                    clippy::needless_pass_by_ref_mut,
                    clippy::unnecessary_wraps,
                    reason = "does nothing without this backend"
                )
            )]
            fn smithay_set(&mut self, _selection: Selection, _text: String) -> Option<String> {
                #[cfg(feature = "smithay-clipboard")]
                if let Some(clipboard) = &mut self.smithay {
                    match _selection {
                        Selection::Clipboard => clipboard.store(_text),
                        Selection::Primary => clipboard.store_primary(_text),
                    }
                    return None;
                }

                Some(_text)
            }

            #[cfg_attr(
                not(feature = "arboard"),
                expect(
                    clippy::unused_self,
                    clippy::needless_pass_by_ref_mut,
                    reason = "does nothing without this backend"
                )
            )]
            fn arboard_get_primary(&mut self) -> Option<String> {
                #[cfg(feature = "arboard")]
                if let Some(clipboard) = &mut self.arboard {
                    use arboard::GetExtLinux as _;

                    return match clipboard
                        .get()
                        .clipboard(arboard::LinuxClipboardKind::Primary)
                        .text()
                    {
                        Ok(text) => Some(text),
                        Err(err) => {
                            // An empty PRIMARY selection is the normal state, not an
                            // error worth shouting about.
                            log::debug!("arboard primary selection paste error: {err}");
                            None
                        }
                    };
                }

                None
            }

            #[cfg_attr(
                not(feature = "arboard"),
                expect(
                    clippy::unused_self,
                    clippy::needless_pass_by_ref_mut,
                    reason = "does nothing without this backend"
                )
            )]
            fn arboard_set_primary(&mut self, _text: String) {
                #[cfg(feature = "arboard")]
                if let Some(clipboard) = &mut self.arboard {
                    use arboard::SetExtLinux as _;

                    if let Err(err) = clipboard
                        .set()
                        .clipboard(arboard::LinuxClipboardKind::Primary)
                        .text(_text)
                    {
                        log::error!("arboard primary selection error: {err}");
                    }
                }
            }
        }
    }
    _ => {
        #[expect(
            clippy::unused_self,
            clippy::needless_pass_by_ref_mut,
            clippy::unnecessary_wraps,
            reason = "these mirror the real implementations above"
        )]
        impl Clipboard {
            fn smithay_get(&mut self, _selection: Selection) -> Result<Option<String>, Unavailable> {
                Err(Unavailable)
            }

            fn smithay_set(&mut self, _selection: Selection, text: String) -> Option<String> {
                Some(text)
            }

            fn arboard_get_primary(&mut self) -> Option<String> {
                None
            }

            fn arboard_set_primary(&mut self, _text: String) {}
        }
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
