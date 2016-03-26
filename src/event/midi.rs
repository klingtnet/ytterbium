extern crate portmidi;

#[derive(Debug)]
pub enum MidiEvent {
    Unknown,
    Unsupported,
    NoEvent,
    NoteOn {
        key: u32,
        velocity: f32,
        channel: u8,
    },
    NoteOff {
        key: u32,
        velocity: f32,
        channel: u8,
    },
    PolyphonicKeyPressure {
        key: u32,
        velocity: f32,
        channel: u8,
    },
    ControlChange {
        controller: u32,
        value: f32,
        channel: u8,
    },
    ProgramChange {
        program: u32,
        channel: u8,
    },
    ChannelPressure {
        pressure: u32,
        channel: u8,
    },
    PitchBend {
        pitchbend: u32,
        channel: u8,
    },
    SysEx,
    SysExEnd,
    TimeCodeQuarterFrame {
        msg_type: u8,
        value: u8,
    },
    SongPosition(u32),
    SongSelect(u32),
    TuneRequest,
    TimingClock,
    Start,
    Stop,
    Continue,
    ActiveSensing,
    Reset,
}

impl From<portmidi::MidiEvent> for MidiEvent {
    fn from(event: portmidi::MidiEvent) -> Self {
        let status = event.message.status;
        let data1 = event.message.data1;
        let data2 = event.message.data2;
        match status {
            0xF0 => MidiEvent::SysEx,
            0xF1 => {
                MidiEvent::TimeCodeQuarterFrame {
                    msg_type: ((data1 & 0xF0) >> 4) as u8,
                    value: (data1 & 0x0F) as u8,
                }
            }
            0xF2 => MidiEvent::SongPosition(data1 as u32 + ((data2 as u32) << 8)),
            0xF3 => MidiEvent::SongSelect(data1 as u32),
            0xF6 => MidiEvent::TuneRequest,
            0xF7 => MidiEvent::SysExEnd,
            0xF8 => MidiEvent::TimingClock,
            0xFA => MidiEvent::Start,
            0xFB => MidiEvent::Continue,
            0xFC => MidiEvent::Stop,
            0xFE => MidiEvent::ActiveSensing,
            0xFF => MidiEvent::Reset,
            0xF4 | 0xF5 | 0xF9 | 0xFD => MidiEvent::Unknown,
            _ => {
                let channel = status & 0x0F;
                // TODO: nested enum for ChannelMode messages?
                match status & 0xF0 {
                    0x80 => {
                        MidiEvent::NoteOff {
                            key: data1 as u32,
                            velocity: data2 as f32 / 127f32,
                            channel: channel,
                        }
                    }
                    0x90 => {
                        MidiEvent::NoteOn {
                            key: data1 as u32,
                            velocity: data2 as f32 / 127f32,
                            channel: channel,
                        }
                    }
                    0xA0 => {
                        MidiEvent::PolyphonicKeyPressure {
                            key: data1 as u32,
                            velocity: data2 as f32 / 127f32,
                            channel: channel,
                        }
                    }
                    0xB0 => {
                        match data1 {
                            120...127 => MidiEvent::Unsupported,
                            _ => {
                                MidiEvent::ControlChange {
                                    controller: data1 as u32,
                                    value: data2 as f32 / 127f32,
                                    channel: channel,
                                }
                            }
                        }
                    }
                    0xC0 => {
                        MidiEvent::ProgramChange {
                            program: data1 as u32,
                            channel: channel,
                        }
                    }
                    0xD0 => {
                        MidiEvent::ChannelPressure {
                            pressure: data1 as u32,
                            channel: channel,
                        }
                    }
                    0xE0 => {
                        MidiEvent::PitchBend {
                            pitchbend: data1 as u32 + ((data2 as u32) << 8),
                            channel: channel,
                        }
                    }
                    _ => MidiEvent::Unknown,
                }
            }
        }
    }
}
