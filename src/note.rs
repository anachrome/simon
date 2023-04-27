use rand::Rng;
use rand::seq::SliceRandom;
use rand::distributions::{Distribution, Standard};

pub const MIDDLE_OCTAVE: u8 = 4;
pub const MIDDLE_C: u8 = 60;

// struct major key
// contains tonal center (enharmonic notes merged, because this is a listening test)
// function that generates a random note, in the key, in some range
//   (should the range be specified in notes, octaves, ??)
// function that returns the name of the key (ideally in a filename-friendly format)

// TODO: handle other modes
struct Key {
    pub pitch_class: PitchClass,
}

// a simple abstraction over midi events: notes with duration are an easier structure to work with
// than midi note-on and note-off events
pub struct Note {
    pub pitch: midly::num::u7,
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
        notes.extend(major_scale_intervals.iter().map(|n| (12 * (octave + 1)) + key + n))
    }
    notes.push((12 * upper_octave) + key);

    *notes.choose(&mut rng).unwrap()
}

const major_scale_intervals: &[u8] = &[0, 2, 4, 5, 7, 9, 11];
pub const keys: &[&str] = &["C", "Des", "D", "Es", "E", "F", "Ges", "G", "Aes", "A", "Bes", "B"];

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
