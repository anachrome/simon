mod note;

use std::sync::mpsc::{channel, Receiver};

use rand::Rng;

use midly::live::LiveEvent;
use midly::MidiMessage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //
    // set up midi
    //

    let mut midi_in = midir::MidiInput::new("midir test input")?;
    let midi_out = midir::MidiOutput::new("midir test output")?;

    // we may eventually care about this for identifying dcs, buuuuut not now
    midi_in.ignore(midir::Ignore::ActiveSense);

    // TODO(lily) streamline this

    println!("available input ports:");
    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p)?);
    }

    println!("available output ports:");
    for (i, p) in midi_out.ports().iter().enumerate() {
        println!("{}: {}", i, midi_out.port_name(p)?);
    }

    println!("");

    let out_port = &midi_out.ports()[1];
    let in_port = &midi_in.ports()[1];

    let (sender, receiver) = channel();
    let mut conn_out = midi_out.connect(out_port, "midir-test")?;
    let mut _conn_in = midi_in.connect(in_port, "midir-test", move |_stamp, bytes, _| {
        let event = LiveEvent::parse(&bytes).unwrap();

        if let LiveEvent::Midi { channel: midi_channel, message: midi_message } = event {
            sender.send((midi_channel, midi_message)).unwrap();
        } else {
            println!("received: {:?}", event);
        }
    }, ())?;

    //
    // main loop
    //

    let game = SingleNoteGame {
        key: rand::thread_rng().gen_range(0..13),
        min_octave: 3,
        max_octave: 5,
    };

    let tonic = note::Note {
        pitch: (note::MIDDLE_OCTAVE + game.key).into(),
        velocity: 64.into(),
        duration: std::time::Duration::from_millis(500u64),
    };
    tonic.play_on(&mut conn_out);


    while read_single_pitch(&receiver) != tonic.pitch {
        // wait for the user to acknowledge the tonic
    }

    // TODO: play cadence or other sort of introductory material here
    std::thread::sleep(std::time::Duration::from_millis(500));

    const TRIES: u64 = 20u64;
    let mut successes = 0;
    for _ in 0..TRIES {

        let phrase = game.gen_phrase();
        phrase.play_on(&mut conn_out);

        while let Ok(_) = receiver.try_recv() {
            // ignore any keys pressed before/while the phrase is played
        }

        if SingleNoteGame::check_guess(phrase, &receiver) {
            successes += 1;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    //
    // write out stats
    //

    let home_dir = home::home_dir().ok_or("cannot find home directory")?;
    let stats_dir = home_dir.join(".simon");
    std::fs::create_dir_all(&stats_dir)?;
    let stats_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(stats_dir.join(game.filename()))?;
    let mut csv_writer = csv::Writer::from_writer(stats_file);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    csv_writer.serialize(&[timestamp, TRIES, successes])?;

    println!("{} / {}", successes, TRIES);

    Ok(())
}

trait Playable {
    fn play_on(&self, conn: &mut midir::MidiOutputConnection);
}

trait Game {
    type Phrase: Playable;

    fn key(&self) -> &str;
    fn filename(&self) -> String;
    // TODO: introductory + intermission cadences etc.
    fn gen_phrase(&self) -> Self::Phrase;
    fn check_guess(phrase: Self::Phrase, receiver: &Receiver<(midly::num::u4, midly::MidiMessage)>) -> bool;
}

// TODO: generic read phrase
fn read_single_pitch(receiver: &Receiver<(midly::num::u4, midly::MidiMessage)>) -> midly::num::u7 {
    let mut note = None;

    loop {
        let (midi_channel, midi_message) = receiver.recv().unwrap();
        println!("{:?}, received: {:?}", midi_channel, midi_message);

        // if note is None, then we just read the first input into note; if note is Some(...), then
        // we wait until it is released before returning
        if let Some(note) = note {
            match midi_message {
                MidiMessage::NoteOn  { key, vel } if key == note && vel == 0 =>
                    return note,
                _ => continue,
            }
        } else {
            match midi_message {
                MidiMessage::NoteOn { key, vel } if vel > 0 =>
                    note = Some(key),
                _ => continue,
            }
        }
    }
}

impl Playable for note::Note {
    fn play_on(&self, conn: &mut midir::MidiOutputConnection) {
        let mut on_msg = Vec::new();
        LiveEvent::Midi {
            channel: 0u8.into(),
            message: MidiMessage::NoteOn {
                key: self.pitch,
                vel: self.velocity,
            }
        }.write(&mut on_msg).unwrap();
        conn.send(&on_msg).unwrap();

        std::thread::sleep(self.duration);

        let mut off_msg = Vec::new();
        LiveEvent::Midi {
            channel: 0u8.into(),
            message: MidiMessage::NoteOff {
                key: self.pitch,
                vel: self.velocity,
            }
        }.write(&mut off_msg).unwrap();
        conn.send(&off_msg).unwrap();
    }
}

impl Playable for note::Chord {
    fn play_on(&self, conn: &mut midir::MidiOutputConnection) {
        for pitch in &self.pitches {
            let mut on_msg = Vec::new();
            LiveEvent::Midi {
                channel: 0u8.into(),
                message: MidiMessage::NoteOn {
                    key: *pitch,
                    vel: self.velocity,
                }
            }.write(&mut on_msg).unwrap();
            conn.send(&on_msg).unwrap();
        }

        std::thread::sleep(self.duration);

        for pitch in &self.pitches {
            let mut off_msg = Vec::new();
            LiveEvent::Midi {
                channel: 0u8.into(),
                message: MidiMessage::NoteOff {
                    key: *pitch,
                    vel: self.velocity,
                }
            }.write(&mut off_msg).unwrap();
            conn.send(&off_msg).unwrap();
        }
    }
}

struct SingleNoteGame {
    key: u8,
    min_octave: u8,
    max_octave: u8,
}

impl Game for SingleNoteGame {
    type Phrase = note::Note;

    fn key(&self) -> &str {
        note::keys[self.key as usize]
    }

    fn filename(&self) -> String {
        format!{"single-note-{:?}-major-{}-octaves.csv", self.key, self.max_octave - self.min_octave}
    }

    fn gen_phrase(&self) -> note::Note {
        note::Note {
            //pitch: note::Pitch::new(rand::random(), rand::thread_rng().gen_range(self.min_octave .. self.max_octave)),
            pitch: note::random_pitch(self.key, self.min_octave, self.max_octave).into(),
            velocity: 64.into(),
            duration: std::time::Duration::from_millis(500u64),
        }
    }

    fn check_guess(phrase: note::Note, receiver: &Receiver<(midly::num::u4, midly::MidiMessage)>) -> bool {
        phrase.pitch == read_single_pitch(receiver)
    }
}
