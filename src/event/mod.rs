extern crate rosc;
extern crate portmidi;

use types::*;

#[derive(Debug)]
pub enum ControlEvent {
    Unknown,
    Unsupported,
    NoteOn {
        key: u8,
        freq: Float,
        velocity: Float,
    },
    NoteOff {
        key: u8,
        velocity: Float,
    },
}

pub trait Controllable {
    fn handle(&mut self, msg: &ControlEvent);
}
