extern crate portmidi;
extern crate rosc;

use types::*;

use dsp::{FilterType, Waveform};

macro_rules! feq {
    ($lhs:expr, $rhs:expr) => {
        ($lhs - $rhs).abs() < 1.0E-7
    };
}

#[derive(Debug, Clone)]
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
    Volume(Vec<Float>),
    Pan(Vec<Float>),
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
    FM {
        id: String,
        levels: Vec<Float>,
    },
    Filter {
        filter_type: Option<FilterType>,
        freq: Option<Float>,
        q: Option<Float>,
    },
}

pub trait Controllable {
    fn handle(&mut self, msg: &ControlEvent);
}
