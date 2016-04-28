extern crate rustfft;
extern crate num;
extern crate rand;

use types::*;
use event::{ControlEvent, Controllable};

const TABLE_SIZE: usize = 512;

pub struct WavetableOsc {
    phaseIncr: Float,
    sample_rate: usize,
    freq: Float,
    phase: Float,
    phasor: Float,
    table: Vec<Float>,
    // tables: Vec<Vec<Float>>,
}

#[derive(PartialEq,Eq,Hash,Debug)]
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
            phaseIncr: freq * TABLE_SIZE as Float / sample_rate as Float,
            sample_rate: sample_rate,
            freq: freq,
            phase: 0.0,
            phasor: 0.0,
            table: table,
        }
    }

    pub fn set_freq(&mut self, freq: Float) {
        self.freq = freq;
        self.phaseIncr = freq * TABLE_SIZE as Float / self.sample_rate as Float;
        self.phasor = self.phase;
    }

    pub fn tick(&mut self) -> Float {
        let (i,j) = (self.phasor.floor() as usize % TABLE_SIZE, self.phasor.ceil() as usize() % TABLE_SIZE);
        let sample = self.table[i] + (self.table[j] - self.table[i]) * (self.phasor - i as Float);
        self.phasor = if TABLE_SIZE as Float - self.phasor < self.phaseIncr {
            0.0
        } else {
            self.phasor + self.phaseIncr
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
