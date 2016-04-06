extern crate rosc;

use rosc::OscPacket;

use errors::RunError;
use std::net::{UdpSocket, SocketAddr};
use std::sync::mpsc;

use io::Receiver;

use event::ControlEvent;

pub struct OscReceiver {
    socket: UdpSocket,
    buf: [u8; rosc::decoder::MTU],
}
impl OscReceiver {
    pub fn new(addr: SocketAddr) -> Result<Self, RunError> {
        let socket = try!(UdpSocket::bind(addr).map_err(RunError::IoError));
        Ok(OscReceiver {
            socket: socket,
            buf: [0u8; rosc::decoder::MTU],
        })
    }
}
impl OscReceiver {
    fn receive(&mut self) -> Result<OscPacket, RunError> {
        let (size, _) = try!(self.socket
                                    .recv_from(&mut self.buf)
                                    .map_err(RunError::IoError));
        rosc::decoder::decode(&self.buf[..size])
            .map_err(RunError::OscError)
    }

    fn to_control_event(&self, event: OscPacket) -> ControlEvent {
        ControlEvent::Unknown
    }
}
impl Receiver for OscReceiver {
    fn receive_and_send(&mut self, tx: mpsc::Sender<ControlEvent>) {
        loop {
            match self.receive() {
                Ok(packet) => tx.send(self.to_control_event(packet)).unwrap(),
                Err(RunError::OscError(err)) => println!("Could not decode osc packet: {:?}", err),
                err => panic!(err),
            }
        }
    }
}
