/// Speak the given text out loud.
pub fn speak(text: &str) {
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
                speech_synthesis.speak(&utterance);
            }
        }
    }
}
