pub trait GetAtoms<'a> {
  pub fn text(self) -> Atoms<'a>;
}

impl<'a> GetAtoms<'a> for Button<'a> {
  pub fn text(self) -> Atoms<'a> {
    self.layout.atoms
  }
}

impl<'a> GetAtoms<'a> for Checkbox<'a> {
  pub fn text(self) -> Atoms<'a> {
    self.atoms
  }
}

impl<'a> GetAtoms<'a> for DragValue<'a> {
  pub fn text(self) -> Atoms<'a> {
    self.atoms
  }
}


impl<'a> GetAtoms<'a> for RadioButton<'a> {
  pub fn text(self) -> Atoms<'a> {
    self.atoms
  }
}
