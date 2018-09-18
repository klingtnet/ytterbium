use event::ControlEvent;
use types::Stereo;

mod dynamics;
mod env_gen;
mod filter;
mod flow;
mod voice;
mod wavetable;

pub use self::dynamics::{HardLimiter, SoftLimiter};
pub use self::env_gen::{ADSRState, ADSR};
pub use self::filter::{Filter, FilterType};
pub use self::flow::{BufferSink, Flow};
pub use self::voice::VoiceManager;
pub use self::wavetable::{generate_wavetables, Waveform, Wavetable, WavetableOsc};

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
