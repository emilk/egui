#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoolState {
    pub init: bool,
    pub state: bool,
}

impl Default for BoolState {
    fn default() -> Self {
        Self {
            init: true,
            state: false,
        }
    }
}

impl BoolState {
    pub fn new(self, init: bool, state: bool) -> Self {
        Self { init, state }
    }

    pub fn toggle(&mut self) {
        self.state = !self.state;
    }

    pub fn is_allowed(&self) -> bool {
        self.init && self.state == true
    }
}
