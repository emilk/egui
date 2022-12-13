use std::os::raw::c_void;

/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works works within the same app is used instead.
pub struct Clipboard {
    #[cfg(all(feature = "arboard", not(target_os = "android")))]
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
    #[allow(unused_variables)]
    pub fn new(#[allow(unused_variables)] wayland_display: Option<*mut c_void>) -> Self {
        Self {
            #[cfg(all(feature = "arboard", not(target_os = "android")))]
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
            smithay: init_smithay_clipboard(wayland_display),

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
            return match clipboard.load() {
                Ok(text) => Some(text),
                Err(err) => {
                    tracing::error!("smithay paste error: {err}");
                    None
                }
            };
        }

        #[cfg(all(feature = "arboard", not(target_os = "android")))]
        if let Some(clipboard) = &mut self.arboard {
            return match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(err) => {
                    tracing::error!("arboard paste error: {err}");
                    None
                }
            };
        }

        Some(self.clipboard.clone())
    }

    pub fn set(&mut self, text: String) {
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

        #[cfg(all(feature = "arboard", not(target_os = "android")))]
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_text(text) {
                tracing::error!("arboard copy/cut error: {err}");
            }
            return;
        }

        self.clipboard = text;
    }
}

#[cfg(all(feature = "arboard", not(target_os = "android")))]
fn init_arboard() -> Option<arboard::Clipboard> {
    tracing::debug!("Initializing arboard clipboard…");
    match arboard::Clipboard::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            tracing::warn!("Failed to initialize arboard clipboard: {err}");
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
    wayland_display: Option<*mut c_void>,
) -> Option<smithay_clipboard::Clipboard> {
    if let Some(display) = wayland_display {
        tracing::debug!("Initializing smithay clipboard…");
        #[allow(unsafe_code)]
        Some(unsafe { smithay_clipboard::Clipboard::new(display) })
    } else {
        tracing::debug!("Cannot initialize smithay clipboard without a display handle");
        None
    }
}
