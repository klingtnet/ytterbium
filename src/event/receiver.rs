extern crate portmidi as midi;
extern crate rosc;

use errors::RunError;
use std::net::{UdpSocket, Ipv4Addr};
use std::sync::mpsc;
use std::time::Duration;
use std::str::FromStr;
use std::thread;

#[derive(Debug)]
pub enum RawControlEvent {
    Osc(rosc::OscPacket),
    Midi(midi::MidiEvent),
}

pub trait Receiver {
    fn receive_and_send(&mut self, mpsc::Sender<RawControlEvent>);
}

pub struct OscReceiver {
    socket: UdpSocket,
    buf: [u8; rosc::decoder::MTU],
}
impl OscReceiver {
    pub fn new(ipv4: String, port: u16) -> Result<Self, RunError> {
        let ipv4_addr = try!(Ipv4Addr::from_str(&ipv4).map_err(|err| RunError::AddrError(err)));
        let socket = try!(UdpSocket::bind((ipv4_addr, port as u16))
                              .map_err(|err| RunError::IoError(err)));
        Ok(OscReceiver {
            socket: socket,
            buf: [0u8; rosc::decoder::MTU],
        })
    }
}
impl OscReceiver {
    fn receive(&mut self) -> Result<RawControlEvent, RunError> {
        let (size, addr) = try!(self.socket
                                    .recv_from(&mut self.buf)
                                    .map_err(|err| RunError::IoError(err)));
        rosc::decoder::decode(&self.buf)
            .map(|msg| RawControlEvent::Osc(msg))
            .map_err(|err| RunError::OscError(err))
    }
}
impl Receiver for OscReceiver {
    fn receive_and_send(&mut self, tx: mpsc::Sender<RawControlEvent>) {
        loop {
            match self.receive() {
                Ok(raw_event) => tx.send(raw_event).unwrap(),
                Err(RunError::OscError(err)) => println!("Could not decode osc packet: {:?}", err),
                err @ _ => panic!(err),
            }
        }
    }
}

pub struct MidiReceiver {
    context: midi::PortMidi,
    in_ports: Vec<midi::InputPort>,
    buf_len: usize,
}
impl MidiReceiver {
    pub fn new() -> Result<Self, RunError> {
        const BUF_LEN: usize = 1024;
        let context = try!(midi::PortMidi::new().map_err(|err| RunError::MidiError(err)));
        let in_devices = context.devices()
                                .unwrap()
                                .into_iter()
                                .filter(|dev| dev.is_input())
                                .collect::<Vec<midi::DeviceInfo>>();
        let in_ports = in_devices.into_iter()
                                 .filter_map(|dev| {
                                     context.input_port(dev, BUF_LEN)
                                            .ok()
                                 })
                                 .collect::<Vec<midi::InputPort>>();
        if in_ports.is_empty() {
            Err(RunError::NoMidiDeviceAvailable)
        } else {
            Ok(MidiReceiver {
                context: context,
                in_ports: in_ports,
                buf_len: BUF_LEN,
            })
        }
    }
}
impl MidiReceiver {
    fn receive(&self, port: &midi::InputPort) -> Result<Option<Vec<midi::MidiEvent>>, RunError> {
        port.read_n(self.buf_len).map_err(|err| RunError::MidiError(err))
    }
}
impl Receiver for MidiReceiver {
    fn receive_and_send(&mut self, tx: mpsc::Sender<RawControlEvent>) {
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

            while let Some(event) = event_buf.pop() {
                tx.send(RawControlEvent::Midi(event)).unwrap();
            }

            thread::sleep(timeout);
        }
    }
}
