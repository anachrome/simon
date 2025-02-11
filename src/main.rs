mod note;

use std::sync::mpsc::{channel, Receiver};

use rand::Rng;

use serde_derive::Serialize;

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
        key: rand::thread_rng().gen_range(0..12),
        min_octave: note::MIDDLE_OCTAVE - 1,
        max_octave: note::MIDDLE_OCTAVE + 1,
    };

    let tonic = note::Note {
        pitch: (note::MIDDLE_C + game.key).into(),
        velocity: 64.into(),
        duration: std::time::Duration::from_millis(500u64),
    };
    tonic.play_on(&mut conn_out);

    println!("tonic: {}", tonic.pitch);

    while read_single_pitch(&receiver) != tonic.pitch {
        println!("pitch is not tonic");
        // wait for the user to acknowledge the tonic
    }
    std::thread::sleep(std::time::Duration::from_millis(500));

    play_cadence(game.key, &mut conn_out);
    std::thread::sleep(std::time::Duration::from_millis(250u64));

    const TRIES: u64 = 20u64;
    let mut successes = 0;
    for _ in 0..TRIES {

        let secret_note = note::Note {
            //pitch: note::Pitch::new(rand::random(), rand::thread_rng().gen_range(self.min_octave .. self.max_octave)),
            pitch: note::random_pitch(game.key, game.min_octave, game.max_octave).into(),
            velocity: 64.into(),
            duration: std::time::Duration::from_millis(500u64),
        };

        secret_note.play_on(&mut conn_out);

        while let Ok(_) = receiver.try_recv() {
            // ignore any keys pressed before/while the phrase is played
        }

        if secret_note.pitch == read_single_pitch(&receiver) {
            successes += 1;
        } else {
            while secret_note.pitch != read_single_pitch(&receiver) {
                // do not progress to the next phrase until the right note is played
                println!("checking one extra guess");
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    play_cadence(game.key, &mut conn_out);

    println!("{} / {}", successes, TRIES);
    log_stats(&game.filename(), Stats{ tries: TRIES, successes: successes })
}

fn play_cadence(key: u8, conn: &mut midir::MidiOutputConnection) {
    let dominant_chord = note::Chord {
        pitches: [-5i8, 2, 5, 11].iter()
                              .map(|chord_tone| ((chord_tone + note::MIDDLE_C as i8 + key as i8) as u8).into())
                              .collect(),
        velocity: 64.into(),
        duration: std::time::Duration::from_millis(750u64),
    };
    let tonic_chord = note::Chord {
        pitches: [0, 4, 7, 12].iter()
                              .map(|chord_tone| (chord_tone + note::MIDDLE_C + key).into())
                              .collect(),
        velocity: 64.into(),
        duration: std::time::Duration::from_millis(750u64),
    };
    dominant_chord.play_on(conn);
    tonic_chord.play_on(conn);
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Stats {
    tries: u64,
    successes: u64,
}

fn log_stats(filename: &str, stats: Stats) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = home::home_dir().ok_or("cannot find home directory")?;
    let stats_dir = home_dir.join(".simon");
    std::fs::create_dir_all(&stats_dir)?;
    let stats_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(stats_dir.join(filename))?;
    let mut csv_writer = csv::Writer::from_writer(stats_file);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    csv_writer.serialize((timestamp, stats))?;

    Ok(())
}

trait Playable {
    fn play_on(&self, conn: &mut midir::MidiOutputConnection);
}

trait Game {
    type Phrase: Playable;

    fn filename(&self) -> String;
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
                MidiMessage::NoteOff  { key, vel: _ } if key == note =>
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

    fn filename(&self) -> String {
        format!{"single-note-{}-major-{}-octaves.csv", note::KEYS[self.key as usize], self.max_octave - self.min_octave}
    }
}
