use core::fmt;

pub mod chacha_rng;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Random,
    Predictable,
}

pub trait ZRng {
    fn type_name(&self) -> &str;
    fn seed(&mut self, seed: u16);
    fn predictable(&mut self, seed: u16);
    fn random(&mut self, range: u16) -> u16;
}

impl fmt::Debug for dyn ZRng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}
