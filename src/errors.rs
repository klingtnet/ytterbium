extern crate portmidi;
extern crate rosc;

#[derive(Debug)]
pub enum RunError {
    IoError(::std::io::Error),
    OscError(rosc::OscError),
    MidiError(portmidi::Error),
    NoMidiDeviceAvailable,
}
