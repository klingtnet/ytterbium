extern crate rosc;
extern crate portmidi;

use types::*;

use dsp::Waveform;

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
        velocity: Float,
    },
    NoteOff {
        key: u8,
        velocity: Float,
    },
    ADSR {
        id: String,
        attack: Time,
        decay: Time,
        sustain: Float,
        release: Time,
    },
    Waveform {
        id: String,
        waveform: Waveform,
    },
    Volume {
        id: String,
        volume: Float,
    },
    OscMixer {
        levels: Vec<Float>,
    },
    Pan {
        id: String,
        pan: Float,
    },
    Phase {
        id: String,
        phase: Float,
    },
    Transpose {
        id: String,
        transpose: i32,
    },
    Detune {
        id: String,
        detune: i32,
    },
    FmLevel {
        id: String,
        levels: [Float; 3],
    },
}

pub trait Controllable {
    fn handle(&mut self, msg: &ControlEvent);
}
