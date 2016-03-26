extern crate rosc;
extern crate portmidi;

use std::sync::mpsc;
use rosc::{OscPacket, OscType};

use event::midi::MidiEvent;
use event::receiver::RawControlEvent;

#[derive(Debug)]
pub enum ControlEvent {
    Unknown,
    Unsupported,
    NoEvent,
    NoteOn {
        key: u32,
        velocity: f32,
    },
    NoteOff {
        key: u32,
        velocity: f32,
    },
    Start,
    Stop,
    Continue,
}
impl From<RawControlEvent> for ControlEvent {
    fn from(raw: RawControlEvent) -> ControlEvent {
        match raw {
            RawControlEvent::Osc(packet) => translate_osc(packet),
            RawControlEvent::Midi(event) => translate_midi(event),
        }
    }
}

pub struct EventRouter<R, S: From<R>> {
    rx: mpsc::Receiver<R>,
    tx: mpsc::Sender<S>,
}
impl<R, S: From<R>> EventRouter<R, S> {
    pub fn new(rx: mpsc::Receiver<R>, tx: mpsc::Sender<S>) -> Self {
        EventRouter { tx: tx, rx: rx }
    }

    pub fn route(&self) {
        loop {
            let in_msg = self.rx.recv().unwrap();
            self.tx.send(S::from(in_msg)).unwrap();
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
                        OscType::Float(x) => {
                            if x == 1.0 {
                                ControlEvent::NoteOn
                            } else if x == 0.0 {
                                ControlEvent::NoteOff
                            } else {
                                ControlEvent::Unknown
                            }
                        }
                        _ => ControlEvent::Unknown,
                    }
                }
                None => ControlEvent::NoEvent,
            }
        }
        OscPacket::Bundle(_) => ControlEvent::Unknown,
    }
}

pub fn translate_midi(event: portmidi::MidiEvent) -> ControlEvent {
    match MidiEvent::from(event) {
        MidiEvent::NoteOn{key, velocity, ..} => ControlEvent::NoteOn{key: key, velocity: velocity},
        MidiEvent::NoteOff{key, velocity, ..} => ControlEvent::NoteOff{key:key, velocity: velocity},
        _ => ControlEvent::Unsupported,
    }
}
