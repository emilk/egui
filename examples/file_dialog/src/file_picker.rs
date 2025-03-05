use eframe::egui::mutex::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

/// A file picker
/// * prevents multiple concurrent pick operations
/// * provides an API convenient for UI usage (see `is_picking` and `picked`)
///
/// Currently only picks files, but the API could be expanded.
#[derive(Default)]
pub struct Picker {
    state: PickerState,
}

#[derive(Default)]
enum PickerState {
    #[default]
    Pending,
    // use a boolean to indicate of picking has completed
    Picking(Arc<Mutex<(bool, Option<PathBuf>)>>),
}

#[derive(Clone, PartialEq)]
pub enum PickError {
    NotPicking,
    InProgress,
    Cancelled,
}

impl Picker {
    pub fn is_picking(&self) -> bool {
        matches!(self.state, PickerState::Picking(_))
    }

    pub fn pick_file(&mut self) {
        // initialise the boolean flag in the mutex to false, so that when the main thread continues it can see a
        // file has not been picked yet.  note that the mutex may not be locked until the picker thread starts to run
        // and lock it.
        let picker = Arc::new(Mutex::new((false, None)));
        self.state = PickerState::Picking(picker.clone());
        std::thread::Builder::new()
            .name("picker".to_owned())
            .spawn(move || {
                let mut guard = picker.lock();
                *guard = (
                    true,
                    rfd::FileDialog::new()
                        .pick_file()
                        .map(std::path::PathBuf::from),
                );
            })
            .unwrap();
    }

    /// when picked, returns the picked path, or an error indicating the reason
    ///
    /// this method is designed to be very fast while the picker is not picking (pending)
    pub fn picked(&mut self) -> Result<PathBuf, PickError> {
        let mut was_picked = false;

        let result = match &mut self.state {
            PickerState::Picking(arc) => {
                if let Some(mut guard) = arc.try_lock() {
                    match &mut *guard {
                        (true, picked) => {
                            was_picked = true;
                            match picked.take() {
                                Some(picked_path) => Ok(picked_path),
                                None => Err(PickError::Cancelled),
                            }
                        }
                        // arc not locked, but not picked yet either
                        (false, _) => Err(PickError::InProgress),
                    }
                } else {
                    // not picked yet, arc locked by thread
                    Err(PickError::InProgress)
                }
            }
            PickerState::Pending =>
            // not picked
            {
                Err(PickError::NotPicking)
            }
        };

        if was_picked {
            // this causes the arc and mutex to be dropped, ready for the next pick.
            self.state = PickerState::Pending;
        }

        result
    }
}
