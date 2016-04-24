extern crate rosc;
extern crate portmidi;

#[derive(Debug)]
pub enum RunError {
    IoError(::std::io::Error),
    OscError(rosc::OscError),
    MidiError(portmidi::Error),
    NoMidiDeviceAvailable,
}
