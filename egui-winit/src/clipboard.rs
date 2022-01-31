/// Handles interfacing either with the OS clipboard.
/// If the "clipboard" feature is off it will instead simulate the clipboard locally.
pub struct Clipboard {
    #[cfg(feature = "copypasta")]
    copypasta: Option<copypasta::ClipboardContext>,

    /// Fallback manual clipboard.
    #[cfg(not(feature = "copypasta"))]
    clipboard: String,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            #[cfg(feature = "copypasta")]
            copypasta: init_copypasta(),

            #[cfg(not(feature = "copypasta"))]
            clipboard: String::default(),
        }
    }
}

impl Clipboard {
    pub fn get(&mut self) -> Option<String> {
        #[cfg(feature = "copypasta")]
        if let Some(clipboard) = &mut self.copypasta {
            use copypasta::ClipboardProvider as _;
            match clipboard.get_contents() {
                Ok(contents) => Some(contents),
                Err(err) => {
                    tracing::error!("Paste error: {}", err);
                    None
                }
            }
        } else {
            None
        }

        #[cfg(not(feature = "copypasta"))]
        Some(self.clipboard.clone())
    }

    pub fn set(&mut self, text: String) {
        #[cfg(feature = "copypasta")]
        if let Some(clipboard) = &mut self.copypasta {
            use copypasta::ClipboardProvider as _;
            if let Err(err) = clipboard.set_contents(text) {
                tracing::error!("Copy/Cut error: {}", err);
            }
        }

        #[cfg(not(feature = "copypasta"))]
        {
            self.clipboard = text;
        }
    }
}

#[cfg(feature = "copypasta")]
fn init_copypasta() -> Option<copypasta::ClipboardContext> {
    match copypasta::ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            tracing::error!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}
