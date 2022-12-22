use rand::Rng;
use rand::seq::SliceRandom;
use rand::distributions::{Distribution, Standard};

pub struct Pitch {
    pub pitch_class: PitchClass,
    pub octave: u8,
}

impl Pitch {
    pub fn midi(&self) -> u8 {
        // C4 -> 60
        (12 * (self.octave + 1)) + self.pitch_class as u8
    }
}

#[derive(Clone, Copy)]
pub enum PitchClass {
    C = 0,
    D = 2,
    E = 4,
    F = 5,
    G = 7,
    A = 9,
    B = 11,
}

impl Distribution<PitchClass> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PitchClass {
        use PitchClass::*;
        *[C, D, E, F, G, A, B].choose(rng).unwrap()
    }
}
