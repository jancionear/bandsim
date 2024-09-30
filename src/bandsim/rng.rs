use rand::SeedableRng;

pub type DefaultRng = rand::rngs::StdRng;

pub fn rng_from_seed(seed: u64) -> DefaultRng {
    let mut seed_bytes = Vec::new();
    for _ in 0..4 {
        seed_bytes.extend_from_slice(&seed.to_be_bytes());
    }
    DefaultRng::from_seed(seed_bytes.try_into().unwrap())
}
