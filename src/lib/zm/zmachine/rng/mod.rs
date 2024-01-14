//! [Random number generator](https://inform-fiction.org/zmachine/standards/z1point1/sect02.html#four)
use core::fmt;

pub mod chacha_rng;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// RNG mode
pub enum Mode {
    /// Random mode, returns (pseudo-)random numbers
    Random,
    /// Predictable mode, returns a predictable sequence of numbers
    Predictable,
}

pub trait ZRng {
    /// RNG type name
    ///
    /// # Returns
    /// RNG type name string
    fn type_name(&self) -> &str;

    /// Seed the RNG and updates the mode to [Mode::Random]
    ///
    /// # Arguments
    /// * `seed` - seed value, or 0 to seed from entropy
    fn seed(&mut self, seed: u16);

    /// Sets the RNG mode to [Mode::Predictable] and sets the predictable range
    ///
    /// # Arguments
    /// * `seed` - The upper limit of the predictable range
    fn predictable(&mut self, seed: u16);

    /// Gets the next random number, per the current [Mode]
    ///
    /// # Arguments
    /// * `range` - the upper limit of the result
    ///
    /// # Returns
    /// Random value in the range 1..=`range`
    fn random(&mut self, range: u16) -> u16;
}

impl fmt::Debug for dyn ZRng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}
