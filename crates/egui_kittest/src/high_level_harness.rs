use crate::{Harness, HarnessBuilder, Node};
use egui::mutex::Mutex;
use kittest::{By, Queryable};
use std::sync::Arc;

#[derive(Clone)]
pub struct HighLevelHarness<'a>(Arc<Mutex<Harness<'a>>>);
pub struct HighLevelQuery<'a> {
    query: By<'a>,
    harness: HighLevelHarness<'a>,
}

impl<'a> HighLevelQuery<'a> {
    pub fn query<T>(&self, f: impl FnOnce(Node<'_>) -> T) -> Option<T> {
        self.harness.0.lock().query(self.query.clone()).map(f)
    }

    pub fn get<T>(&self, f: impl FnOnce(Node<'_>) -> T) -> T {
        f(self.harness.0.lock().get(self.query.clone()))
    }

    // pub fn query_all<T>(&self, f: impl FnMut(Node<'_>) -> T) -> impl DoubleEndedIterator<Item = T> {
    //     self.harness.0.lock().query_all(self.query.clone()).map(f)
    // }

    pub fn click(&self) {
        self.get(|n| n.click());
        self.harness.run();
    }

    pub fn assert_exists(&self) {
        self.get(|_| ());
    }
}

impl<'a> HighLevelHarness<'a> {
    pub fn new(harness: Harness<'a>) -> Self {
        Self(Arc::new(Mutex::new(harness)))
    }

    pub fn by_label(&self, label: &'a str) -> HighLevelQuery<'a> {
        HighLevelQuery {
            query: By::new().label(label),
            harness: self.clone(),
        }
    }

    pub fn event(&self, event: egui::Event) {
        let mut harness = self.0.lock();
        harness.event(event);
        harness.run();
    }

    pub fn run(&self) {
        self.0.lock().run();
    }
}

#[cfg(test)]
mod test {
    use crate::Harness;
    use crate::high_level_harness::HighLevelHarness;

    #[test]
    fn integration_test() {
        let mut clicked = false;
        let harness = HighLevelHarness::new(Harness::new_ui(|ui| {
            ui.label("Hello");
            if ui.button("Click me").clicked() {
                clicked = true;
            }
        }));

        let button = harness.by_label("Click me");

        // Node is no longer restrained by the lifetime of the harness!
        harness.run();

        // Event functions will automatically run the harness!
        button.click();
    }
}
