use crate::{Atom, AtomKind, Image, WidgetText};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

// Rarely there should be more than 2 atoms in one Widget.
// I guess it could happen in a menu button with Image and right text...
pub(crate) const ATOMS_SMALL_VEC_SIZE: usize = 2;

/// A list of [`Atom`]s.
#[derive(Clone, Debug, Default)]
pub struct Atoms<'a>(SmallVec<[Atom<'a>; ATOMS_SMALL_VEC_SIZE]>);

impl<'a> Atoms<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        atoms.into_atoms()
    }

    /// Insert a new [`Atom`] at the end of the list (right side).
    pub fn push_right(&mut self, atom: impl Into<Atom<'a>>) {
        self.0.push(atom.into());
    }

    /// Insert a new [`Atom`] at the beginning of the list (left side).
    pub fn push_left(&mut self, atom: impl Into<Atom<'a>>) {
        self.0.insert(0, atom.into());
    }

    /// Concatenate and return the text contents.
    // TODO(lucasmerlin): It might not always make sense to return the concatenated text, e.g.
    // in a submenu button there is a right text '⏵' which is now passed to the screen reader.
    pub fn text(&self) -> Option<Cow<'_, str>> {
        let mut string: Option<Cow<'_, str>> = None;
        for atom in &self.0 {
            if let AtomKind::Text(text) = &atom.kind {
                if let Some(string) = &mut string {
                    let string = string.to_mut();
                    string.push(' ');
                    string.push_str(text.text());
                } else {
                    string = Some(Cow::Borrowed(text.text()));
                }
            }
        }

        // If there is no text, try to find an image with alt text.
        if string.is_none() {
            string = self.iter().find_map(|a| match &a.kind {
                AtomKind::Image(image) => image.alt_text.as_deref().map(Cow::Borrowed),
                _ => None,
            });
        }

        string
    }

    pub fn iter_kinds(&'a self) -> impl Iterator<Item = &'a AtomKind<'a>> {
        self.0.iter().map(|atom| &atom.kind)
    }

    pub fn iter_kinds_mut(&'a mut self) -> impl Iterator<Item = &'a mut AtomKind<'a>> {
        self.0.iter_mut().map(|atom| &mut atom.kind)
    }

    pub fn iter_images(&'a self) -> impl Iterator<Item = &'a Image<'a>> {
        self.iter_kinds().filter_map(|kind| {
            if let AtomKind::Image(image) = kind {
                Some(image)
            } else {
                None
            }
        })
    }

    pub fn iter_images_mut(&'a mut self) -> impl Iterator<Item = &'a mut Image<'a>> {
        self.iter_kinds_mut().filter_map(|kind| {
            if let AtomKind::Image(image) = kind {
                Some(image)
            } else {
                None
            }
        })
    }

    pub fn iter_texts(&'a self) -> impl Iterator<Item = &'a WidgetText> {
        self.iter_kinds().filter_map(|kind| {
            if let AtomKind::Text(text) = kind {
                Some(text)
            } else {
                None
            }
        })
    }

    pub fn iter_texts_mut(&'a mut self) -> impl Iterator<Item = &'a mut WidgetText> {
        self.iter_kinds_mut().filter_map(|kind| {
            if let AtomKind::Text(text) = kind {
                Some(text)
            } else {
                None
            }
        })
    }

    pub fn map_atoms(&mut self, mut f: impl FnMut(Atom<'a>) -> Atom<'a>) {
        self.iter_mut()
            .for_each(|atom| *atom = f(std::mem::take(atom)));
    }

    pub fn map_kind<F>(&'a mut self, mut f: F)
    where
        F: FnMut(AtomKind<'a>) -> AtomKind<'a>,
    {
        for kind in self.iter_kinds_mut() {
            *kind = f(std::mem::take(kind));
        }
    }

    pub fn map_images<F>(&'a mut self, mut f: F)
    where
        F: FnMut(Image<'a>) -> Image<'a>,
    {
        self.map_kind(|kind| {
            if let AtomKind::Image(image) = kind {
                AtomKind::Image(f(image))
            } else {
                kind
            }
        });
    }

    pub fn map_texts<F>(&'a mut self, mut f: F)
    where
        F: FnMut(WidgetText) -> WidgetText,
    {
        self.map_kind(|kind| {
            if let AtomKind::Text(text) = kind {
                AtomKind::Text(f(text))
            } else {
                kind
            }
        });
    }
}

impl<'a> IntoIterator for Atoms<'a> {
    type Item = Atom<'a>;
    type IntoIter = smallvec::IntoIter<[Atom<'a>; ATOMS_SMALL_VEC_SIZE]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Helper trait to convert a tuple of atoms into [`Atoms`].
///
/// ```
/// use egui::{Atoms, Image, IntoAtoms, RichText};
/// let atoms: Atoms = (
///     "Some text",
///     RichText::new("Some RichText"),
///     Image::new("some_image_url"),
/// ).into_atoms();
/// ```
impl<'a, T> IntoAtoms<'a> for T
where
    T: Into<Atom<'a>>,
{
    fn collect(self, atoms: &mut Atoms<'a>) {
        atoms.push_right(self);
    }
}

/// Trait for turning a tuple of [`Atom`]s into [`Atoms`].
pub trait IntoAtoms<'a> {
    fn collect(self, atoms: &mut Atoms<'a>);

    fn into_atoms(self) -> Atoms<'a>
    where
        Self: Sized,
    {
        let mut atoms = Atoms::default();
        self.collect(&mut atoms);
        atoms
    }
}

impl<'a> IntoAtoms<'a> for Atoms<'a> {
    fn collect(self, atoms: &mut Self) {
        atoms.0.extend(self.0);
    }
}

macro_rules! all_the_atoms {
    ($($T:ident),*) => {
        impl<'a, $($T),*> IntoAtoms<'a> for ($($T),*)
        where
            $($T: IntoAtoms<'a>),*
        {
            fn collect(self, _atoms: &mut Atoms<'a>) {
                #[allow(clippy::allow_attributes)]
                #[allow(non_snake_case)]
                let ($($T),*) = self;
                $($T.collect(_atoms);)*
            }
        }
    };
}

all_the_atoms!();
all_the_atoms!(T0, T1);
all_the_atoms!(T0, T1, T2);
all_the_atoms!(T0, T1, T2, T3);
all_the_atoms!(T0, T1, T2, T3, T4);
all_the_atoms!(T0, T1, T2, T3, T4, T5);

impl<'a> Deref for Atoms<'a> {
    type Target = [Atom<'a>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Atoms<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
