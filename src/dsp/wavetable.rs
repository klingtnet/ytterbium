extern crate rustfft;
extern crate num;
extern crate rand;
extern crate bincode;

use std::collections::HashMap;
use std::rc::Rc;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use self::num::{Complex, Zero};
use self::rustfft::FFT;
use self::bincode::rustc_serialize::{decode_from, encode_into};
use self::bincode::SizeLimit;

use types::*;
use io::PitchConvert;
use event::{ControlEvent, Controllable};

const OVERSAMPLING: usize = 2;
const INVERSE: bool = true;
const SCALE: bool = true;

/// Stores a period of a band-limited signal together with
/// the maximum frequency before aliasing occurs.
#[derive(RustcDecodable, RustcEncodable)]
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
        let idx = if phasor < 0.0 {
            phasor + 1.0
        } else {
            phasor
        } * table_len as Float;
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
                sample.re *= scale.recip();
            }
        }
    };
}

/// Builds wavetables for each waveform and returns a `HashMap` containing them.
pub fn generate_wavetables(fundamental_freq: Float,
                           sample_rate: usize)
                           -> HashMap<Waveform, Vec<Wavetable>> {
    let mut tables: HashMap<Waveform, Vec<Wavetable>> = HashMap::new();
    for waveform in &[Waveform::Saw,
                      Waveform::Sine,
                      Waveform::Square,
                      Waveform::Tri,
                      Waveform::SharpTri,
                      Waveform::Random] {
        let filename = format!("ytterbium-{}-wavetable-{:?}.bin",
                               env!("CARGO_PKG_VERSION"),
                               waveform);
        let band_limited_table = {
            if let Ok(file) = File::open(&filename) {
                let mut reader = BufReader::new(file);
                decode_from(&mut reader, SizeLimit::Infinite)
                    .expect(&format!("could not decode wavetable: {}", filename))
            } else {
                let band_limited_table = build_wavetables(*waveform, fundamental_freq, sample_rate);
                let file = File::create(&filename)
                    .expect(&format!("could not create file for wavetable: {}", filename));
                let mut writer = BufWriter::new(file);
                encode_into(&band_limited_table, &mut writer, SizeLimit::Infinite)
                    .expect(&format!("could not encode wavetable: {}", filename));
                band_limited_table
            }
        };
        tables.insert(*waveform, band_limited_table);
    }
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
            let harmonics = sample_rate / (2 * (2.0 * fundamental_freq) as usize);
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
pub struct WavetableOsc {
    phase_incr: Float,
    sample_rate: usize,
    key: u8,
    detune_hz: Float,
    phase: Float,
    phase_changed: bool,
    phasor: Float,
    transpose: i32, // transposition in octaves
    last_sample: Float,
    waveform: Waveform,
    id: String,
    pitch_convert: Rc<PitchConvert>,
    tables: Rc<HashMap<Waveform, Vec<Wavetable>>>,
}
impl WavetableOsc {
    /// Constructs a wavetable oscillator for the given sample rate.
    pub fn new(sample_rate: usize,
               wavetables: Rc<HashMap<Waveform, Vec<Wavetable>>>,
               pitch_convert: Rc<PitchConvert>)
               -> Self {
        WavetableOsc {
            phase_incr: 0.0,
            sample_rate: sample_rate,
            key: 0,
            detune_hz: 0.0, // Hz
            phase: 0.0,
            phase_changed: false,
            phasor: 0.0,
            transpose: 0,
            last_sample: 0.0,
            waveform: Waveform::Sine,
            id: "".to_owned(),
            pitch_convert: pitch_convert,
            tables: wavetables,
        }
    }

    pub fn with_id<S: Into<String>>(id: S,
                                    sample_rate: usize,
                                    wavetables: Rc<HashMap<Waveform, Vec<Wavetable>>>,
                                    pitch_convert: Rc<PitchConvert>)
                                    -> Self {
        let mut osc = WavetableOsc::new(sample_rate, wavetables, pitch_convert);
        osc.set_id(id);
        osc
    }

    /// Sets the oscillators frequency in Hz.
    pub fn set_freq(&mut self, freq: Float) {
        self.phase_incr = (freq * Float::powi(2.0, self.transpose)) / self.sample_rate as Float;
    }

    /// Sets the waveform to use.
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn set_phase(&mut self, phase: Float) {
        const PHASE_DELTA: Float = 0.01;
        if (self.phase - phase).abs() > PHASE_DELTA {
            self.phase_changed = true;
        }
        self.phase = phase;
    }

    pub fn set_id<S: Into<String>>(&mut self, id: S) {
        self.id = id.into();
    }

    /// Returns the next sample from the oscillator.
    pub fn tick(&mut self) -> Float {
        let phasor = (self.phasor + self.phase).fract();
        let mut sample = self.sample(phasor);
        if self.phase_changed {
            sample = (self.last_sample + sample) / 2.0;
            self.phase_changed = false;
        }
        self.phasor += self.phase_incr;
        self.last_sample = sample;
        sample
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

    /// Resets the state of the oscillator.
    fn reset(&mut self) {
        self.phase = 0.0;
        self.phasor = 0.0;
        self.last_sample = 0.0;
    }
}

impl Controllable for WavetableOsc {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            ControlEvent::NoteOn { key, .. } => {
                self.key = key;
                let freq = self.pitch_convert.key_to_hz(key) + self.detune_hz;
                self.set_freq(freq);
            }
            ControlEvent::Waveform { ref id, waveform } => {
                if *id == self.id {
                    self.set_waveform(waveform);
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
            _ => (),
        }
    }
}

#[test]
fn test_wavetable_sweep() {
    extern crate hound;
    const SAMPLE_RATE: usize = 48_000;
    const LOW_FREQ: Float = 20.0;
    let wavetables = Rc::new(generate_wavetables(LOW_FREQ, SAMPLE_RATE));
    let pitch_convert = Rc::new(PitchConvert::default());
    let mut osc = WavetableOsc::new(SAMPLE_RATE, wavetables, pitch_convert);

    let wave_spec = hound::WavSpec {
        channels: 1,
        sample_format: hound::SampleFormat::Int,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 32,
    };
    for waveform in &[Waveform::Sine,
                      Waveform::Saw,
                      Waveform::Square,
                      Waveform::Tri,
                      Waveform::SharpTri,
                      Waveform::Random] {
        let filename = format!("ytterbium-{}-{:?}-sweep.wav",
                               env!("CARGO_PKG_VERSION"),
                               waveform);
        // An existing file will be overwritten.
        osc.reset();
        let mut writer = hound::WavWriter::create(filename, wave_spec).unwrap();
        let scale = ::std::i32::MAX as Float;
        osc.set_waveform(*waveform);
        let mut freq = LOW_FREQ;
        let num_samples = SAMPLE_RATE * 10;
        let multiplier = 1.0 + ((LOW_FREQ * 1000.0).ln() - (LOW_FREQ).ln()) / num_samples as Float;
        for _ in 0..num_samples {
            osc.set_freq(freq);
            let sample = osc.tick() * scale;
            writer.write_sample(sample as i32).unwrap();
            freq *= multiplier;
        }
        writer.finalize().unwrap();
    }
}

// test negative phase values
#[test]
fn test_wavetable_phase() {
    const SAMPLE_RATE: usize = 48_000;
    const LOW_FREQ: Float = 20.0;
    const EPSILON: f64 = 0.0001;
    let wavetables = Rc::new(generate_wavetables(LOW_FREQ, SAMPLE_RATE));
    let pitch_convert = Rc::new(PitchConvert::default());
    let mut osc = WavetableOsc::new(SAMPLE_RATE, wavetables, pitch_convert);

    for freq in &[1.0, 1000.0, ((SAMPLE_RATE >> 1) - 1) as Float] {
        osc.reset();
        osc.set_freq(*freq);
        let num_samples = SAMPLE_RATE / *freq as usize; // one period

        let mut total_error = 0.0;
        let phase_incr = (2.0 * PI * freq) / SAMPLE_RATE as Float; // for reference sine

        for idx in 0..num_samples {
            let sample = osc.tick();
            let sine = Float::sin(phase_incr * idx as Float);
            let error = sine - sample;
            total_error += error * error; // squared error
        }
        assert_relative_eq!(total_error, 0.0, epsilon = EPSILON);

        // +90 degree: sin->cos
        osc.reset();
        total_error = 0.0;
        osc.set_phase(0.25);

        for idx in 0..num_samples {
            let sample = osc.tick();
            let cosine = Float::cos(phase_incr * idx as Float);
            let error = cosine - sample;
            total_error += error * error; // squared error
        }
        assert_relative_eq!(total_error, 0.0, epsilon = EPSILON);

        // 180 degree
        osc.reset();
        total_error = 0.0;
        osc.set_phase(0.5);

        for idx in 0..num_samples {
            let sample = osc.tick();
            let sine = Float::sin(phase_incr * idx as Float + PI);
            let error = sine - sample;
            total_error += error * error; // squared error
        }
        assert_relative_eq!(total_error, 0.0, epsilon = EPSILON);

        // -90 degree
        osc.reset();
        total_error = 0.0;
        osc.set_phase(-0.5);

        for idx in 0..num_samples {
            let sample = osc.tick();
            let sine = Float::sin(phase_incr * idx as Float - PI);
            let error = sine - sample;
            total_error += error * error; // squared error
        }
        assert_relative_eq!(total_error, 0.0, epsilon = EPSILON);
    }
}

#[test]
fn test_wavetable_fm() {
    extern crate hound;
    const SAMPLE_RATE: usize = 48_000;
    const LOW_FREQ: Float = 20.0;
    let wavetables = Rc::new(generate_wavetables(LOW_FREQ, SAMPLE_RATE));
    let pitch_convert = Rc::new(PitchConvert::default());
    let mut carrier = WavetableOsc::new(SAMPLE_RATE, wavetables.clone(), pitch_convert.clone());
    let mut modulator = WavetableOsc::new(SAMPLE_RATE, wavetables.clone(), pitch_convert.clone());
    let num_samples = SAMPLE_RATE * 10;

    let wave_spec = hound::WavSpec {
        channels: 1,
        sample_format: hound::SampleFormat::Int,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 32,
    };
    let scale = ::std::i32::MAX as Float;
    let filename = format!("ytterbium-{}-fm.wav", env!("CARGO_PKG_VERSION"));

    let carrier_freq = 440.0;
    carrier.set_freq(carrier_freq);

    let mut modulator_freq = carrier_freq / 8.0;
    let freq_multiplier =
        1.0 + ((carrier_freq * 8.0).ln() - (modulator_freq).ln()) / num_samples as Float;
    modulator.set_freq(modulator_freq);

    let mut mod_index: Float = 0.01;
    let multiplier = 1.0 + ((mod_index * 1000.0).ln() - (mod_index).ln()) / num_samples as Float;

    let mut writer = hound::WavWriter::create(filename, wave_spec).unwrap();
    for idx in 0..num_samples {
        let mod_sample = modulator.tick();
        let sample = carrier.tick();
        carrier.set_phase(mod_sample * mod_index);
        writer.write_sample((sample * scale) as i32).unwrap();
        mod_index *= multiplier;
        modulator_freq *= freq_multiplier;
        modulator.set_freq(modulator_freq);
    }
    writer.finalize().unwrap();
}
