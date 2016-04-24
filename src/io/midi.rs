extern crate portmidi;

use types::*;
use errors::RunError;
use std::sync::mpsc;
use std::time::Duration;
use std::thread;

use io::Receiver;

use event::ControlEvent;

pub struct MidiReceiver {
    context: portmidi::PortMidi,
    in_ports: Vec<portmidi::InputPort>,
    buf_len: usize,
    pitch_convert: PitchConvert,
}
impl MidiReceiver {
    pub fn new() -> Result<Self, RunError> {
        const BUF_LEN: usize = 1024;
        let context = try!(portmidi::PortMidi::new().map_err(RunError::MidiError));
        let in_devices = context.devices()
                                .unwrap()
                                .into_iter()
                                .filter(|dev| dev.is_input())
                                .collect::<Vec<portmidi::DeviceInfo>>();
        let in_ports = in_devices.into_iter()
                                 .filter_map(|dev| {
                                     context.input_port(dev, BUF_LEN)
                                            .ok()
                                 })
                                 .collect::<Vec<portmidi::InputPort>>();
        if in_ports.is_empty() {
            Err(RunError::NoMidiDeviceAvailable)
        } else {
            Ok(MidiReceiver {
                context: context,
                in_ports: in_ports,
                buf_len: BUF_LEN,
                pitch_convert: PitchConvert::new(440.0),
            })
        }
    }
}
impl MidiReceiver {
    fn receive(&self,
               port: &portmidi::InputPort)
               -> Result<Option<Vec<portmidi::MidiEvent>>, RunError> {
        port.read_n(self.buf_len).map_err(RunError::MidiError)
    }

    fn to_control_event(&self, event: MidiEvent) -> ControlEvent {
        match event {
            MidiEvent::NoteOn { key, velocity, .. } => {
                ControlEvent::NoteOn {
                    key: key,
                    freq: self.pitch_convert.key_to_hz(key),
                    velocity: velocity,
                }
            }
            MidiEvent::NoteOff { key, velocity, .. } => {
                ControlEvent::NoteOff {
                    key: key,
                    velocity: velocity,
                }
            }
            _ => ControlEvent::Unsupported,
        }
    }
}
impl Receiver for MidiReceiver {
    fn receive_and_send(&mut self, tx: mpsc::Sender<ControlEvent>) {
        let mut event_buf = Vec::with_capacity(self.buf_len);
        let timeout = Duration::from_millis(20);
        loop {
            for port in &self.in_ports {
                match self.receive(port) {
                    Ok(Some(mut events)) => event_buf.append(&mut events),
                    Ok(_) => (),
                    Err(RunError::MidiError(err)) => println!("receive_and_send) Error: {:?}", err),
                    Err(err) => panic!(err),
                }
            }

            event_buf.sort_by_key(|e| e.timestamp);
            while let Some(event) = event_buf.pop() {
                tx.send(self.to_control_event(MidiEvent::from(event))).unwrap();
            }

            thread::sleep(timeout);
        }
    }
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
            0xF2 => MidiEvent::SongPosition(data1 as u16 + ((data2 as u16) << 8)),
            0xF3 => MidiEvent::SongSelect(data1 as u8),
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
                            key: data1 as u8,
                            velocity: data2 as Float / 127.0,
                            channel: channel,
                        }
                    }
                    0x90 => {
                        MidiEvent::NoteOn {
                            key: data1 as u8,
                            velocity: data2 as Float / 127.0,
                            channel: channel,
                        }
                    }
                    0xA0 => {
                        MidiEvent::PolyphonicKeyPressure {
                            key: data1 as u8,
                            velocity: data2 as Float / 127.0,
                            channel: channel,
                        }
                    }
                    0xB0 => {
                        match data1 {
                            120...127 => MidiEvent::Unsupported,
                            _ => {
                                MidiEvent::ControlChange {
                                    controller: data1 as u8,
                                    value: data2 as Float / 127.0,
                                    channel: channel,
                                }
                            }
                        }
                    }
                    0xC0 => {
                        MidiEvent::ProgramChange {
                            program: data1 as u8,
                            channel: channel,
                        }
                    }
                    0xD0 => {
                        MidiEvent::ChannelPressure {
                            pressure: data1 as u8,
                            channel: channel,
                        }
                    }
                    0xE0 => {
                        MidiEvent::PitchBend {
                            pitchbend: data1 as u16 + ((data2 as u16) << 8),
                            channel: channel,
                        }
                    }
                    _ => MidiEvent::Unknown,
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum MidiEvent {
    Unknown,
    Unsupported,
    NoteOn {
        key: u8,
        velocity: Float,
        channel: u8,
    },
    NoteOff {
        key: u8,
        velocity: Float,
        channel: u8,
    },
    PolyphonicKeyPressure {
        key: u8,
        velocity: Float,
        channel: u8,
    },
    ControlChange {
        controller: u8,
        value: Float,
        channel: u8,
    },
    ProgramChange {
        program: u8,
        channel: u8,
    },
    ChannelPressure {
        pressure: u8,
        channel: u8,
    },
    PitchBend {
        pitchbend: u16,
        channel: u8,
    },
    SysEx,
    SysExEnd,
    TimeCodeQuarterFrame {
        msg_type: u8,
        value: u8,
    },
    SongPosition(u16),
    SongSelect(u8),
    TuneRequest,
    TimingClock,
    Start,
    Stop,
    Continue,
    ActiveSensing,
    Reset,
}

struct PitchConvert {
    table: Vec<Float>,
}
impl PitchConvert {
    pub fn new(tune_freq: Float) -> Self {
        PitchConvert {
            // see https://en.wikipedia.org/wiki/MIDI_Tuning_Standard
            table: (0..128)
                       .map(|key| {
                           let dist_concert_a = key as isize - 69;
                           let two: Float = 2.0;
                           two.powf(dist_concert_a as Float / 12.0) * tune_freq
                       })
                       .collect::<Vec<_>>(),
        }
    }

    pub fn key_to_hz(&self, key: u8) -> Float {
        if (key as usize) < self.table.len() {
            self.table[key as usize]
        } else {
            self.table[self.table.len() - 1]
        }
    }
}
