use rand::Rng;
use rand::seq::SliceRandom;
use rand::distributions::{Distribution, Standard};

// a simple abstraction over midi events: notes with duration are an easier structure to work with
// than midi note-on and note-off events
pub struct Note {
    pub pitch: Pitch,
    pub velocity: midly::num::u7,
    pub duration: std::time::Duration,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pitch {
    pub midi: u8
}

impl Pitch {
    pub fn new(pitch_class: PitchClass, octave: u8) -> Pitch {
        Pitch { midi: (12 * (octave + 1)) + pitch_class as u8 }
    }
}

// TODO something better for accidentals
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
