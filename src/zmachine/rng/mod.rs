pub mod chacha_rng;

pub enum Mode {
    Random,
    Predictable,
}

pub trait ZRng {
    fn seed(&mut self, seed: u16);
    fn predictable(&mut self, seed: u16);
    fn random(&mut self, range: u16) -> u16;
}
