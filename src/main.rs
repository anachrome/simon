mod note;

use std::sync::mpsc::{channel, Receiver};
use std::thread::sleep;
use std::time::Duration;

use midly::live::LiveEvent;
use midly::MidiMessage;

// big todos
// single note key of C game w/ cadence + multiple octaves
// use midly for sending (get rid of play_note closure)
// more robust error handling

trait Playable {
    fn play_on(conn: midir::MidiOutputConnection);
}

trait Game<'a> {
    type Phrase: Playable;

    fn filename() -> &'a str;
    // TODO: introductory + intermission cadences etc.
    fn gen_phrase() -> Self::Phrase;
    fn recv_phrase(receiver: Receiver<(midly::num::u4, midly::MidiMessage)>) -> Self::Phrase;
    fn check_guess(phrase: Self::Phrase, guess: Self::Phrase) -> bool;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    {
        let mut play_note = |note: u8, duration: u64| {
            const NOTE_ON_MSG: u8 = 0x90;
            const NOTE_OFF_MSG: u8 = 0x80;
            const VELOCITY: u8 = 0x32;
            // We're ignoring errors in here
            let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
            sleep(Duration::from_millis(duration * 150));
            let _ = conn_out.send(&[NOTE_OFF_MSG, note, VELOCITY]);
        };

        let read_note = || {
            loop {
                let (midi_channel, midi_message) = receiver.recv().unwrap();

                println!("{:?}, received: {:?}", midi_channel, midi_message);
                match midi_message {
                    MidiMessage::NoteOn { key, vel } if vel > 0 => return key,
                    _ => continue,
                }
            }
        };

        const TRIES: u64 = 0u64;
        let mut successes = 0;
        for _ in 0..TRIES {

            let note = note::Pitch {
                pitch_class: rand::random(),
                octave: 4,
            };

            play_note(note.midi(), 4);

            while let Ok(_) = receiver.try_recv() {
                // ignore any keys pressed before/during the phrase was played
            }

            let guess = read_note();
            if u8::from(guess) == note.midi() {
                successes += 1;
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        let home_dir = home::home_dir().ok_or("cannot find home directory")?;
        let stats_dir = home_dir.join(".simon");
        std::fs::create_dir_all(&stats_dir)?;
        let stats_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(stats_dir.join("log.csv"))?;
        let mut csv_writer = csv::Writer::from_writer(stats_file);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        csv_writer.serialize(&[timestamp, TRIES, successes])?;

        println!("{} / {}", successes, TRIES);
    }

    Ok(())
}
