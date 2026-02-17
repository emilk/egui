use eframe::Storage;
use log::warn;
use ron::ser::PrettyConfig;

use crate::state::AgentState;

/// Storage key used for persisting [`AgentState`].
pub const STORAGE_KEY: &str = "eframe_gui_agent/state";

/// Load state from eframe storage.
pub fn load_state_from_storage(storage: Option<&dyn Storage>) -> Option<AgentState> {
    let storage = storage?;
    let raw = storage.get_string(STORAGE_KEY)?;
    match ron::from_str::<AgentState>(&raw) {
        Ok(state) => Some(state),
        Err(err) => {
            warn!("Failed to restore agent state: {err}");
            None
        }
    }
}

/// Persist state to the provided storage.
pub fn save_state_to_storage(storage: &mut dyn Storage, state: &AgentState) {
    match ron::ser::to_string_pretty(state, PrettyConfig::new()) {
        Ok(serialized) => storage.set_string(STORAGE_KEY, serialized),
        Err(err) => warn!("Failed to serialize agent state: {err}"),
    }
}
