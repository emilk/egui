use std::sync::Arc;

/// An immutable string, backed by either `&'static str` or `Arc<String>`.
///
/// Wherever you see `impl Into<Estring>` pass either a `String` or
/// a `&'static str` (a `"string literal"`).
///
/// Estring provides fast `Clone`.
#[derive(Clone)]
pub enum Estring {
    Static(&'static str),
    Owned(Arc<str>),
}

impl Estring {
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Owned(s) => s,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }
}

impl Default for Estring {
    fn default() -> Self {
        Self::Static("")
    }
}

impl std::convert::AsRef<str> for Estring {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::Borrow<str> for Estring {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::hash::Hash for Estring {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl PartialEq for Estring {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl std::cmp::Eq for Estring {}

impl std::cmp::PartialOrd for Estring {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Estring {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::fmt::Display for Estring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Debug for Estring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

// ----------------------------------------------------------------------------

impl std::convert::From<&'static str> for Estring {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl std::convert::From<String> for Estring {
    fn from(s: String) -> Self {
        Self::Owned(s.into())
    }
}

impl std::convert::From<&String> for Estring {
    fn from(s: &String) -> Self {
        Self::Owned(s.clone().into())
    }
}

impl std::convert::From<&Estring> for Estring {
    fn from(s: &Estring) -> Self {
        s.clone()
    }
}

// ----------------------------------------------------------------------------

impl std::ops::Index<std::ops::Range<usize>> for Estring {
    type Output = str;

    #[inline]
    fn index(&self, index: std::ops::Range<usize>) -> &str {
        self.as_str().index(index)
    }
}

impl std::ops::Index<std::ops::RangeTo<usize>> for Estring {
    type Output = str;

    #[inline]
    fn index(&self, index: std::ops::RangeTo<usize>) -> &str {
        self.as_str().index(index)
    }
}

impl std::ops::Index<std::ops::RangeFrom<usize>> for Estring {
    type Output = str;

    #[inline]
    fn index(&self, index: std::ops::RangeFrom<usize>) -> &str {
        self.as_str().index(index)
    }
}

impl std::ops::Index<std::ops::RangeFull> for Estring {
    type Output = str;

    #[inline]
    fn index(&self, index: std::ops::RangeFull) -> &str {
        self.as_str().index(index)
    }
}

impl std::ops::Index<std::ops::RangeInclusive<usize>> for Estring {
    type Output = str;

    #[inline]
    fn index(&self, index: std::ops::RangeInclusive<usize>) -> &str {
        self.as_str().index(index)
    }
}

impl std::ops::Index<std::ops::RangeToInclusive<usize>> for Estring {
    type Output = str;

    #[inline]
    fn index(&self, index: std::ops::RangeToInclusive<usize>) -> &str {
        self.as_str().index(index)
    }
}
