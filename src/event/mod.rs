extern crate rosc;
extern crate portmidi;

#[derive(Debug)]
pub enum ControlEvent {
    Unknown,
    Unsupported,
    NoteOn {
        key: u8,
        freq: f32,
        velocity: f32,
    },
    NoteOff {
        key: u8,
        velocity: f32,
    },
}
