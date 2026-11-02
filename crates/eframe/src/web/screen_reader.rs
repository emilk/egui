/// Speak the given text out loud, in `lang` (a BCP 47 tag) if given.
pub fn speak(text: &str, lang: Option<&str>) {
    if text.is_empty() {
        return;
    }

    if let Some(window) = web_sys::window() {
        log::debug!("Speaking {text:?}");

        if let Ok(speech_synthesis) = window.speech_synthesis() {
            speech_synthesis.cancel(); // interrupt previous speech, if any

            if let Ok(utterance) = web_sys::SpeechSynthesisUtterance::new_with_text(text) {
                utterance.set_rate(1.0);
                utterance.set_pitch(1.0);
                utterance.set_volume(1.0);
                if let Some(lang) = lang {
                    // Otherwise the browser picks a voice for the page's `lang`, or its own language.
                    utterance.set_lang(lang);
                }
                speech_synthesis.speak(&utterance);
            }
        }
    }
}
