extern crate rosc;

use errors::RunError;
use std::net::{UdpSocket, Ipv4Addr};
use std::sync::mpsc;
use std::str::FromStr;

use receiver::Receiver;
use event::RawControlEvent;

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
