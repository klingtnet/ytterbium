extern crate rosc;

use rosc::{OscPacket, OscMessage, OscType};

use errors::RunError;
use std::net::{UdpSocket, SocketAddr};
use std::sync::mpsc;

use io::Receiver;

use event::ControlEvent;
use dsp::Waveform;
use types::*;

macro_rules! exp_scale {
    ($val:expr) => {
        ((($val as Float) * ::std::f64::consts::LN_10).exp() - 1.0)/10.0
    }
}

const TRANSPOSE_RANGE: usize = 6; // Octaves
const GRID_SIZE: usize = 96;

pub struct OscReceiver {
    socket: UdpSocket,
    buf: [u8; rosc::decoder::MTU],
    transpose: u8,
    note_grid: [f32; GRID_SIZE + TRANSPOSE_RANGE * 12],
}
impl OscReceiver {
    pub fn new(addr: SocketAddr) -> Result<Self, RunError> {
        let socket = try!(UdpSocket::bind(addr).map_err(RunError::IoError));
        Ok(OscReceiver {
            socket: socket,
            buf: [0u8; rosc::decoder::MTU],
            transpose: 0u8,
            note_grid: [0.0; GRID_SIZE + TRANSPOSE_RANGE * 12],
        })
    }
}
impl OscReceiver {
    fn receive(&mut self) -> Result<OscPacket, RunError> {
        let (size, _) = try!(self.socket
            .recv_from(&mut self.buf)
            .map_err(RunError::IoError));
        rosc::decoder::decode(&self.buf[..size]).map_err(RunError::OscError)
    }

    fn as_control_event(&mut self, packet: OscPacket) -> Vec<ControlEvent> {
        let mut events = Vec::new();
        for msg in Self::unwrap_packet(packet) {
            let parts = msg.addr.split('/').filter(|s| !s.is_empty()).collect::<Vec<&str>>();
            if parts.is_empty() {
                continue;
            }
            match parts[0] {
                "KEYS" => self.handle_keys(&msg, &parts, &mut events),
                "OSCILLATORS" => self.handle_oscillators(&msg, &parts[1..], &mut events),
                _ => println!("unmapped message: {:?}", msg),
            }
        }
        events
    }

    fn handle_keys(&mut self, msg: &OscMessage, address: &[&str], events: &mut Vec<ControlEvent>) {
        if address.len() < 3 {
            return;
        }
        match (address[1], address[2]) {
            ("GRID", "x") => {
                if msg.args.is_none() {
                    return;
                }
                for (idx, key) in msg.args.as_ref().unwrap().into_iter().enumerate() {
                    if let OscType::Float(velocity) = *key {
                        // The key grid goes from left to right and bottom to up, i.e. the lowest
                        // note is in the bottom left corner and the highest note in the top right.

                        let rev_idx = 12 * ((idx / 12) + 1) - idx % 12;
                        let transposed_key = (GRID_SIZE - rev_idx) as u8 + self.transpose;
                        // Determine if a `NoteOn` or `NoteOff` event was received by
                        // subtracting the last velocity from the received one.
                        // A negative difference determines a `NoteOff` and a positive
                        // one obviously a `NoteOn`.
                        let old_velocity = self.note_grid[transposed_key as usize];
                        if feq!(velocity, old_velocity) {
                            continue;
                        }
                        self.note_grid[transposed_key as usize] = velocity;
                        if velocity > old_velocity {
                            events.push(ControlEvent::NoteOn {
                                key: transposed_key,
                                velocity: velocity as Float,
                            });
                        } else {
                            events.push(ControlEvent::NoteOff {
                                key: transposed_key,
                                velocity: velocity as Float,
                            });
                        }
                    }
                }
            }
            ("TRANSPOSE", "x") => {
                let args = msg.args.as_ref().unwrap();
                // The transpose setting only modifies the internal state of the OSC Receiver by
                // updating the `transpose` value which is added to every received key.
                // TODO: This could also be done completly by the Lemur App.
                if let OscType::Float(scale) = args[0] {
                    self.transpose = (scale * 6.0) as u8 * 12;
                }
            }
            _ => {}
        }
    }

    fn handle_oscillators(&mut self,
                          msg: &OscMessage,
                          address: &[&str],
                          events: &mut Vec<ControlEvent>) {
        if address.len() < 3 {
            return;
        }
        // TODO: refactor this mess
        match address[0] {
            "MIX" => {
                match (address[1], address[2]) {
                    ("PAN", "x") => {
                        let args = msg.args
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|arg| match *arg {
                                OscType::Float(val) => val as Float,
                                _ => 0.0,
                            })
                            .collect::<Vec<_>>();
                        events.push(ControlEvent::Pan(args));
                    }

                    ("VOLUME", "x") => {
                        let args = msg.args
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|arg| match *arg {
                                OscType::Float(val) => (1.0 - val as Float) * -61.0,
                                _ => 0.0,
                            })
                            .collect::<Vec<_>>();
                        events.push(ControlEvent::Volume(args));
                    }
                    _ => {}
                }
            }
            "FM" => {
                match address[1] {
                    "OSC1" | "OSC2" | "OSC3" | "OSC4" => {
                        if let ("LEVEL", "x") = (address[2], address[3]) {
                            let args = msg.args
                                .as_ref()
                                .unwrap()
                                .iter()
                                .map(|arg| match *arg {
                                    OscType::Float(val) => val as Float,
                                    _ => 0.0,
                                })
                                .collect::<Vec<Float>>();
                            events.push(ControlEvent::FM {
                                id: address[1].to_owned(),
                                levels: args,
                            });
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                match (address[1], address[2]) {
                    ("ADSR", "x") => {
                        let args = msg.args
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|arg| {
                                match *arg {
                                    // TODO: Choose a quadratic scale?
                                    OscType::Float(val) => exp_scale!(val),
                                    _ => 1.0E-4,
                                }
                            })
                            .collect::<Vec<Float>>();
                        let id_prefix: String = "ADSR-".to_owned();
                        events.push(ControlEvent::ADSR {
                            id: id_prefix + address[0],
                            attack: 10.0 * args[0] as Time,
                            decay: 20.0 * args[1] as Time,
                            sustain: Float::from_db((1.0 - args[2]) * -40.0),
                            release: 20.0 * args[3] as Time,
                        });
                    }
                    ("PHASE", "x") => {
                        let args = msg.args.as_ref().unwrap();
                        if let OscType::Float(phase) = args[0] {
                            events.push(ControlEvent::Phase {
                                id: address[0].to_owned(),
                                phase: phase as Float,
                            });
                        }
                    }
                    ("TRANSPOSE", "x") => {
                        let args = msg.args.as_ref().unwrap();
                        if let OscType::Float(transpose) = args[0] {
                            events.push(ControlEvent::Transpose {
                                id: address[0].to_owned(),
                                transpose: transpose as i32,
                            });
                        }
                    }
                    ("DETUNE", "x") => {
                        let args = msg.args.as_ref().unwrap();
                        if let OscType::Float(detune) = args[0] {
                            events.push(ControlEvent::Detune {
                                id: address[0].to_owned(),
                                detune: (detune * 100.0) as i32,
                            });
                        }
                    }
                    ("WAVEFORM", "selection") => {
                        let args = msg.args.as_ref().unwrap();
                        if let OscType::Float(selection) = args[0] {
                            if let Some(waveform) = match selection as usize {
                                0 => Some(Waveform::Sine),
                                1 => Some(Waveform::Saw),
                                2 => Some(Waveform::Square),
                                3 => Some(Waveform::Tri),
                                4 => Some(Waveform::SharpTri),
                                5 => Some(Waveform::Random),
                                _ => None,
                            } {
                                events.push(ControlEvent::Waveform {
                                    id: address[0].to_owned(),
                                    waveform: waveform,
                                })
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn unwrap_packet(packet: OscPacket) -> Vec<OscMessage> {
        let mut messages = Vec::new();
        if let OscPacket::Bundle(bundle) = packet {
            if !bundle.content.is_empty() {
                for packet in bundle.content {
                    if let OscPacket::Message(msg) = packet {
                        messages.push(msg);
                    } else {
                        println!("Not a message in contents: {:?}", packet);
                    }
                }
            }
        }
        messages
    }
}
impl Receiver for OscReceiver {
    fn receive_and_send(&mut self, tx: mpsc::Sender<ControlEvent>) {
        loop {
            match self.receive() {
                Ok(packet) => {
                    let events = self.as_control_event(packet);
                    for event in events {
                        match event {
                            ControlEvent::Unsupported => continue,
                            _ => tx.send(event).unwrap(),
                        }
                    }
                }
                Err(RunError::OscError(err)) => println!("Could not decode osc packet: {:?}", err),
                err => panic!(err),
            }
        }
    }
}
