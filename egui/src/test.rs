use std::default::Default;
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Test {
    pub value: u8,
}

impl Test {
    pub(crate) fn copy_test(&mut self, test: &Test) {
        self.value = test.value;
    }
}
