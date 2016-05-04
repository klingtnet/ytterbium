use std::default::Default;

use types::*;
use event::{ControlEvent, Controllable};

#[derive(PartialEq,Debug,Copy,Clone)]
pub enum ADSRState {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}
impl ADSRState {
    fn progress(&self) -> ADSRState {
        match *self {
            ADSRState::Attack => ADSRState::Decay,
            ADSRState::Decay => ADSRState::Sustain,
            ADSRState::Sustain => ADSRState::Release,
            ADSRState::Release | ADSRState::Off => ADSRState::Off,
        }
    }
}

pub struct ADSR {
    sample_rate: usize,
    attack: (Time, Float),
    decay: Time,
    sustain: Float,
    release: Time,
    state: ADSRState,
    ticks_left: usize,
    gain: Float,
    velocity: Float,
    level: Float,
    target_level: Float,
}
impl ADSR {
    pub fn new(sample_rate: usize) -> Self {
        let mut adsr = Self::default();
        adsr.sample_rate = sample_rate;
        adsr
    }

    pub fn tick(&mut self) -> Float {
        self.velocity *
        match self.state {
            ADSRState::Off => 0.0,
            ADSRState::Sustain => self.level,
            _ => {
                if self.ticks_left == 0 {
                    let next_state = self.state.progress();
                    self.state_change(next_state);
                }
                self.level = self.target_level * self.gain + (1.0 - self.gain) * self.level;
                self.ticks_left -= 1;
                self.level
            }
        }
    }

    fn state_change(&mut self, state: ADSRState) {
        if state == self.state {
            return;
        }

        self.state = state;
        match state {
            ADSRState::Attack => {
                let (time, level) = self.attack;
                self.ticks_left = (time * self.sample_rate as Time) as usize;
                self.gain = 4.0 / self.ticks_left as Float;
                self.target_level = level;
            }
            ADSRState::Decay => {
                let time = self.decay;
                self.ticks_left = (time * self.sample_rate as Time) as usize;
                self.gain = 4.0 / self.ticks_left as Float;
                self.target_level = self.sustain;
            }
            ADSRState::Release => {
                let time = self.release;
                self.ticks_left = (time * self.sample_rate as Time) as usize;
                self.gain = 8.0 / self.ticks_left as Float;
                self.target_level = 0.0
            }
            _ => {}
        }
    }
}
impl Controllable for ADSR {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            ControlEvent::NoteOn { velocity, .. } => {
                self.state_change(ADSRState::Attack);
                self.velocity = Float::from_db((1.0 - velocity) * -30.0);
            }
            ControlEvent::NoteOff { .. } => self.state_change(ADSRState::Release),
            ControlEvent::ADSR { attack, decay, sustain, release, .. } => {
                self.attack.0 = attack;
                self.decay = decay;
                self.sustain = sustain;
                self.release = release;
            }
            _ => (),
        }
    }
}
impl Default for ADSR {
    fn default() -> Self {
        ADSR {
            attack: (0.05, Float::from_db(-3.0)),
            decay: 0.25,
            sustain: Float::from_db(-12.0),
            release: 1.5,
            sample_rate: 48_000,
            state: ADSRState::Off,
            ticks_left: 0,
            gain: 0.0,
            level: 0.0,
            velocity: 0.0,
            target_level: 1.0,
        }
    }
}
