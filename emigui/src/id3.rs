use ahash::AHashMap;
// use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Id(Arc<String>);

impl Id {
    pub fn as_string(&self) -> &String {
        &self.0
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ----------------------------------------------------------------------

type Generation = u64;

// One per context
#[derive(Default)]
pub struct IdInterner {
    /// used to garbage-collect id:s which hasn't been used in a while
    generation: Generation,

    /// Maps
    children: AHashMap<Id, AHashMap<String, (Generation, Id)>>,
}

impl IdInterner {
    pub fn new_root(&self, root_id: &str) -> Id {
        Id(Arc::new(root_id.to_string()))
    }

    /// Append `comp` to `parent_id`.
    /// This is pretty cheap if the same lookup was done last frame,
    /// else it will cost a memory allocation
    pub fn child(&mut self, parent_id: &Id, comp: &str) -> Id {
        if let Some(map) = self.children.get_mut(parent_id) {
            if let Some((gen, child_id)) = map.get_mut(comp) {
                *gen = self.generation;
                child_id.clone()
            } else {
                let child_id = Id(Arc::new(format!("{}/{}", parent_id, comp)));
                map.insert(comp.to_owned(), (self.generation, child_id.clone()));
                child_id
            }
        } else {
            let child_id = Id(Arc::new(format!("{}/{}", parent_id, comp)));
            let mut map = AHashMap::new();
            map.insert(comp.to_owned(), (self.generation, child_id.clone()));
            self.children.insert(parent_id.clone(), map);
            child_id
        }
    }

    /// Called by the context once per frame
    pub fn gc(&mut self) {
        let current_gen = self.generation;
        for value in self.children.values_mut() {
            value.retain(|_comp, (gen, _id)| *gen == current_gen);
        }
        self.children.retain(|_k, v| !v.is_empty());
        self.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id() {
        let interner = parking_lot::Mutex::new(IdInterner::default());
        let root: Id = interner.lock().new_root("root");
        let child_a: Id = interner.lock().child(&root, "child");
        let child_b: Id = interner.lock().child(&root, "child");

        assert!(root != child_a);
        assert_eq!(child_a, child_b);
    }
}
