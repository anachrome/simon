use rand::Rng;
use rand::seq::SliceRandom;
use rand::distributions::{Distribution, Standard};

pub const MIDDLE_OCTAVE: u8 = 4;
pub const MIDDLE_C: u8 = 60;

// a simple abstraction over midi events: notes with duration are an easier structure to work with
// than midi note-on and note-off events
#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub pitch: midly::num::u7,
    pub velocity: midly::num::u7,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct Chord {
    pub pitches: std::vec::Vec<midly::num::u7>,
    pub velocity: midly::num::u7,
    pub duration: std::time::Duration,
}

pub fn random_pitch(key: u8, lower_octave: u8, upper_octave: u8) -> u8 {
    // TODO: flesh out the types enough to get rid of these asserts
    assert!(key <= 12);
    assert!(lower_octave < upper_octave);

    let mut rng = rand::thread_rng();
    let mut notes = Vec::new();
    for octave in lower_octave..upper_octave {
        notes.extend(MAJOR_SCALE_INTERVALS.iter().map(|n| (12 * (octave + 1)) + key + n))
    }
    notes.push((12 * upper_octave) + key);

    *notes.choose(&mut rng).unwrap()
}

const MAJOR_SCALE_INTERVALS: &[u8] = &[0, 2, 4, 5, 7, 9, 11];
pub const KEYS: &[&str] = &["C", "Des", "D", "Es", "E", "F", "Ges", "G", "Aes", "A", "Bes", "B"];

// TODO something better for accidentals
#[derive(Clone, Copy, Debug)]
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
