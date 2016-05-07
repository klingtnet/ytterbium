use types::Stereo;

mod env_gen;
mod wavetable;
mod voice;

pub use self::env_gen::{ADSR, ADSRState};
pub use self::wavetable::{Wavetable, WavetableOsc, generate_wavetables, Waveform};
pub use self::voice::{VoiceManager};

pub trait SignalSource {
    fn tick(&mut self) -> Stereo;
}
pub trait SignalLink {
    fn tick(&mut self, Stereo) -> Stereo;
}
pub trait SignalSink {
    fn tick(&mut self, Stereo);
}
pub trait SignalFlow {
    fn tick(&mut self);
}
