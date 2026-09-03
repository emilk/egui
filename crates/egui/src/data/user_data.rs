use core::any::Any;
use std::sync::Arc;

use crate::{ColorImage, Event, ViewportId, mutex::Mutex};

type ScreenshotCallbackFn =
    dyn FnOnce(ViewportId, Arc<ColorImage>) -> Option<Event> + Send + 'static;

/// A wrapper around `dyn Any`, used for passing custom user data
/// to [`crate::ViewportCommand::Screenshot`].
#[derive(Clone, Debug, Default)]
pub struct UserData {
    /// A user value given to the screenshot command,
    /// that will be returned in [`crate::Event::Screenshot`].
    pub data: Option<Arc<dyn Any + Send + Sync>>,
}

impl UserData {
    /// You can also use [`Self::default`].
    pub fn new(user_info: impl Any + Send + Sync) -> Self {
        Self {
            data: Some(Arc::new(user_info)),
        }
    }
}

impl PartialEq for UserData {
    fn eq(&self, other: &Self) -> bool {
        match (&self.data, &other.data) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl Eq for UserData {}

impl core::hash::Hash for UserData {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.data.as_ref().map(Arc::as_ptr).hash(state);
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for UserData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none() // can't serialize an `Any`
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for UserData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UserDataVisitor;

        impl serde::de::Visitor<'_> for UserDataVisitor {
            type Value = UserData;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a None value")
            }

            fn visit_none<E>(self) -> Result<UserData, E>
            where
                E: serde::de::Error,
            {
                Ok(UserData::default())
            }
        }

        deserializer.deserialize_option(UserDataVisitor)
    }
}

/// A one-shot callback for receiving a screenshot.
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
            callback: Arc::new(Mutex::new(Some(Box::new(move |_viewport_id, image| {
                callback(image);
                None
            })))),
        }
    }

    /// Adapt the event-based screenshot API to the callback-based implementation.
    #[doc(hidden)]
    pub fn event(user_data: UserData) -> Self {
        Self {
            callback: Arc::new(Mutex::new(Some(Box::new(move |viewport_id, image| {
                Some(Event::Screenshot {
                    viewport_id,
                    user_data,
                    image,
                })
            })))),
        }
    }

    /// Complete this screenshot request.
    ///
    /// Callback requests are invoked immediately. Requests created for the legacy event-based
    /// API instead return the event that the integration should enqueue for the next pass.
    #[doc(hidden)]
    pub fn complete(self, viewport_id: ViewportId, image: Arc<ColorImage>) -> Option<Event> {
        let callback = self.callback.lock().take()?;
        callback(viewport_id, image)
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
