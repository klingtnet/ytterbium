mod midi;
mod osc;

use std::sync::mpsc;
use event::ControlEvent;

pub use self::midi::*;
pub use self::osc::*;

use types::Float;

pub trait Receiver {
    fn receive_and_send(&mut self, mpsc::Sender<ControlEvent>);
}

pub const CONCERT_A: Float = 440.0;

pub struct PitchConvert {
    table: Vec<Float>,
}
impl PitchConvert {
    pub fn new(tune_freq: Float) -> Self {
        PitchConvert {
            // see https://en.wikipedia.org/wiki/MIDI_Tuning_Standard
            table: (0..128)
                       .map(|key| {
                           let dist_concert_a = key as isize - 69;
                           let two: Float = 2.0;
                           two.powf(dist_concert_a as Float / 12.0) * tune_freq
                       })
                       .collect::<Vec<_>>(),
        }
    }

    pub fn key_to_hz(&self, key: u8) -> Float {
        if (key as usize) < self.table.len() {
            self.table[key as usize]
        } else {
            self.table[self.table.len() - 1]
        }
    }
}
