/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works works within the same app is used instead.
pub struct Clipboard {
    #[cfg(feature = "arboard")]
    arboard: Option<arboard::Clipboard>,

    /// Fallback manual clipboard.
    clipboard: String,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            #[cfg(feature = "arboard")]
            arboard: init_arboard(),

            clipboard: String::default(),
        }
    }
}

impl Clipboard {
    pub fn get(&mut self) -> Option<String> {
        #[cfg(feature = "arboard")]
        if let Some(clipboard) = &mut self.arboard {
            return match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(err) => {
                    tracing::error!("Paste error: {}", err);
                    None
                }
            };
        }

        Some(self.clipboard.clone())
    }

    pub fn set(&mut self, text: String) {
        #[cfg(feature = "arboard")]
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_text(text) {
                tracing::error!("Copy/Cut error: {}", err);
            }
            return;
        }

        self.clipboard = text;
    }
}

#[cfg(feature = "arboard")]
fn init_arboard() -> Option<arboard::Clipboard> {
    match arboard::Clipboard::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            tracing::error!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}
