use crate::{Atomic, AtomicKind};
use smallvec::SmallVec;

// Rarely there should be more than 2 atomics in one Widget.
// I guess it could happen in a menu button with Image and right text...
pub(crate) const ATOMICS_SMALL_VEC_SIZE: usize = 2;

/// A list of [`Atomic`]s.
#[derive(Clone, Debug, Default)]
pub struct Atomics<'a>(SmallVec<[Atomic<'a>; ATOMICS_SMALL_VEC_SIZE]>);

impl<'a> Atomics<'a> {
    pub fn new(content: impl IntoAtomics<'a>) -> Self {
        content.into_atomics()
    }

    /// Insert a new [`Atomic`] at the end of the list (right side).
    pub fn push(&mut self, atomic: impl Into<Atomic<'a>>) {
        self.0.push(atomic.into());
    }

    /// Insert a new [`Atomic`] at the beginning of the list (left side).
    pub fn push_front(&mut self, atomic: impl Into<Atomic<'a>>) {
        self.0.insert(0, atomic.into());
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Atomic<'a>> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Atomic<'a>> {
        self.0.iter_mut()
    }

    /// Concatenate and return the text contents.
    // TODO(lucasmerlin): It might not always make sense to return the concatenated text, e.g.
    // in a submenu button there is a right text '⏵' which is now passed to the screen reader.
    pub fn text(&self) -> Option<String> {
        let mut string: Option<String> = None;
        for atomic in &self.0 {
            if let AtomicKind::Text(text) = &atomic.kind {
                if let Some(string) = &mut string {
                    string.push(' ');
                    string.push_str(text.text());
                } else {
                    string = Some(text.text().to_owned());
                }
            }
        }
        string
    }
}

impl<'a> IntoIterator for Atomics<'a> {
    type Item = Atomic<'a>;
    type IntoIter = smallvec::IntoIter<[Atomic<'a>; ATOMICS_SMALL_VEC_SIZE]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Helper trait to convert a tuple of atomics into [`Atomics`].
///
/// ```
/// use egui::{Atomics, Image, IntoAtomics, RichText};
/// let atomics: Atomics = (
///     "Some text",
///     RichText::new("Some RichText"),
///     Image::new("some_image_url"),
/// ).into_atomics();
/// ```
impl<'a, T> IntoAtomics<'a> for T
where
    T: Into<Atomic<'a>>,
{
    fn collect(self, atomics: &mut Atomics<'a>) {
        atomics.push(self);
    }
}

pub trait IntoAtomics<'a> {
    fn collect(self, atomics: &mut Atomics<'a>);

    fn into_atomics(self) -> Atomics<'a>
    where
        Self: Sized,
    {
        let mut atomics = Atomics::default();
        self.collect(&mut atomics);
        atomics
    }
}

impl<'a> IntoAtomics<'a> for Atomics<'a> {
    fn collect(self, atomics: &mut Self) {
        atomics.0.extend(self.0);
    }
}

macro_rules! all_the_atomics {
    ($($T:ident),*) => {
        impl<'a, $($T),*> IntoAtomics<'a> for ($($T),*)
        where
            $($T: IntoAtomics<'a>),*
        {
            fn collect(self, _atomics: &mut Atomics<'a>) {
                #[allow(non_snake_case)]
                let ($($T),*) = self;
                $($T.collect(_atomics);)*
            }
        }
    };
}

all_the_atomics!();
all_the_atomics!(T0, T1);
all_the_atomics!(T0, T1, T2);
all_the_atomics!(T0, T1, T2, T3);
all_the_atomics!(T0, T1, T2, T3, T4);
all_the_atomics!(T0, T1, T2, T3, T4, T5);
