use dsp::{ControllableLink, SignalLink};
use event::ControlEvent;
use types::Stereo;

pub struct HardLimiter {}
impl SignalLink for HardLimiter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        let mut output = input;
        if input.0.abs() > 1.0 {
            output.0 = 1.0 * input.1.signum();
        }
        if input.1.abs() > 1.0 {
            output.1 = 1.0 * input.1.signum();
        }
        output
    }
}

pub struct SoftLimiter {}
impl ControllableLink for SoftLimiter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        // Padé approximation for tanh
        // http://www.musicdsp.org/showone.php?id=238
        match (input.0.abs() > 3.0, input.1.abs() > 3.0) {
            (false, false) => input * (input * input + 27.0) / (input * input * 9.0 + 27.0),
            (true, false) => Stereo(input.0.signum(), input.1),
            (false, true) => Stereo(input.0, input.1.signum()),
            _ => Stereo(input.0.signum(), input.1.signum()),
        }
    }
    fn handle(&mut self, _msg: &ControlEvent) {}
}
