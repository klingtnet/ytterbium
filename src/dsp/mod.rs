use types::Stereo;
use event::ControlEvent;

mod env_gen;
mod wavetable;
mod voice;
mod flow;
mod dynamics;
mod filter;

pub use self::env_gen::{ADSR, ADSRState};
pub use self::wavetable::{Wavetable, WavetableOsc, generate_wavetables, Waveform};
pub use self::voice::VoiceManager;
pub use self::flow::{Flow, BufferSink};
pub use self::dynamics::{HardLimiter, SoftLimiter};
pub use self::filter::{Filter, FilterType};

pub trait SignalSource {
    fn tick(&mut self) -> Stereo;
}
pub trait SignalLink {
    fn tick(&mut self, Stereo) -> Stereo;
}
pub trait ControllableLink {
    fn tick(&mut self, Stereo) -> Stereo;
    fn handle(&mut self, msg: &ControlEvent);
}
pub trait SignalSink {
    fn tick(&mut self, Stereo);
}
pub trait SignalFlow {
    fn tick(&mut self);
}
