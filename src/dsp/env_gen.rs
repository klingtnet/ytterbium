use std::default::Default;

use event::{ControlEvent, Controllable};
use types::*;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ADSRState {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}
impl ADSRState {
    fn progress(self) -> ADSRState {
        match self {
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
    id: String,
}
impl ADSR {
    pub fn new(sample_rate: usize) -> Self {
        let mut adsr = Self::default();
        adsr.sample_rate = sample_rate;
        adsr
    }

    pub fn with_id<S: Into<String>>(sample_rate: usize, id: S) -> Self {
        let mut adsr = Self::new(sample_rate);
        adsr.id = id.into();
        adsr
    }

    pub fn tick(&mut self) -> Float {
        self.velocity * match self.state {
            ADSRState::Off => 0.0,
            ADSRState::Sustain => self.level,
            _ => {
                if self.ticks_left == 0 {
                    let next_state = self.state.progress();
                    self.state_change(next_state);
                    self.tick()
                } else {
                    self.level = self.target_level * self.gain + (1.0 - self.gain) * self.level;
                    self.ticks_left -= 1;
                    self.level
                }
            }
        }
    }

    pub fn state(&self) -> ADSRState {
        self.state
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
                // TODO: Retrigger option?
                // self.level = 0.0;
                // TODO: make velocity sensitivity controllable
                self.velocity = Float::from_db((1.0 - velocity) * -30.0);
            }
            ControlEvent::NoteOff { .. } => self.state_change(ADSRState::Release),
            ControlEvent::ADSR {
                ref id,
                attack,
                decay,
                sustain,
                release,
            } => {
                // check path
                // TODO: make sure that all values are non-zero!
                if *id == self.id {
                    self.attack.0 = attack;
                    self.decay = decay;
                    self.sustain = sustain;
                    self.release = release;
                }
            }
            _ => (),
        }
    }
}
impl Default for ADSR {
    fn default() -> Self {
        ADSR {
            attack: (0.02, MINUS_THREE_DB),
            decay: 0.2,
            sustain: Float::from_db(-12.0),
            release: 0.6,
            sample_rate: 48_000,
            state: ADSRState::Off,
            ticks_left: 0,
            gain: 0.0,
            level: 0.0,
            velocity: 0.0,
            target_level: 1.0,
            id: "".to_owned(),
        }
    }
}

const TEST_EPSILON: Float = 0.02; // -34dB

#[test]
fn test_state_change() {
    let sample_rate = 48_000;
    let mut adsr = ADSR::new(sample_rate);
    assert_eq!(adsr.state(), ADSRState::Off);
    // setup ADSR parameters
    adsr.handle(&ControlEvent::ADSR {
        id: "".to_owned(),
        attack: 0.1,
        decay: 0.3,
        sustain: Float::from_db(-16.0),
        release: 2.0,
    });
    // initialize envelope
    adsr.handle(&ControlEvent::NoteOn {
        key: 0,
        velocity: 1.0,
    });
    assert_eq!(adsr.state(), ADSRState::Attack);
    // the state change is active in the n+1 tick
    let mut ticks = (adsr.attack.0 * sample_rate as Time) as isize + 1;
    while ticks > 0 {
        adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Decay);
    ticks = (adsr.decay * sample_rate as Time) as isize + 1;
    while ticks > 0 {
        adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Sustain);
    assert_relative_eq!(adsr.tick(), adsr.sustain, epsilon = TEST_EPSILON);
    adsr.handle(&ControlEvent::NoteOff {
        key: 0,
        velocity: 0.0,
    });
    ticks = (adsr.release * sample_rate as Time) as isize + 1;
    while ticks > 0 {
        adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Off);
}

#[test]
fn test_short_envelope() {
    let sample_rate = 48_000;
    let mut adsr = ADSR::new(sample_rate);
    assert_eq!(adsr.state(), ADSRState::Off);
    // setup ADSR parameters
    adsr.handle(&ControlEvent::ADSR {
        id: "".to_owned(),
        attack: 0.01,
        decay: 0.01,
        sustain: Float::from_db(-16.0),
        release: 0.01,
    });
    // initialize envelope
    adsr.handle(&ControlEvent::NoteOn {
        key: 0,
        velocity: 1.0,
    });
    assert_eq!(adsr.state(), ADSRState::Attack);
    // the state change is active in the n+1 tick
    let mut ticks = (adsr.attack.0 * sample_rate as Time) as isize + 1;
    let mut level = 0.0;
    while ticks > 0 {
        level = adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Decay);
    assert_relative_eq!(level, Float::from_db(-3.0), epsilon = TEST_EPSILON);
    ticks = (adsr.decay * sample_rate as Time) as isize + 1;
    while ticks > 0 {
        level = adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Sustain);
    assert_relative_eq!(level, adsr.sustain, epsilon = TEST_EPSILON);
    adsr.handle(&ControlEvent::NoteOff {
        key: 0,
        velocity: 0.0,
    });
    ticks = (adsr.release * sample_rate as Time) as isize;
    while ticks > 0 {
        level = adsr.tick();
        assert!(level > 0.0);
        ticks -= 1;
    }
    assert_relative_eq!(level, 0.0, epsilon = TEST_EPSILON);
    level = adsr.tick();
    assert_eq!(adsr.state(), ADSRState::Off);
    assert_relative_eq!(level, 0.0);
}

#[test]
fn test_long_envelope() {
    let sample_rate = 48_000;
    let mut adsr = ADSR::new(sample_rate);
    assert_eq!(adsr.state(), ADSRState::Off);
    // setup ADSR parameters
    adsr.handle(&ControlEvent::ADSR {
        id: "".to_owned(),
        attack: 10.0,
        decay: 20.0,
        sustain: Float::from_db(-24.0),
        release: 30.0,
    });
    // initialize envelope
    adsr.handle(&ControlEvent::NoteOn {
        key: 0,
        velocity: 1.0,
    });
    assert_eq!(adsr.state(), ADSRState::Attack);
    // the state change is active in the n+1 tick
    let mut ticks = (adsr.attack.0 * sample_rate as Time) as isize + 1;
    let mut level = 0.0;
    while ticks > 0 {
        level = adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Decay);
    assert_relative_eq!(level, Float::from_db(-3.0), epsilon = TEST_EPSILON);
    ticks = (adsr.decay * sample_rate as Time) as isize + 1;
    while ticks > 0 {
        level = adsr.tick();
        ticks -= 1;
    }
    assert_eq!(adsr.state(), ADSRState::Sustain);
    assert_relative_eq!(level, adsr.sustain, epsilon = TEST_EPSILON);
    adsr.handle(&ControlEvent::NoteOff {
        key: 0,
        velocity: 0.0,
    });
    ticks = (adsr.release * sample_rate as Time) as isize;
    while ticks > 0 {
        level = adsr.tick();
        assert!(level > 0.0);
        ticks -= 1;
    }
    assert_relative_eq!(level, 0.0, epsilon = TEST_EPSILON);
    level = adsr.tick();
    assert_eq!(adsr.state(), ADSRState::Off);
    assert_relative_eq!(level, 0.0);
}
