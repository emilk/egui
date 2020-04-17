use std::{collections::hash_map::DefaultHasher, hash::Hash};

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Id(u64);

impl Id {
    pub fn whole_screen() -> Self {
        Self(0)
    }

    pub fn popup() -> Self {
        Self(1)
    }

    pub fn new<H: Hash>(source: &H) -> Id {
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        Id(hasher.finish())
    }

    pub fn with<H: Hash>(self, child: &H) -> Id {
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(self.0);
        child.hash(&mut hasher);
        Id(hasher.finish())
    }
}
