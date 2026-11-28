//! Generateur pseudo-aleatoire rapide et deterministe (PCG32).
use glam::Vec2;

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng { state: seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) };
        r.next_u32();
        r
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Flottant dans [0, 1).
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    #[inline]
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }

    /// Entier dans [0, n).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u32() as u64 * n as u64) >> 32) as u32
    }

    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }

    /// Vecteur unitaire de direction aleatoire.
    pub fn dir(&mut self) -> Vec2 {
        let a = self.f() * std::f32::consts::TAU;
        Vec2::new(libm::cosf(a), libm::sinf(a))
    }
}
