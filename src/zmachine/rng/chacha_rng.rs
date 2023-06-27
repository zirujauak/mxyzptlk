use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::zmachine::rng::*;

pub struct ChaChaRng {
    mode: Mode,
    predictable_range: u16,
    predictable_next: u16,
    rng: ChaCha8Rng,
}

impl ChaChaRng {
    pub fn new() -> ChaChaRng {
        ChaChaRng {
            mode: Mode::Random,
            predictable_range: 1,
            predictable_next: 1,
            rng: ChaCha8Rng::from_entropy(),
        }
    }
}

impl RNG for ChaChaRng {
    fn seed(&mut self, seed: u16) {
        if seed == 0 {
            self.rng = ChaCha8Rng::from_entropy();
        } else {
            self.rng = ChaCha8Rng::seed_from_u64(seed as u64)
        }
        self.mode = Mode::Random;
    }

    fn predictable(&mut self, seed: u16) {
        self.predictable_range = seed;
        self.predictable_next = 1;
        self.mode = Mode::Predictable;
    }

    fn random(&mut self, range: u16) -> u16 {
        match self.mode {
            Mode::Predictable => {
                let v = self.predictable_next % range;
                if self.predictable_next == self.predictable_range {
                    self.predictable_next = 1;
                } else {
                    self.predictable_next = self.predictable_next + 1
                }
                v
            }
            Mode::Random => self.rng.gen_range(1..=range),
        }
    }
}
