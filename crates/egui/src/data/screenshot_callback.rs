use std::sync::Arc;

use crate::{ColorImage, mutex::Mutex};

type ScreenshotCallbackFn = dyn FnOnce(Arc<ColorImage>) + Send + 'static;

/// A one-shot callback for receiving a screenshot.
///
/// Create one with [`Self::new`] and send it with [`crate::ViewportCommand::Screenshot`],
/// or use the convenience method [`crate::Context::request_screenshot`].
///
/// Clones share the same callback. Whichever clone is completed first consumes it, so it is
/// invoked at most once.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ScreenshotCallback {
    #[cfg_attr(feature = "serde", serde(skip))]
    callback: Arc<Mutex<Option<Box<ScreenshotCallbackFn>>>>,
}

impl ScreenshotCallback {
    /// Create a callback that will be invoked once the screenshot is ready.
    ///
    /// This doesn't request a new frame when the data arrives. Call `ctx.request_repaint` if needed.
    pub fn new(callback: impl FnOnce(Arc<ColorImage>) + Send + 'static) -> Self {
        Self {
            callback: Arc::new(Mutex::new(Some(Box::new(callback)))),
        }
    }

    /// Complete this screenshot request, invoking the callback.
    ///
    /// Does nothing if the request was already completed.
    pub fn complete(self, image: Arc<ColorImage>) {
        let Some(callback) = self.callback.lock().take() else {
            return;
        };
        callback(image);
    }
}

impl core::fmt::Debug for ScreenshotCallback {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ScreenshotCallback")
            .field("called", &self.callback.lock().is_none())
            .finish()
    }
}

// The callback itself has no meaningful identity as viewport-command data.
impl PartialEq for ScreenshotCallback {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.callback, &other.callback)
    }
}

impl Eq for ScreenshotCallback {}
