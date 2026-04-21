pub trait GetAtoms<'a> {
  pub fn text(self) -> Atom<'a>;
}
