extern crate portmidi as midi;
extern crate rosc;

use errors::RunError;
use std::net::UdpSocket;
use std::sync::mpsc;

#[derive(Debug)]
pub enum RawControlEvent {
    Osc(rosc::OscPacket),
    Midi(midi::MidiEvent),
}

pub fn osc_receiver(socket: UdpSocket, tx: mpsc::Sender<RawControlEvent>) -> Result<(), RunError> {
    let mut buf = [0u8; rosc::decoder::MTU];
    loop {
        let (size, addr) = try!(socket.recv_from(&mut buf)
                                      .map_err(|err| RunError::SocketError(err)));
        match rosc::decoder::decode(&buf).map_err(|err| RunError::OscError(err)) {
            Ok(packet) => tx.send(RawControlEvent::Osc(packet)).unwrap(),
            Err(e) => println!("Osc packet decoding error: {:?}", e),
        }
    }
}

pub fn midi_receiver(tx: mpsc::Sender<RawControlEvent>) -> Result<(), RunError> {
    try!(midi::initialize().map_err(|err| RunError::MidiError(err)));
    match midi::count_devices() as usize {
        0 => {
            println!("No Midi device found");
            Ok(())
        }
        device_cnt @ _ => {
            let mut devices: Vec<midi::DeviceInfo> = Vec::with_capacity(device_cnt);
            for i in 0..device_cnt {
                midi::get_device_info(i as i32).map(|info| devices.push(info));
            }
            println!("Found the following midi-devices:");
            for device in devices {
                println!("\tid: {}, name: {}, type: {}",
                         device.device_id,
                         device.name,

                         if device.input {
                             "input"
                         } else {
                             "output"
                         });
            }
            midi::terminate().map_err(|err| RunError::MidiError(err))
        }
    }
}


