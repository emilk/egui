fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Read data from local storage.
pub fn local_storage_get(key: &str) -> Option<String> {
    local_storage().map(|storage| storage.get_item(key).ok())??
}

/// Write data to local storage.
pub fn local_storage_set(key: &str, value: &str) {
    local_storage().map(|storage| storage.set_item(key, value));
}

#[cfg(feature = "persistence")]
pub(crate) fn load_memory(ctx: &egui::Context) {
    if let Some(memory_string) = local_storage_get("egui_memory_ron") {
        match ron::from_str(&memory_string) {
            Ok(memory) => {
                ctx.memory_mut(|m| *m = memory);
            }
            Err(err) => {
                log::warn!("Failed to parse memory RON: {err}");
            }
        }
    }
}

#[cfg(not(feature = "persistence"))]
pub(crate) fn load_memory(_: &egui::Context) {}

#[cfg(feature = "persistence")]
pub(crate) fn save_memory(ctx: &egui::Context) {
    match ctx.memory(ron::to_string) {
        Ok(ron) => {
            local_storage_set("egui_memory_ron", &ron);
        }
        Err(err) => {
            log::warn!("Failed to serialize memory as RON: {err}");
        }
    }
}

#[cfg(not(feature = "persistence"))]
pub(crate) fn save_memory(_: &egui::Context) {}
