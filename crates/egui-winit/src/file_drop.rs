//! Translate winit's asynchronous file transfers into egui input.
use std::{collections::HashMap, path::PathBuf};
use winit::{
    data_transfer::{DataTransferId, TypeHint},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, AsyncRequestSerial, DndAction},
};

#[derive(Default)]
pub(crate) struct FileDrops {
    pending: HashMap<DataTransferId, Pending>,
}

struct Pending {
    serial: Option<AsyncRequestSerial>,
    position: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl FileDrops {
    pub fn has_hover(&self) -> bool {
        self.pending
            .values()
            .any(|pending| pending.serial.is_none())
    }

    pub fn on_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        event: &WindowEvent,
        bounded: bool,
    ) -> Option<(Vec<PathBuf>, Option<winit::dpi::PhysicalPosition<f64>>)> {
        match event {
            WindowEvent::DragEntered { id, position } => {
                let supported = self.pending.len() < 8
                    && event_loop
                        .data_transfer(*id)
                        .is_ok_and(|data| data.has_type(&TypeHint::UriList));
                let actions = if supported {
                    &[DndAction::Copy][..]
                } else {
                    &[]
                };
                if event_loop.set_valid_dnd_actions(*id, actions).is_ok() && supported {
                    self.pending.insert(
                        *id,
                        Pending {
                            serial: None,
                            position: *position,
                        },
                    );
                }
            }
            WindowEvent::DragPosition { id, position, .. } => {
                if let Some(pending) = self.pending.get_mut(id) {
                    pending.position = Some(*position);
                }
            }
            WindowEvent::DragDropped {
                id,
                proposed_action,
            } => {
                if proposed_action.is_some_and(|action| action != DndAction::Copy) {
                    self.pending.remove(id);
                } else if let Some(pending) = self.pending.get_mut(id) {
                    let request = if bounded {
                        event_loop.fetch_data_transfer_with_limits(
                            *id,
                            &TypeHint::UriList,
                            winit::data_transfer::DataTransferLimits {
                                max_bytes: 1024 * 1024,
                                timeout: core::time::Duration::from_secs(10),
                            },
                        )
                    } else {
                        event_loop.fetch_data_transfer(*id, &TypeHint::UriList)
                    };
                    match request {
                        Ok(serial) => pending.serial = Some(serial),
                        Err(_) => {
                            self.pending.remove(id);
                        }
                    }
                }
            }
            WindowEvent::DragLeft { id }
                if self.pending.get(id).is_some_and(|p| p.serial.is_none()) =>
            {
                self.pending.remove(id);
            }
            WindowEvent::DataTransferReceived { id, serial, value } => {
                let pending = self.pending.get(id)?;
                if pending.serial != Some(*serial) {
                    return None;
                }
                match value.try_as_uris().and_then(|uris| {
                    parse_uris(&uris).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid local file list",
                        )
                    })
                }) {
                    Ok(paths) => {
                        let position = self.pending.remove(id)?.position;
                        if valid_paths(&paths) {
                            return Some((paths, position));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => {
                        self.pending.remove(id);
                    }
                }
            }
            WindowEvent::Destroyed => self.pending.clear(),
            _ => {}
        }
        None
    }
}

fn valid_paths(paths: &[PathBuf]) -> bool {
    !paths.is_empty()
        && paths.len() <= 256
        && paths
            .iter()
            .all(|p| p.is_absolute() && !p.as_os_str().as_encoded_bytes().contains(&0))
        && paths.iter().map(|p| p.as_os_str().len()).sum::<usize>() <= 1024 * 1024
}

fn parse_uris(uris: &[String]) -> Option<Vec<PathBuf>> {
    if uris.is_empty()
        || uris.len() > 256
        || uris.iter().map(String::len).sum::<usize>() > 1024 * 1024
    {
        return None;
    }
    let paths: Vec<PathBuf> = uris
        .iter()
        .filter(|uri| !uri.is_empty() && !uri.starts_with('#'))
        .map(|uri| {
            let bytes = uri.as_bytes();
            for (index, byte) in bytes.iter().enumerate() {
                if *byte == b'%'
                    && (bytes.get(index + 1).is_none_or(|b| !b.is_ascii_hexdigit())
                        || bytes.get(index + 2).is_none_or(|b| !b.is_ascii_hexdigit()))
                {
                    return None;
                }
            }
            let url = url::Url::parse(uri).ok()?;
            if url.scheme() != "file"
                || url.query().is_some()
                || url.fragment().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
                || url
                    .host_str()
                    .is_some_and(|host| !host.is_empty() && host != "localhost")
            {
                return None;
            }
            url.to_file_path().ok()
        })
        .collect::<Option<_>>()?;
    valid_paths(&paths).then_some(paths)
}

#[cfg(test)]
mod tests {
    #[test]
    fn file_paths_are_local_and_bounded() {
        use std::path::PathBuf;
        let root = std::path::absolute("image with space.png").unwrap();
        assert!(super::valid_paths(core::slice::from_ref(&root)));
        assert!(!super::valid_paths(&[]));
        assert!(!super::valid_paths(&[PathBuf::from("relative.png")]));
        assert!(!super::valid_paths(&vec![root; 257]));
    }

    #[test]
    fn file_uris_reject_remote_credentials_queries_and_invalid_escapes() {
        for uri in [
            "https://example.com/image.png",
            "file://remote/image.png",
            "file://user@localhost/image.png",
            "file:///image.png?query=1",
            "file:///image.png#fragment",
            "file:///image%00.png",
            "file:///image%GG.png",
            "file:///image%.png",
        ] {
            assert!(super::parse_uris(&[uri.into()]).is_none(), "{uri}");
        }
    }

    #[test]
    fn file_uris_preserve_spaces_and_unicode() {
        let path = std::path::absolute("image with space ✓.png").unwrap();
        let uri = url::Url::from_file_path(&path).unwrap();
        assert_eq!(super::parse_uris(&[uri.into()]), Some(vec![path]));
    }
}
