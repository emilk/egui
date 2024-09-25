use crate::event::AKEvent;
use crate::query::Queryable;
use crate::Node;
use std::ops::Deref;
use std::sync::Mutex;

pub struct Tree {
    tree: accesskit_consumer::Tree,
    queued_events: Mutex<Vec<AKEvent>>,
}

impl Tree {
    pub fn new(update: accesskit::TreeUpdate) -> Tree {
        Self {
            tree: accesskit_consumer::Tree::new(update, true),
            queued_events: Mutex::new(Vec::new()),
        }
    }

    pub fn update(&mut self, update: accesskit::TreeUpdate) {
        self.tree.update(update);
    }

    pub fn root(&self) -> Node<'_> {
        self.node()
    }

    pub fn take_events(&self) -> Vec<AKEvent> {
        self.queued_events.lock().unwrap().drain(..).collect()
    }
}

impl<'tree, 'node> Queryable<'tree, 'node> for Tree
where
    'node: 'tree,
{
    /// Return the root node
    fn node(&'node self) -> Node<'tree> where {
        Node::new(self.tree.state().root(), &self.queued_events)
    }
}
