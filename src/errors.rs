extern crate rosc;
extern crate portmidi as midi;
extern crate rsoundio;

use std::io;

#[derive(Debug)]
pub enum RunError {
    IoError(io::Error),
    OscError(rosc::OscError),
    MidiError(midi::Error),
    NoMidiDeviceAvailable,
}
