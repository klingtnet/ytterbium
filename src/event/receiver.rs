extern crate portmidi as midi;
extern crate rosc;

use errors::RunError;
use std::net::{UdpSocket, Ipv4Addr};
use std::sync::mpsc;
use std::str::FromStr;

#[derive(Debug)]
pub enum RawControlEvent {
    Osc(rosc::OscPacket),
    Midi(midi::MidiEvent),
}

pub trait Receiver {
    fn receive(&mut self) -> Result<RawControlEvent, RunError>;
    fn receive_and_send(&mut self);
}

pub struct OscReceiver {
    socket: UdpSocket,
    tx: mpsc::Sender<RawControlEvent>,
    buf: [u8; rosc::decoder::MTU],
}
impl OscReceiver {
    pub fn new(ipv4: String,
               port: u16,
               tx: mpsc::Sender<RawControlEvent>)
               -> Result<Self, RunError> {
        let ipv4_addr = try!(Ipv4Addr::from_str(&ipv4).map_err(|err| RunError::AddrError(err)));
        let socket = try!(UdpSocket::bind((ipv4_addr, port as u16))
                              .map_err(|err| RunError::SocketError(err)));
        Ok(OscReceiver {
            socket: socket,
            tx: tx, 
            buf: [0u8; rosc::decoder::MTU],
        })
    }
}
impl Receiver for OscReceiver {
    fn receive(&mut self) -> Result<RawControlEvent, RunError> {
        let (size, addr) = try!(self.socket
                                    .recv_from(&mut self.buf)
                                    .map_err(|err| RunError::SocketError(err)));
        rosc::decoder::decode(&self.buf)
            .map(|msg| RawControlEvent::Osc(msg))
            .map_err(|err| RunError::OscError(err))
    }

    fn receive_and_send(&mut self) {
        loop {
            match self.receive() {
                Ok(raw_event) => self.tx.send(raw_event).unwrap(),
                Err(RunError::OscError(err)) => println!("Could not decode osc packet: {:?}", err),
                err @ _ => panic!(err),
            }
        }
    }
}

pub struct MidiReceiver {
    // context: Midi,
    tx: mpsc::Sender<RawControlEvent>,
}
impl MidiReceiver {
    pub fn new(tx: mpsc::Sender<RawControlEvent>) -> Result<Self, RunError> {
        Ok(MidiReceiver { tx: tx })
    }
}
impl Receiver for MidiReceiver {
    fn receive(&mut self) -> Result<RawControlEvent, RunError> {
        unimplemented!()
    }

    fn receive_and_send(&mut self) {
        unimplemented!()
    }
}
