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

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn clones_share_one_callback() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in_callback = Arc::clone(&calls);
        let callback = ScreenshotCallback::new(move |_| {
            calls_in_callback.fetch_add(1, Ordering::Relaxed);
        });

        let clone = callback.clone();
        let image = Arc::new(ColorImage::default());

        assert!(
            clone
                .complete(ViewportId::ROOT, Arc::clone(&image))
                .is_none()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        assert!(callback.complete(ViewportId::ROOT, image).is_none());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn event_adapter_returns_a_screenshot_event() {
        let user_data = UserData::new("request");
        let callback = ScreenshotCallback::event(user_data.clone());
        let image = Arc::new(ColorImage::default());

        let Some(Event::Screenshot {
            viewport_id,
            user_data: returned_user_data,
            image: returned_image,
        }) = callback.complete(ViewportId::ROOT, Arc::clone(&image))
        else {
            panic!("expected a screenshot event");
        };

        assert_eq!(viewport_id, ViewportId::ROOT);
        assert_eq!(returned_user_data, user_data);
        assert!(Arc::ptr_eq(&returned_image, &image));
    }
}
