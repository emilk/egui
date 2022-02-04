pub struct ScreenReader {
    #[cfg(all(feature = "screen_reader", not(target_os = "linux")))]
    tts: Option<tts::Tts>,
}

#[cfg(any(target_os = "linux", not(feature = "screen_reader")))]
impl Default for ScreenReader {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(feature = "screen_reader", not(target_os = "linux")))]
impl Default for ScreenReader {
    fn default() -> Self {
        let tts = match tts::Tts::default() {
            Ok(screen_reader) => {
                tracing::debug!("Initialized screen reader.");
                Some(screen_reader)
            }
            Err(err) => {
                tracing::warn!("Failed to load screen reader: {}", err);
                None
            }
        };
        Self { tts }
    }
}

impl ScreenReader {
    #[cfg(any(target_os = "linux", not(feature = "screen_reader")))]
    #[allow(clippy::unused_self)]
    pub fn speak(&mut self, _text: &str) {}

    #[cfg(all(feature = "screen_reader", not(target_os = "linux")))]
    pub fn speak(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(tts) = &mut self.tts {
            tracing::debug!("Speaking: {:?}", text);
            let interrupt = true;
            if let Err(err) = tts.speak(text, interrupt) {
                tracing::warn!("Failed to read: {}", err);
            }
        }
    }
}
