extern crate rosc;
extern crate portmidi;

pub mod router;

use io::MidiEvent;
use rosc::OscPacket;

#[derive(Debug)]
pub enum RawControlEvent {
    Osc(OscPacket),
    Midi(portmidi::MidiEvent),
}

#[derive(Debug)]
pub enum ControlEvent {
    Unknown,
    Unsupported,
    NoEvent,
    NoteOn {
        key: u8,
        velocity: f32,
    },
    NoteOff {
        key: u8,
        velocity: f32,
    },
}
impl From<RawControlEvent> for ControlEvent {
    fn from(raw: RawControlEvent) -> ControlEvent {
        match raw {
            RawControlEvent::Osc(packet) => translate_osc(packet),
            RawControlEvent::Midi(event) => translate_midi(event),
        }
    }
}

pub fn translate_osc(packet: rosc::OscPacket) -> ControlEvent {
    // TODO: map OSC packet to a ControlEvent
    match packet {
        OscPacket::Message(msg) => {
            println!("{:?}", msg);
            let addr: Vec<&str> = msg.addr.split('/').filter(|part| !part.is_empty()).collect();
            // TODO: differentiate how to handle args by means of address
            // this means, call a different match based on the control address
            match msg.args {
                Some(args) => {
                    // TODO: iterate over args and build message
                    match args[0] {
                        _ => ControlEvent::Unknown,
                    }
                }
                None => ControlEvent::NoEvent,
            }
        }
        OscPacket::Bundle(_) => ControlEvent::Unknown,
    }
}

// TODO: do MidiTuningStandard conversion here and return a NoteOn with frequency and velocity
pub fn translate_midi(event: portmidi::MidiEvent) -> ControlEvent {
    match MidiEvent::from(event) {
        MidiEvent::NoteOn{key, velocity, ..} => {
            ControlEvent::NoteOn {
                key: key,
                velocity: velocity,
            }
        }
        MidiEvent::NoteOff{key, velocity, ..} => {
            ControlEvent::NoteOff {
                key: key,
                velocity: velocity,
            }
        }
        _ => ControlEvent::Unsupported,
    }
}
