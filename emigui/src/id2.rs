use std::{borrow::Cow, sync::Arc};

use ahash::AHashMap;

// TODO: is stored Id string with own hash for faster lookups
// i.e. pub struct Id(u64, Arc<String>);
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

// #[derive(Eq, Hash, PartialEq)]
// pub enum Component<'a> {
//     /// E.g. name of a window
//     String(Cow<'a, str>),

//     /// For loop indices, hashes etc
//     Int(u64),
// }

// impl<'a> Component<'a> {
//     fn to_owned(self) -> Component<'static> {
//         match self {
//             Component::String(s) => Component::String(s.into()),
//             Component::Int(int) => Component::Int(int),
//         }
//     }
// }

// impl<'a> std::fmt::Debug for Component<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Component::String(v) => v.fmt(f),
//             Component::Int(v) => v.fmt(f),
//         }
//     }
// }

// impl<'a> std::fmt::Display for Component<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Component::String(v) => v.fmt(f),
//             Component::Int(v) => v.fmt(f),
//         }
//     }
// }

// ----------------------------------------------------------------------

type Generation = u64;

// One per context
#[derive(Default)]
pub struct IdInterner {
    /// used to garbage-collect id:s which hasn't been used in a while
    generation: Generation,

    /// Maps
    children: AHashMap<(Id, Cow<'static, str>), (Generation, Id)>,
}

impl IdInterner {
    pub fn new_root(&self, root_id: &str) -> Id {
        Id(Arc::new(root_id.to_string()))
    }

    /// Append `comp` to `parent_id`.
    /// This is pretty cheap if the same lookup was done last frame,
    /// else it will cost a memory allocation
    pub fn child<'a>(&mut self, parent_id: &'a Id, comp: &'a str) -> Id {
        if let Some(existing) = self.children.get_mut(&(parent_id.clone(), comp.into())) {
            existing.0 = self.generation;
            existing.1.clone()
        } else {
            let child_id = Id(Arc::new(format!("{}/{}", parent_id, comp)));
            self.children.insert(
                (parent_id.clone(), comp.into()),
                (self.generation, child_id.clone()),
            );
            child_id
        }
    }

    /// Called by the context once per frame
    pub fn gc(&mut self) {
        let current_gen = self.generation;
        self.children.retain(|_k, v| v.0 == current_gen);
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
        let child_a: Id = interner.lock().child(&root, Component::Int(42));
        let child_b: Id = interner.lock().child(&root, Component::Int(42));

        assert!(root != child_a);
        assert_eq!(child_a, child_b);
    }
}
