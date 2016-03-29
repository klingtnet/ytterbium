extern crate rosc;
extern crate portmidi as midi;
extern crate rsoundio;

use std::net::AddrParseError;
use std::io;

#[derive(Debug)]
pub enum RunError {
    AddrError(AddrParseError),
    IoError(io::Error),
    OscError(rosc::OscError),
    MidiError(midi::Error),
    NoMidiDeviceAvailable,
}
