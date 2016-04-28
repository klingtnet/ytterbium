extern crate rustfft;
extern crate num;
extern crate rand;

use types::*;
use event::{ControlEvent, Controllable};

const TABLE_SIZE: usize = 512;

pub struct WavetableOsc {
    w: Float,
    sample_rate: usize,
    freq: Float,
    phase: Float,
    pos: Float,
    table: Vec<Float>,
    // tables: Vec<Vec<Float>>,
}
enum Waveform {
    Sine,
    Saw,
    Square,
    Random,
    Noise,
}

impl WavetableOsc {
    pub fn new(freq: Float, sample_rate: usize) -> Self {
        let mut spectrum: Vec<_> = vec![num::Complex { re: 0.0, im: 0.0 }; TABLE_SIZE];
        // sine
        spectrum[1] = num::Complex { re: 0.0, im: 1.0 };
        let mut fft = rustfft::FFT::new(TABLE_SIZE, false);
        let mut signal = spectrum.clone();
        fft.process(&spectrum, &mut signal);
        let table = signal.iter().map(|c| c.re as Float).collect::<Vec<Float>>();
        WavetableOsc {
            w: freq * TABLE_SIZE as Float / sample_rate as Float,
            sample_rate: sample_rate,
            freq: freq,
            phase: 0.0,
            pos: 0.0,
            table: table,
        }
    }

    pub fn set_freq(&mut self, freq: Float) {
        self.freq = freq;
        self.w = freq * TABLE_SIZE as Float / self.sample_rate as Float;
        self.pos = self.phase;
    }

    pub fn tick(&mut self) -> Float {
        let (i,j) = (self.pos.floor() as usize % TABLE_SIZE, self.pos.ceil() as usize() % TABLE_SIZE);
        let sample = self.table[i] + (self.table[j] - self.table[i]) * (self.pos - i as Float);
        self.pos = if TABLE_SIZE as Float - self.pos < self.w {
            0.0
        } else {
            self.pos + self.w
        };
        sample
    }
}

impl Controllable for WavetableOsc {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            ControlEvent::NoteOn { freq, .. } => {
                self.set_freq(freq);
            }
            _ => (),
        }
    }
}
