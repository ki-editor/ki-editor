#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternator<T: Clone + std::fmt::Debug + PartialEq + Eq> {
    primary: T,
    secondary: Option<T>,
}

impl<T: Clone + std::fmt::Debug + PartialEq + Eq> Alternator<T> {
    pub fn new(primary: T) -> Self {
        Self {
            primary,
            secondary: None,
        }
    }

    pub fn cycle(&mut self) {
        if let Some(secondary) = self.secondary.take() {
            self.secondary = Some(std::mem::replace(&mut self.primary, secondary));
        }
    }

    pub fn copy_primary_to_secondary(&mut self) {
        self.secondary = Some(self.primary.clone());
    }

    pub fn primary(&self) -> &T {
        &self.primary
    }

    pub fn replace_primary(self, mode: T) -> Alternator<T> {
        Alternator {
            primary: mode,
            secondary: self.secondary,
        }
    }
}
