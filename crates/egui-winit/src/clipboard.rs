use winit::event_loop::EventLoopWindowTarget;

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
    /// Construct a new instance
    ///
    /// # Safety
    ///
    /// The returned `Clipboard` must not outlive the input `_event_loop`.
    pub fn new<T>(_event_loop: &EventLoopWindowTarget<T>) -> Self {
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
            smithay: init_smithay_clipboard(_event_loop),

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
fn init_smithay_clipboard<T>(
    _event_loop: &EventLoopWindowTarget<T>,
) -> Option<smithay_clipboard::Clipboard> {
    // Note: ideally "smithay-clipboard" would imply "wayland", but it doesn't.
    #[cfg(feature = "wayland")]
    {
        use winit::platform::wayland::EventLoopWindowTargetExtWayland as _;
        if let Some(display) = _event_loop.wayland_display() {
            tracing::debug!("Initializing smithay clipboard…");
            #[allow(unsafe_code)]
            Some(unsafe { smithay_clipboard::Clipboard::new(display) })
        } else {
            tracing::debug!("Cannot initialize smithay clipboard without a display handle");
            None
        }
    }

    #[cfg(not(feature = "wayland"))]
    {
        tracing::debug!(
            "You need to enable the 'wayland' feature of 'egui-winit' to get a working clipboard"
        );
        None
    }
}
