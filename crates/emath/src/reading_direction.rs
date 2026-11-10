//! Reading direction for text and UI chrome ([`ReadingDirection::Ltr`], [`ReadingDirection::Rtl`]).

/// The reading direction of text and UI chrome:
/// left-to-right ([`Self::Ltr`]) or right-to-left ([`Self::Rtl`]).
///
/// This is orthogonal to the crate's coordinate convention
/// ("X+ is right and Y+ is down", see the crate docs):
/// the physical meaning of the X axis never changes, but the *reading order*
/// in which content is encountered depends on this direction.
/// Physical math stays unmirrored; logical reading-order helpers
/// (e.g. on [`Align`](crate::Align) and [`Rect`](crate::Rect)) take a `ReadingDirection`.
///
/// Which direction applies in a given context is a question of locale and user
/// preference; no locale logic lives in `emath`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ReadingDirection {
    /// Left-to-right reading order (e.g. Latin, Greek and Cyrillic scripts).
    #[default]
    Ltr,

    /// Right-to-left reading order (e.g. Arabic and Hebrew scripts).
    Rtl,
}

impl ReadingDirection {
    /// Is this right-to-left?
    ///
    /// The one predicate most consumers need: mirroring a left-to-right
    /// layout is gated on this.
    #[inline(always)]
    #[must_use]
    pub fn is_rtl(self) -> bool {
        self == Self::Rtl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ltr() {
        assert_eq!(ReadingDirection::default(), ReadingDirection::Ltr);
    }

    #[test]
    fn is_rtl() {
        assert!(!ReadingDirection::Ltr.is_rtl());
        assert!(ReadingDirection::Rtl.is_rtl());
    }
}
