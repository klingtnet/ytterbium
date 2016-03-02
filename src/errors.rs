extern crate rosc;
extern crate portmidi as midi;
extern crate rsoundio;

use std::net::AddrParseError;
use std::io;

#[derive(Debug)]
pub enum RunError {
    Unknown,
    Unimplemented,
    AddrError(AddrParseError),
    SocketError(io::Error),
    OscError(rosc::OscError),
    MidiError(midi::PortMidiError),
    ThreadError(String),
    AudioError(rsoundio::SioError),
}
