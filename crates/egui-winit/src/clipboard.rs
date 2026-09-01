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
            smithay: Self::init_smithay(_raw_display_handle),

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
                    log::error!("arboard paste error: {err}");
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

// The backends that only exist on X11 and Wayland. Everything below is written
// twice, once for real and once as a no-op, so that the rest of this file can
// call it without repeating the list of operating systems.

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#[cfg_attr(
    not(any(feature = "arboard", feature = "smithay-clipboard")),
    expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        clippy::unnecessary_wraps,
        reason = "these do nothing without a clipboard backend to talk to"
    )
)]
impl Clipboard {
    #[cfg(feature = "smithay-clipboard")]
    fn init_smithay(
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

    /// `Err` if there is no smithay clipboard; `Ok(None)` if reading failed.
    fn smithay_get(&mut self, _selection: Selection) -> Result<Option<String>, Unavailable> {
        #[cfg(feature = "smithay-clipboard")]
        if let Some(clipboard) = &mut self.smithay {
            let read = match _selection {
                Selection::Clipboard => clipboard.load(),
                Selection::Primary => clipboard.load_primary(),
            };

            return Ok(match read {
                Ok(text) => Some(text),
                Err(err) => {
                    log::debug!("smithay paste error: {err}");
                    None
                }
            });
        }

        Err(Unavailable)
    }

    /// Returns the text back if there is no smithay clipboard to take it.
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

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
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
