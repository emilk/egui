use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A file picker
/// * prevents multiple concurrent pick operations
/// * provides an API convinient for UI usage (see `is_picking` and `picked`)
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
        std::thread::spawn(move || {
            let mut guard = picker.lock().unwrap();
            *guard = (
                true,
                rfd::FileDialog::new()
                    .pick_file()
                    .map(std::path::PathBuf::from),
            );
        });
    }

    /// when picked, returns true, and the result of the pick, which may be None
    /// otherwise returns false
    ///
    /// this method is designed to be very fast while the picker is not picking (pending)
    pub fn picked(&mut self) -> (bool, Option<PathBuf>) {
        let mut was_picked = false;

        let return_value = match &mut self.state {
            PickerState::Picking(arc) => {
                if let Ok(mut guard) = arc.try_lock() {
                    match &mut *guard {
                        (true, picked) => {
                            was_picked = true;
                            let result = picked.take();
                            (true, result)
                        }
                        (false, _) => (false, None),
                    }
                } else {
                    (false, None)
                }
            }
            _ => (false, None),
        };

        if was_picked {
            // this causes the arc and mutex to be dropped, ready for the next pick.
            self.state = PickerState::Pending;
        }

        return_value
    }
}
