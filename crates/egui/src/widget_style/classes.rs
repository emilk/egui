use std::{borrow::Cow, fmt};

use smallvec::SmallVec;

use crate::TextBuffer as _;

/// The root class is a special class present on every top-level [`crate::Ui`].
pub const ROOT_CLASS: &str = "root";

/// The selected class is a special class present on selected [`crate::Button`].
pub const SELECTED_CLASS: &str = "selected";

/// A class is a static string identifier.
pub type ClassName = Cow<'static, str>;

/// Classes are string identifier that can be set on widget/Ui.
///
/// This can be used by styling engine to compute a different style
/// based on the set of classes present on the widget/Ui.
#[derive(Debug, Default, Clone, Hash)]
pub struct Classes {
    classes: SmallVec<[ClassName; 5]>,
}

impl Classes {
    /// Add a class to the list if the condition is true
    #[inline]
    fn add_if(&mut self, class: impl Into<ClassName>, condition: bool) {
        if condition {
            self.classes.push(class.into());
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

impl std::fmt::Display for Classes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.classes.iter().for_each(|class| {
            let _ = f.write_str(class.as_str());
        });
        f.write_str("")
    }
}

/// Any widgets supporting [`Classes`] must implement this trait
pub trait HasClasses {
    fn classes(&self) -> &Classes;

    fn classes_mut(&mut self) -> &mut Classes;

    /// Add the given class by consuming `self`
    #[inline]
    fn with_class(mut self, class: impl Into<ClassName>) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Add the given class by consuming `self` if the condition is true
    #[inline]
    fn with_class_if(mut self, class: impl Into<ClassName>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// Add the given class in-place
    #[inline]
    fn add_class(&mut self, class: impl Into<ClassName>) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Add the given class in-place if the condition is true
    #[inline]
    fn add_class_if(&mut self, class: impl Into<ClassName>, condition: bool) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// True if the class is present
    fn has(&self, class: impl Into<ClassName>) -> bool {
        self.classes().classes.contains(&class.into())
    }

    /// The list of class
    fn as_slice(&self) -> &[ClassName] {
        &self.classes().classes
    }
}
