use rand::RngExt;

pub trait Random: Send + Sync {
    fn unit(&self) -> f64;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadRandom;

impl Random for ThreadRandom {
    fn unit(&self) -> f64 {
        rand::rng().random_range(0.0..1.0)
    }
}

#[cfg(test)]
pub(crate) struct FixedRandom(pub f64);

#[cfg(test)]
impl Random for FixedRandom {
    fn unit(&self) -> f64 {
        self.0
    }
}
