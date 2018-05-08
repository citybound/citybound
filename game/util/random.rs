pub use rand::Rng;
use rand::SeedableRng;
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

// A hashing function with hopefully low correlation between seeds
// but not necessarily good randomness of sequential probes on the same seed
pub struct FnvRng {
    seed: u64,
}

impl Rng for FnvRng {
    fn next_u64(&mut self) -> u64 {
        let current = self.seed;
        let mut hasher = FnvHasher::default();
        self.seed.hash(&mut hasher);
        self.seed = hasher.finish();
        current
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
}

impl SeedableRng<u64> for FnvRng {
    fn from_seed(seed: u64) -> FnvRng {
        FnvRng { seed }
    }

    fn reseed(&mut self, seed: u64) {
        self.seed = seed;
    }
}

pub fn seed<S: Hash>(seed: S) -> FnvRng {
    let mut hasher = FnvHasher::default();
    seed.hash(&mut hasher);
    FnvRng::from_seed(hasher.finish())
}
