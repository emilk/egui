use smallvec::SmallVec;
use std::borrow::{Borrow, Cow};
use std::fmt;
use std::sync::Arc;

use crate::TextBuffer as _;

/// A class, used to customize widget styling and behavior.
#[derive(Clone)]
pub struct ClassName(ClassNameInner);

#[derive(Clone)]
enum ClassNameInner {
    Static(&'static str),
    Owned(Arc<str>),
}

impl ClassName {
    /// A class from a string known at compile time. This never allocates.
    #[inline]
    pub const fn from_static(class: &'static str) -> Self {
        Self(ClassNameInner::Static(class))
    }

    /// A class from anything that converts into one.
    #[inline]
    pub fn new(class: impl Into<ClassName>) -> Self {
        class.into()
    }

    /// The class as a string.
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            ClassNameInner::Static(class) => class,
            ClassNameInner::Owned(class) => class,
        }
    }
}

impl From<&'static str> for ClassName {
    #[inline]
    fn from(class: &'static str) -> Self {
        Self::from_static(class)
    }
}

impl From<String> for ClassName {
    #[inline]
    fn from(class: String) -> Self {
        Self(ClassNameInner::Owned(class.into()))
    }
}

impl From<&String> for ClassName {
    #[inline]
    fn from(class: &String) -> Self {
        Self(ClassNameInner::Owned(class.as_str().into()))
    }
}

impl From<Arc<str>> for ClassName {
    #[inline]
    fn from(class: Arc<str>) -> Self {
        Self(ClassNameInner::Owned(class))
    }
}

impl From<Cow<'static, str>> for ClassName {
    #[inline]
    fn from(class: Cow<'static, str>) -> Self {
        match class {
            Cow::Borrowed(class) => Self::from_static(class),
            Cow::Owned(class) => class.into(),
        }
    }
}

impl From<&ClassName> for ClassName {
    #[inline]
    fn from(class: &ClassName) -> Self {
        class.clone()
    }
}

impl Borrow<str> for ClassName {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for ClassName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::ops::Deref for ClassName {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for ClassName {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for ClassName {}

impl PartialEq<str> for ClassName {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for ClassName {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl std::hash::Hash for ClassName {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Debug for ClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Display for ClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Classes are string identifier that can be set on widget/Ui.
///
/// This can be used by styling engine to compute a different style
/// based on the set of classes present on the widget/Ui.
/// Class order is preserved and may be used by a style provider for precedence.
#[derive(Debug, Default, Clone, Hash)]
pub struct Classes {
    classes: SmallVec<[ClassName; 5]>,
}

impl Classes {
    /// Add a class to the list if the condition is true.
    ///
    /// A class is never added twice. This never removes a class: use [`Self::set`] for that.
    #[inline]
    fn add_if(&mut self, class: impl Into<ClassName>, condition: bool) {
        if condition {
            let class = class.into();
            // Always retain and push again, since order of classes can matter.
            self.classes.retain(|existing| existing != &class);
            self.classes.push(class);
        }
    }

    /// Add the class if `present`, remove it otherwise.
    #[inline]
    fn set(&mut self, class: impl Into<ClassName>, present: bool) {
        let class = class.into();
        // Always retain and push again, since order of classes can matter.
        self.classes.retain(|existing| existing != &class);
        if present {
            self.classes.push(class);
        }
    }
}

impl HasClasses for Classes {
    fn classes(&self) -> &Classes {
        self
    }

    fn classes_mut(&mut self) -> &mut Classes {
        self
    }
}

impl core::fmt::Display for Classes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.classes.iter().for_each(|class| {
            let _ = f.write_str(class.as_str());
        });
        f.write_str("")
    }
}

/// Any widgets supporting [`Classes`] must implement this trait.
pub trait HasClasses {
    fn classes(&self) -> &Classes;

    fn classes_mut(&mut self) -> &mut Classes;

    /// Add the given class by consuming `self`.
    #[inline]
    fn with_class(mut self, class: impl Into<ClassName>) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Add all the given classes by consuming `self`.
    #[inline]
    fn with_classes(mut self, classes: Classes) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().classes.extend(classes.classes);
        self
    }

    /// Add the given class by consuming `self` if the condition is true.
    #[inline]
    fn with_class_if(mut self, class: impl Into<ClassName>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
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

    /// Add all the given classes.
    #[inline]
    fn add_classes(&mut self, classes: Classes) -> &mut Self {
        self.classes_mut().classes.extend(classes.classes);
        self
    }

    /// Add the given class in-place if the condition is true.
    #[inline]
    fn add_class_if(&mut self, class: impl Into<ClassName>, condition: bool) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
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

    /// True if the class is present.
    #[inline]
    fn has_class(&self, class: impl Into<ClassName>) -> bool {
        self.classes().classes.contains(&class.into())
    }

    /// The list of class.
    fn as_slice(&self) -> &[ClassName] {
        &self.classes().classes
    }
}

#[cfg(test)]
mod tests {
    use super::{Classes, HasClasses as _};

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
