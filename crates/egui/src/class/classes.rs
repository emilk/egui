use crate::class::{ClassName, HasClasses};
use smallvec::SmallVec;
use std::fmt;

/// [`Classes`] is a collection of [`ClassName`]s that can be added to widgets or containers.
///
/// This can be used by styling engine to compute a different style
/// based on the set of classes present on the widget/Ui.
/// You could also use it to e.g. change widget behavior based on the context of some container.
///
/// Class order is preserved and may be used by a style provider for precedence (last class should win).
///
/// Use [`HasClasses`] to add/modify classes.
#[derive(Debug, Default, Clone, Hash)]
pub struct Classes {
    classes: SmallVec<[ClassName; 5]>,
}

impl Classes {
    /// Add a class to the list if the condition is true.
    ///
    /// A class is never added twice. This never removes a class: use [`Self::set`] for that.
    #[inline]
    pub(crate) fn add_if(&mut self, class: impl Into<ClassName>, condition: bool) {
        if condition {
            self.set(class, true);
        }
    }

    /// Add the class if `present`, remove it otherwise.
    #[inline]
    pub(crate) fn set(&mut self, class: impl Into<ClassName>, present: bool) {
        let class = class.into();
        // Always retain and push again, since order of classes can matter.
        self.classes.retain(|existing| existing != &class);
        if present {
            self.classes.push(class);
        }
    }

    /// Extend the classes and deduplicate them.
    ///
    /// A class that is already present is moved to the end, since order can matter.
    #[inline]
    pub(crate) fn extend(&mut self, classes: impl IntoIterator<Item = impl Into<ClassName>>) {
        for class in classes {
            self.set(class, true);
        }
    }

    /// Iterate over the classes, in order.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, ClassName> {
        self.classes.iter()
    }

    /// Return the classes as a slice
    #[inline]
    pub fn as_slice(&self) -> &[ClassName] {
        self.classes.as_slice()
    }
}

impl IntoIterator for Classes {
    type Item = ClassName;
    type IntoIter = smallvec::IntoIter<[ClassName; 5]>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.classes.into_iter()
    }
}

impl<'a> IntoIterator for &'a Classes {
    type Item = &'a ClassName;
    type IntoIter = core::slice::Iter<'a, ClassName>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.classes.iter()
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
        for (i, class) in self.classes.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            f.write_str(class.as_str())?;
        }
        Ok(())
    }
}
