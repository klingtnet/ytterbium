extern crate rosc;
extern crate portmidi;

use types::*;

use dsp::Waveform;

macro_rules! check_address {
    ($address:expr, $id:expr) => {
        if !$address.is_empty() {
            $address[1..].starts_with(&$id)
        } else {
            false
        }
    }
}

macro_rules! feq {
    ($lhs:expr, $rhs:expr) => {
        ($lhs - $rhs).abs() < 1.0E-7
    }
}


#[derive(Debug,Clone)]
pub enum ControlEvent {
    Unsupported,
    NoteOn {
        key: u8,
        freq: Float,
        velocity: Float,
    },
    NoteOff {
        key: u8,
        velocity: Float,
    },
    ADSR {
        address: String,
        attack: Time,
        decay: Time,
        sustain: Float,
        release: Time,
    },
    Waveform {
        address: String,
        waveform: Waveform,
    },
    Volume {
        address: String,
        volume: Float,
    },
    Pan {
        address: String,
        pan: Float,
    },
    Phase {
        address: String,
        phase: Float,
    },
    Transpose {
        address: String,
        transpose: i32,
    },
    Detune {
        address: String,
        detune: i32,
    }
}

pub trait Controllable {
    fn handle(&mut self, msg: &ControlEvent);
}
