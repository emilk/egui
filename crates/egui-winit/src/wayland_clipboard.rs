//! Share winit's data device instead of starting a second clipboard device.
use std::collections::HashMap;
use winit::{
    data_transfer::{DataTransferId, DataTransferSendBuilder, TypeHint},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, AsyncRequestSerial},
};
#[derive(Default)]
pub(crate) struct Clipboard {
    pending: HashMap<AsyncRequestSerial, (DataTransferId, Option<egui::Pos2>)>,
}

impl Clipboard {
    pub fn paste(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        primary_at: Option<egui::Pos2>,
    ) -> bool {
        let result = if primary_at.is_some() {
            event_loop
                .primary_selection_ext()
                .map(|selection| selection.primary_selection())
        } else {
            Some(event_loop.clipboard())
        };
        let Some(Ok(Some(id))) = result else {
            return false;
        };
        if self.pending.len() >= 8
            || !event_loop
                .data_transfer(id)
                .is_ok_and(|data| data.has_type(&TypeHint::Plaintext))
        {
            return false;
        }
        match event_loop.fetch_data_transfer_with_limits(
            id,
            &TypeHint::Plaintext,
            winit::data_transfer::DataTransferLimits {
                max_bytes: 1024 * 1024,
                timeout: core::time::Duration::from_secs(10),
            },
        ) {
            Ok(serial) => {
                self.pending.insert(serial, (id, primary_at));
                true
            }
            Err(_) => false,
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) -> Option<egui::Event> {
        let WindowEvent::DataTransferReceived { id, serial, value } = event else {
            return None;
        };
        let (expected_id, primary_at) = self.pending.get(serial)?;
        if expected_id != id {
            return None;
        }
        match value.try_as_string() {
            Ok(text) => {
                let primary_at = *primary_at;
                self.pending.remove(serial);
                if text.is_empty() || text.len() > 1024 * 1024 {
                    return None;
                }
                Some(match primary_at {
                    Some(pos) => egui::Event::MiddleClickPaste { pos, text },
                    None => egui::Event::Paste(text.replace("\r\n", "\n")),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => None,
            Err(_) => {
                self.pending.remove(serial);
                None
            }
        }
    }

    pub fn copy(event_loop: &dyn ActiveEventLoop, text: String, primary: bool) -> bool {
        let data = DataTransferSendBuilder::new(text)
            .with_type(TypeHint::Plaintext, |text, _| Some(text.clone()))
            .build();
        if primary {
            event_loop
                .primary_selection_ext()
                .is_some_and(|selection| selection.set_primary_selection(data).is_ok())
        } else {
            event_loop.set_clipboard(data).is_ok()
        }
    }
}
