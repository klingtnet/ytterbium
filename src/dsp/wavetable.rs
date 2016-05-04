extern crate rustfft;
extern crate num;
extern crate rand;

use std::collections::HashMap;
use std::sync::Arc;
use self::num::{Complex, Zero};
use self::rustfft::FFT;

use types::*;
use io::PitchConvert;
use event::{ControlEvent, Controllable};
use dsp::ADSR;

const OVERSAMPLING: usize = 2;
const INVERSE: bool = true;
const SCALE: bool = true;

/// Stores a period of a band-limited signal together with
/// the maximum frequency before aliasing occurs.
pub struct Wavetable {
    /// The band-limited signal
    table: Vec<Float>,
    /// The maximum phase increment (frequency) that is handled by this table.
    /// The oscillators frequency is determined by the amount of phase increment
    /// in each sample tick.
    // TODO: rename to `max_frequency`?
    max_phase_incr: Float,
}
impl Wavetable {
    /// Returns a linear interpolated sample from the wavetable for the given phase.
    /// The phase is mapped to a table index.
    fn sample(&self, phasor: Float) -> Float {
        let table_len = self.table.len();
        let idx = phasor * table_len as Float;
        // linear interpolation
        let (i, j) = (idx.floor() as usize % table_len, idx.ceil() as usize % table_len);
        self.table[i] + (self.table[j] - self.table[i]) * (idx - i as Float)
    }
}

/// Implemented waveforms.
#[derive(PartialEq,Eq,Hash,Debug,Copy,Clone)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Tri,
    SharpTri,
    Random,
}

/// Normalizes the signal into a range of `[-1.0, 1.0]`.
macro_rules! scale {
    ($flag:expr, $signal:expr) => {
        if $flag {
            let scale = $signal.iter().fold(0.0, |acc: Float, val| acc.max(val.re.abs()));
            for sample in &mut $signal {
                sample.re = sample.re * scale.recip();
            }
        }
    };
}

/// Builds wavetables for each waveform and returns a `HashMap` containing them.
pub fn generate_wavetables(fundamental_freq: Float,
                           sample_rate: usize)
                           -> HashMap<Waveform, Vec<Wavetable>> {
    let mut tables: HashMap<Waveform, Vec<Wavetable>> = HashMap::new();
    tables.insert(Waveform::Sine,
                  build_wavetables(Waveform::Sine, fundamental_freq, sample_rate));
    tables.insert(Waveform::Saw,
                  build_wavetables(Waveform::Saw, fundamental_freq, sample_rate));
    tables.insert(Waveform::Square,
                  build_wavetables(Waveform::Square, fundamental_freq, sample_rate));
    tables.insert(Waveform::Tri,
                  build_wavetables(Waveform::Tri, fundamental_freq, sample_rate));
    tables.insert(Waveform::SharpTri,
                  build_wavetables(Waveform::SharpTri, fundamental_freq, sample_rate));
    tables.insert(Waveform::Random,
                  build_wavetables(Waveform::Random, fundamental_freq, sample_rate));
    tables
}

/// Builds the band-limited wavetables for the given waveform, fundamental frequency and
/// sample rate.
fn build_wavetables(waveform: Waveform,
                    fundamental_freq: Float,
                    sample_rate: usize)
                    -> Vec<Wavetable> {
    let min_table_size = 64;
    let mut phase_incr = fundamental_freq * 2.0 / sample_rate as Float;
    let (mut harmonics, mut table_size) = match waveform {
        Waveform::Sine => (1, 4096),
        _ => {
            let harmonics = sample_rate / (2 * fundamental_freq as usize);
            let table_size = harmonics.next_power_of_two() * 2 * OVERSAMPLING;
            (harmonics, table_size)
        }
    };
    let mut tables: Vec<Wavetable> = Vec::with_capacity((harmonics as Float).log2() as usize);
    // use sine if only 1 harmonic is left, otherwise the last table for waveforms with
    // only odd harmonics would be empty!
    while harmonics > 0 {
        let mut fft = FFT::new(table_size, INVERSE);
        let mut spectrum = vec![num::Complex::zero(); table_size];
        let mut signal = spectrum.clone();

        generate_spectrum(waveform, harmonics, &mut spectrum);

        fft.process(&spectrum, &mut signal);
        scale!(SCALE, signal);

        tables.push(Wavetable {
            table: signal.iter().map(|c| c.re).collect::<Vec<_>>(),
            max_phase_incr: phase_incr,
        });

        harmonics >>= 1; // half the number of harmonics
        phase_incr *= 2.0;
        let next_table_size = harmonics.next_power_of_two() * 2 * OVERSAMPLING;
        table_size = ::std::cmp::max(min_table_size, next_table_size);
    }
    tables
}

/// Generates a band-limited spectrum with given number of harmonics for the given waveform.
fn generate_spectrum(waveform: Waveform, harmonics: usize, spectrum: &mut Vec<Complex<Float>>) {
    let table_size = spectrum.len();
    if harmonics == 1 {
        // use a pure sine
        spectrum[1] = Complex {
            re: 1.0,
            im: -1.0,
        };
        spectrum[table_size - 1] = -spectrum[1];
        return;
    }
    match waveform {
        Waveform::Saw => {
            for i in 1..harmonics {
                let magnitude = (i as Float).recip();
                spectrum[i] = Complex {
                    re: 1.0,
                    im: -1.0 * magnitude,
                };
                spectrum[table_size - i] = -spectrum[i];
            }
        }
        Waveform::Square => {
            for i in (1..harmonics).filter(|i| i % 2 == 1) {
                let magnitude = (i as Float).recip();
                spectrum[i] = Complex {
                    re: 1.0,
                    im: -1.0 * magnitude,
                };
                spectrum[table_size - i] = -spectrum[i];
            }
        }
        Waveform::Tri => {
            for i in (1..harmonics).filter(|i| i % 2 == 1) {
                let sign = if i % 4 == 1 {
                    1.0
                } else {
                    -1.0
                };
                let magnitude = ((i * i) as Float).recip();
                spectrum[i] = Complex {
                    re: 1.0,
                    im: -1.0 * magnitude * sign,
                };
                spectrum[table_size - i] = -spectrum[i];
            }
        }
        Waveform::SharpTri => {
            for i in (1..harmonics).filter(|i| i % 2 == 1) {
                let sign = if i % 4 == 1 {
                    1.0
                } else {
                    -1.0
                };
                let magnitude = (i as Float).recip();
                spectrum[i] = Complex {
                    re: 1.0,
                    im: -1.0 * magnitude * sign,
                };
                spectrum[table_size - i] = -spectrum[i];
            }
        }
        Waveform::Random => {
            for i in 1..harmonics {
                let magnitude = (i as Float).recip();
                spectrum[i] = Complex {
                    re: 1.0,
                    im: -rand::random::<Float>() * magnitude,
                };
                spectrum[table_size - i] = -spectrum[i];
            }
        }
        _ => {}
    }
}

/// A band-limited wavetable oscillator.
pub struct WavetableOsc<'a> {
    phase_incr: Float,
    sample_rate: usize,
    key: u8,
    detune_hz: Float,
    phase: Float,
    phasor: Float,
    transpose: i32, // transposition in octaves
    volume: Float,
    volume_envelope: ADSR,
    pan: Stereo,
    waveform: Waveform,
    id: String,
    pitch_convert: Arc<PitchConvert>,
    tables: &'a HashMap<Waveform, Vec<Wavetable>>,
}
impl<'a> WavetableOsc<'a> {
    /// Constructs a wavetable oscillator for the given sample rate.
    pub fn new<S: Into<String>>(id: S,
                                sample_rate: usize,
                                wavetables: &'a HashMap<Waveform, Vec<Wavetable>>,
                                pitch_convert: Arc<PitchConvert>)
                                -> Self {
        WavetableOsc {
            phase_incr: 0.0,
            sample_rate: sample_rate,
            key: 0,
            detune_hz: 0.0, // Hz
            phase: 0.0,
            phasor: 0.0,
            transpose: 0,
            volume: MINUS_SIX_DB,
            volume_envelope: ADSR::new(sample_rate),
            pan: Stereo(MINUS_THREE_DB, MINUS_THREE_DB),
            waveform: Waveform::Saw,
            id: id.into(),
            pitch_convert: pitch_convert,
            tables: wavetables,
        }
    }

    /// Sets the oscillators frequency in Hz.
    pub fn set_freq(&mut self, freq: Float) {
        self.phase_incr = (freq * Float::powi(2.0, self.transpose)) / self.sample_rate as Float;
    }

    /// Sets the waveform to use.
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn set_volume(&mut self, volume: Float) {
        let db = Float::from_db(volume);
        self.volume = if db < -60.0 {
            0.0
        } else {
            db
        };
    }


    pub fn set_phase(&mut self, phase: Float) {
        self.phase = phase;
    }

    /// Returns the next sample from the oscillator.
    pub fn tick(&mut self) -> Stereo {
        let env = self.volume_envelope.tick();
        let sample = self.sample(self.phasor);
        self.phasor = self.phasor + self.phase_incr;
        if self.phasor > 1.0 {
            self.phasor = self.phasor.fract(); // fractional part
        }
        Stereo(sample, sample) * self.volume * self.pan * env
    }

    /// Returns the sample from the appropriate band-limited wavetable.
    fn sample(&self, phasor: Float) -> Float {
        let wavetables = self.tables.get(&self.waveform).unwrap();
        let mut idx = 0;
        for i in 0..wavetables.len() {
            idx = i;
            if wavetables[idx].max_phase_incr > self.phase_incr {
                break;
            }
        }

        let wavetable = &wavetables[idx];
        wavetable.sample(phasor)
    }
}

impl<'a> Controllable for WavetableOsc<'a> {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            ControlEvent::NoteOn { key, .. } => {
                self.key = key;
                let freq = self.pitch_convert.key_to_hz(key) + self.detune_hz;
                self.set_freq(freq);
                self.volume_envelope.handle(msg);
            }
            ControlEvent::NoteOff { .. } => {
                self.volume_envelope.handle(msg);
            }
            ControlEvent::Waveform { ref id, waveform } => {
                if *id == self.id {
                    self.set_waveform(waveform);
                }
            }
            ControlEvent::Volume { ref id, volume } => {
                if *id == self.id {
                    self.set_volume(volume);
                }
            }
            ControlEvent::Phase { ref id, phase } => {
                if *id == self.id {
                    self.set_phase(phase)
                }
            }
            ControlEvent::Transpose { ref id, transpose } => {
                if *id == self.id {
                    self.transpose = transpose
                }
            }
            ControlEvent::Detune { ref id, detune } => {
                if *id == self.id {
                    let (low, current, high) = (self.pitch_convert.key_to_hz(self.key - 1),
                                                self.pitch_convert.key_to_hz(self.key),
                                                self.pitch_convert.key_to_hz(self.key + 1));
                    // linear approximation of cents
                    let cent = if detune < 0 {
                        (low - current)
                    } else {
                        (high - current)
                    } / 100.0;
                    self.detune_hz = (detune as Float) * cent;
                    let detuned_freq = current + self.detune_hz;
                    self.set_freq(detuned_freq);
                }
            }
            ControlEvent::Pan { ref id, pan } => {
                if *id == self.id {
                    if feq!(pan, 0.0) {
                        self.pan = Stereo(MINUS_THREE_DB, MINUS_THREE_DB);
                    } else {
                        // use a quadratic panning
                        let pan_squared = pan * pan;
                        let scale = if pan.signum() < 0.0 {
                            Stereo((1.0 - MINUS_THREE_DB), MINUS_THREE_DB)
                            } else {
                            Stereo(MINUS_THREE_DB, (1.0 - MINUS_THREE_DB))
                        };
                        let delta = Stereo(-pan_squared, pan_squared) * scale * pan.signum();
                        self.pan = Stereo(MINUS_THREE_DB, MINUS_THREE_DB) + delta;
                    }
                }
            }
            ControlEvent::ADSR { ref id, .. } => {
                if *id == self.id {
                    self.volume_envelope.handle(msg);
                }
            }
            _ => (),
        }
    }
}
