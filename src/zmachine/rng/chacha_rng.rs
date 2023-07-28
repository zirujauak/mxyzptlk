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

impl ZRng for ChaChaRng {
    fn type_name(&self) -> &str {
        "ChaChaRng"
    }
    
    fn seed(&mut self, seed: u16) {
        if seed == 0 {
            self.rng = ChaCha8Rng::from_entropy();
        } else {
            self.rng = ChaCha8Rng::seed_from_u64(seed as u64)
        }
        self.mode = Mode::Random;
        self.predictable_range = 1;
        self.predictable_next = 1;
    }

    fn predictable(&mut self, seed: u16) {
        self.predictable_range = seed;
        self.predictable_next = 1;
        self.mode = Mode::Predictable;
    }

    fn random(&mut self, range: u16) -> u16 {
        match self.mode {
            Mode::Predictable => {
                let v = if range < self.predictable_next {
                    self.predictable_next % range
                } else {
                    self.predictable_next
                };
                if self.predictable_next == self.predictable_range {
                    self.predictable_next = 1;
                } else {
                    self.predictable_next += 1
                }
                v
            }
            Mode::Random => self.rng.gen_range(1..=range),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor() {
        let c = ChaChaRng::new();
        assert_eq!(c.mode, Mode::Random);
        assert_eq!(c.predictable_range, 1);
        assert_eq!(c.predictable_next, 1);
    }

    #[test]
    fn test_mode() {
        let mut c = ChaChaRng::new();
        c.predictable(10);
        assert_eq!(c.mode, Mode::Predictable);
        assert_eq!(c.predictable_range, 10);
        assert_eq!(c.predictable_next, 1);
        c.seed(0);
        assert_eq!(c.mode, Mode::Random);
        assert_eq!(c.predictable_range, 1);
        assert_eq!(c.predictable_next, 1);
    }

    #[test]
    fn test_random_entropy() {
        let mut c = ChaChaRng::new();
        for _ in 1..10 {
            assert!((1..=100).contains(&c.random(100)));
        }
    }

    #[test]
    fn test_random_seeded() {
        let mut c = ChaChaRng::new();
        c.seed(1024);
        assert_eq!(c.random(100), 99);
        assert_eq!(c.random(100), 93);
        assert_eq!(c.random(100), 69);
        assert_eq!(c.random(100), 89);
        assert_eq!(c.random(100), 82);
        assert_eq!(c.random(100), 26);
        assert_eq!(c.random(100), 22);
        assert_eq!(c.random(100), 40);
        assert_eq!(c.random(100), 23);
        assert_eq!(c.random(100), 76);
    }

    #[test]
    fn test_random_predictable() {
        let mut c = ChaChaRng::new();
        c.predictable(5);
        for i in 1..4 {
            assert_eq!(c.random(3), i)
        }
        for i in 1..3 {
            assert_eq!(c.random(3), i)
        }
        assert_eq!(c.predictable_next, 1);
    }
}
