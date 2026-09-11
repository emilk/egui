use core::borrow::Borrow;
use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

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
    pub fn new(class: impl Into<Self>) -> Self {
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

impl From<&Self> for ClassName {
    #[inline]
    fn from(class: &Self) -> Self {
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

impl core::ops::Deref for ClassName {
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

impl core::hash::Hash for ClassName {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
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
