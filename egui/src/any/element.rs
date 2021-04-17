use std::any::{Any, TypeId};
use std::fmt;

/// Like [`std::any::Any`], but also implements `Clone`.
pub(crate) struct AnyMapElement {
    value: Box<dyn Any + 'static>,
    clone_fn: fn(&Box<dyn Any + 'static>) -> Box<dyn Any + 'static>,
}

impl fmt::Debug for AnyMapElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyMapElement")
            .field("value_type_id", &self.type_id())
            .finish()
    }
}

impl Clone for AnyMapElement {
    fn clone(&self) -> Self {
        AnyMapElement {
            value: (self.clone_fn)(&self.value),
            clone_fn: self.clone_fn,
        }
    }
}

pub trait AnyMapTrait: 'static + Any + Clone {}

impl<T: 'static + Any + Clone> AnyMapTrait for T {}

impl AnyMapElement {
    pub(crate) fn new<T: AnyMapTrait>(t: T) -> Self {
        AnyMapElement {
            value: Box::new(t),
            clone_fn: |x| {
                let x = x.downcast_ref::<T>().unwrap(); // This unwrap will never panic, because we always construct this type using this `new` function and because we return &mut reference only with type `T`, so type cannot change.
                Box::new(x.clone())
            },
        }
    }

    pub(crate) fn type_id(&self) -> TypeId {
        (*self.value).type_id()
    }

    pub(crate) fn get_mut<T: AnyMapTrait>(&mut self) -> Option<&mut T> {
        self.value.downcast_mut()
    }

    pub(crate) fn get_mut_or_set_with<T: AnyMapTrait>(
        &mut self,
        set_with: impl FnOnce() -> T,
    ) -> &mut T {
        if !self.value.is::<T>() {
            *self = Self::new(set_with());
            // TODO: log this error, because it can occurs when user used same Id or same type for different widgets
        }

        self.value.downcast_mut().unwrap() // This unwrap will never panic because we already converted object to required type
    }
}
