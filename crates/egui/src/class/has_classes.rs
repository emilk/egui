use crate::class::ClassName;

/// Any widgets supporting [`crate::class::Classes`] must implement this trait.
pub trait HasClasses {
    fn classes(&self) -> &crate::class::Classes;

    fn classes_mut(&mut self) -> &mut crate::class::Classes;

    /// True if the class is present.
    #[inline]
    fn has_class(&self, class: &str) -> bool {
        self.classes()
            .iter()
            .any(|existing| existing.as_str() == class)
    }

    /// Add the given class in-place.
    #[inline]
    fn add_class(&mut self, class: impl Into<ClassName>) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Add the given class in-place if `present`, remove it otherwise.
    ///
    /// Use this for a setter that takes a `bool`, so that the last call wins.
    #[inline]
    fn set_class(&mut self, class: impl Into<ClassName>, present: bool) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().set(class.into(), present);
        self
    }

    /// Remove the given class in-place.
    #[inline]
    fn remove_class(&mut self, class: impl Into<ClassName>) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().set(class.into(), false);
        self
    }

    /// Add the given class by consuming `self`.
    #[inline]
    fn with_class(mut self, class: impl Into<ClassName>) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Append all the given classes at the end, deduplicating them.
    #[inline]
    fn add_classes(&mut self, classes: crate::class::Classes) -> &mut Self {
        self.classes_mut().extend(classes);
        self
    }

    /// Append all the given classes at the end, deduplicating them. Consuming `self`.
    #[inline]
    fn with_classes(mut self, classes: crate::class::Classes) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().extend(classes);
        self
    }

    /// Add a class to the list if the condition is true.
    ///
    /// A class is never added twice. This never removes a class: use [`Self::set`] for that.
    #[inline]
    fn add_class_if(&mut self, class: impl Into<ClassName>, condition: bool) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// Add the given class by consuming `self` if the condition is true.
    ///
    /// A class is never added twice. This never removes a class: use [`Self::set_class`] for that.
    #[inline]
    fn with_class_if(mut self, class: impl Into<ClassName>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// Iterate over the classes, in the order they were added.
    #[inline]
    fn iter_classes(&self) -> core::slice::Iter<'_, ClassName> {
        self.classes().iter()
    }

    /// Return the classes as a slice.
    #[inline]
    fn classes_as_slice(&self) -> &[ClassName] {
        self.classes().as_slice()
    }
}

#[cfg(test)]
mod tests {
    use crate::class::{Classes, HasClasses as _};

    #[test]
    fn setting_a_class_moves_it_to_end() {
        let mut classes = Classes::default();
        classes.add_class("first");
        classes.add_class("updated");
        classes.add_class("second");

        classes.set_class("updated", true);

        assert_eq!(classes.as_slice(), ["first", "second", "updated"]);
    }

    #[test]
    fn adding_a_class_twice_moves_it_to_end() {
        let mut classes = Classes::default();
        classes.add_class("first");
        classes.add_class("updated");
        classes.add_class("second");

        classes.add_class("updated");

        assert_eq!(classes.as_slice(), ["first", "second", "updated"]);
    }
}
