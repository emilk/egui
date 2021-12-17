/// Configure padding of grid or table
/// TODO: Use padding settings of egui?
#[derive(Clone, Debug)]
pub struct Padding {
    pub(crate) inner: f32,
    pub(crate) outer: f32,
}

impl Padding {
    pub fn new(inner: f32, outer: f32) -> Self {
        Self { inner, outer }
    }

    pub fn inner(mut self, inner: f32) -> Self {
        self.inner = inner;
        self
    }

    pub fn outer(mut self, outer: f32) -> Self {
        self.outer = outer;
        self
    }
}

impl Default for Padding {
    fn default() -> Self {
        Self::new(5.0, 10.0)
    }
}
