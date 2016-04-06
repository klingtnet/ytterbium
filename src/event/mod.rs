extern crate rosc;
extern crate portmidi;

use rosc::OscPacket;

#[derive(Debug)]
pub enum ControlEvent {
    Unknown,
    Unsupported,
    NoteOn {
        key: u8,
        freq: f32,
        velocity: f32,
    },
    NoteOff {
        key: u8,
        velocity: f32,
    },
}

pub fn translate_osc(packet: rosc::OscPacket) -> ControlEvent {
    // TODO: map OSC packet to a ControlEvent
    match packet {
        OscPacket::Message(msg) => {
            println!("{:?}", msg);
            let addr: Vec<&str> = msg.addr.split('/').filter(|part| !part.is_empty()).collect();
            // TODO: differentiate how to handle args by means of address
            // this means, call a different match based on the control address
            match msg.args {
                Some(args) => {
                    // TODO: iterate over args and build message
                    match args[0] {
                        _ => ControlEvent::Unknown,
                    }
                }
                None => ControlEvent::NoEvent,
            }
        }
        OscPacket::Bundle(_) => ControlEvent::Unknown,
    }
}
