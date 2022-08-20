use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Settings {
    /// Maximum number of undos.
    /// If your state is resource intensive, you should keep this low.
    ///
    /// Default: `100`
    pub max_undos: usize,

    /// When that state hasn't changed for this many seconds,
    /// create a new undo point (if one is needed).
    ///
    /// Default value: `1.0` seconds.
    pub stable_time: f32,

    /// If the state is changing so often that we never get to `stable_time`,
    /// then still create a save point every `auto_save_interval` seconds,
    /// so we have something to undo to.
    ///
    /// Default value: `30` seconds.
    pub auto_save_interval: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_undos: 100,
            stable_time: 1.0,
            auto_save_interval: 30.0,
        }
    }
}

/// Automatic undo system.
///
/// Every frame you feed it the most recent state.
/// The [`Undoer`] compares it with the latest undo point
/// and if there is a change it may create a new undo point.
///
/// [`Undoer`] follows two simple rules:
///
/// 1) If the state has changed since the latest undo point, but has
///    remained stable for `stable_time` seconds, an new undo point is created.
/// 2) If the state does not stabilize within `auto_save_interval` seconds, an undo point is created.
///
/// Rule 1) will make sure an undo point is not created until you _stop_ dragging that slider.
/// Rule 2) will make sure that you will get some undo points even if you are constantly changing the state.
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Undoer<State> {
    settings: Settings,

    /// New undoes are added to the back.
    /// Two adjacent undo points are never equal.
    /// The latest undo point may (often) be the current state.
    undos: VecDeque<State>,

    #[cfg_attr(feature = "serde", serde(skip))]
    flux: Option<Flux<State>>,
}

impl<State> std::fmt::Debug for Undoer<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { undos, .. } = self;
        f.debug_struct("Undoer")
            .field("undo count", &undos.len())
            .finish()
    }
}

/// Represents how the current state is changing
#[derive(Clone)]
struct Flux<State> {
    start_time: f64,
    latest_change_time: f64,
    latest_state: State,
}

impl<State> Undoer<State>
where
    State: Clone + PartialEq,
{
    /// Do we have an undo point different from the given state?
    pub fn has_undo(&self, current_state: &State) -> bool {
        match self.undos.len() {
            0 => false,
            1 => self.undos.back() != Some(current_state),
            _ => true,
        }
    }

    /// Return true if the state is currently changing
    pub fn is_in_flux(&self) -> bool {
        self.flux.is_some()
    }

    pub fn undo(&mut self, current_state: &State) -> Option<&State> {
        if self.has_undo(current_state) {
            self.flux = None;

            if self.undos.back() == Some(current_state) {
                self.undos.pop_back();
            }

            // Note: we keep the undo point intact.
            self.undos.back()
        } else {
            None
        }
    }

    /// Add an undo point if, and only if, there has been a change since the latest undo point.
    ///
    /// * `time`: current time in seconds.
    pub fn add_undo(&mut self, current_state: &State) {
        if self.undos.back() != Some(current_state) {
            self.undos.push_back(current_state.clone());
        }
        while self.undos.len() > self.settings.max_undos {
            self.undos.pop_front();
        }
        self.flux = None;
    }

    /// Call this as often as you want (e.g. every frame)
    /// and [`Undoer`] will determine if a new undo point should be created.
    ///
    /// * `current_time`: current time in seconds.
    pub fn feed_state(&mut self, current_time: f64, current_state: &State) {
        match self.undos.back() {
            None => {
                // First time feed_state is called.
                // always create an undo point:
                self.add_undo(current_state);
            }
            Some(latest_undo) => {
                if latest_undo == current_state {
                    self.flux = None;
                } else {
                    match self.flux.as_mut() {
                        None => {
                            self.flux = Some(Flux {
                                start_time: current_time,
                                latest_change_time: current_time,
                                latest_state: current_state.clone(),
                            });
                        }
                        Some(flux) => {
                            if &flux.latest_state == current_state {
                                let time_since_latest_change =
                                    (current_time - flux.latest_change_time) as f32;
                                if time_since_latest_change >= self.settings.stable_time {
                                    self.add_undo(current_state);
                                }
                            } else {
                                let time_since_flux_start = (current_time - flux.start_time) as f32;
                                if time_since_flux_start >= self.settings.auto_save_interval {
                                    self.add_undo(current_state);
                                } else {
                                    flux.latest_change_time = current_time;
                                    flux.latest_state = current_state.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
