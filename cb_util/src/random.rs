pub use rand::{Rng, RngCore, thread_rng};
pub use uuid::Uuid;
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};
use uuid::{Builder, Version};

// A hashing function with hopefully low correlation between seeds
// but not necessarily good randomness of sequential probes on the same seed
pub struct FnvRng {
    seed: u64,
}

impl RngCore for FnvRng {
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

    fn fill_bytes(&mut self, _bytes: &mut [u8]) {
        unimplemented!()
    }

    fn try_fill_bytes(&mut self, _bytes: &mut [u8]) -> Result<(), ::rand::Error> {
        unimplemented!()
    }
}

pub fn seed<S: Hash>(seed: S) -> FnvRng {
    let mut hasher = FnvHasher::default();
    seed.hash(&mut hasher);
    FnvRng {
        seed: hasher.finish(),
    }
}

pub fn uuid() -> Uuid {
    Builder::from_bytes(thread_rng().gen())
    .set_version(Version::Random)
    .build()
}
