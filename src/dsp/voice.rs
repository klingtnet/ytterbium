use std::collections::{VecDeque, HashMap};

use std::rc::Rc;
use types::*;
use dsp;
use dsp::env_gen::*;
use dsp::wavetable::*;
use io::PitchConvert;
use event::{ControlEvent, Controllable};

use dsp::SignalSource;

const OSC_CNT: usize = 4;

pub struct Voice {
    levels: [Float; OSC_CNT],
    volume_envelopes: [ADSR; OSC_CNT],
    oscillators: [WavetableOsc; OSC_CNT],
}
impl Voice {
    fn new(sample_rate: usize,
           wavetables: Rc<HashMap<Waveform, Vec<Wavetable>>>,
           pitch_convert_handle: Rc<PitchConvert>)
           -> Self {
        Voice {
            levels: [1.0, 0.0, 0.0, 0.0],
            volume_envelopes: [ADSR::new(sample_rate),
                               ADSR::new(sample_rate),
                               ADSR::new(sample_rate),
                               ADSR::new(sample_rate)],
            oscillators: [WavetableOsc::new("OSC1",
                                            sample_rate,
                                            wavetables.clone(),
                                            pitch_convert_handle.clone()),
                          WavetableOsc::new("OSC2",
                                            sample_rate,
                                            wavetables.clone(),
                                            pitch_convert_handle.clone()),
                          WavetableOsc::new("OSC3",
                                            sample_rate,
                                            wavetables.clone(),
                                            pitch_convert_handle.clone()),
                          WavetableOsc::new("OSC4",
                                            sample_rate,
                                            wavetables.clone(),
                                            pitch_convert_handle.clone())],
        }
    }
    fn running(&self) -> bool {
        self.volume_envelopes.iter().all(|envelope| envelope.state() != ADSRState::Off)
    }
    fn tick(&mut self) -> Stereo {
        let mut frame = Stereo::default();
        for i in 0..OSC_CNT {
            frame += self.oscillators[i].tick() * self.volume_envelopes[i].tick() * self.levels[i];
        }
        frame
    }
}
impl Controllable for Voice {
    fn handle(&mut self, msg: &ControlEvent) {
        if let ControlEvent::OscMixer { levels } = *msg {
            for (i, level) in levels.iter().enumerate() {
                self.levels[i] = *level;
            }
        }
        for i in 0..OSC_CNT {
            self.volume_envelopes[i].handle(msg);
            self.oscillators[i].handle(msg);
        }
    }
}

pub struct VoiceManager {
    voices: Vec<Voice>,
    note_queue: VecDeque<(u8, usize)>,
}
impl VoiceManager {
    pub fn new(max_voices: usize, sample_rate: usize) -> Self {
        let wavetables = Rc::new(dsp::generate_wavetables(20.0, sample_rate));
        let pitch_convert = Rc::new(PitchConvert::default());
        let mut voices = Vec::with_capacity(max_voices);
        for _ in 0..max_voices {
            voices.push(Voice::new(sample_rate, wavetables.clone(), pitch_convert.clone()));
        }
        VoiceManager {
            voices: voices,
            note_queue: VecDeque::with_capacity(max_voices),
        }
    }

    fn free_voice(&self) -> Option<usize> {
        for (idx, voice) in self.voices.iter().enumerate() {
            if !voice.running() {
                return Some(idx);
            }
        }
        None
    }
}
impl SignalSource for VoiceManager {
    fn tick(&mut self) -> Stereo {
        let mut out = Stereo::default();
        for voice in &mut self.voices {
            if voice.running() {
                out += voice.tick()
            }
        }
        out
    }
}
impl Controllable for VoiceManager {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            ControlEvent::NoteOn { key, .. } => {
                if let Some(idx) = self.free_voice() {
                    self.note_queue.push_back((key, idx));
                    self.voices[idx].handle(msg)
                } else {
                    let (_, old_idx) = self.note_queue.pop_front().unwrap_or((0, 0));
                    self.voices[old_idx].handle(msg);
                    self.note_queue.push_back((key, old_idx))
                }
            }
            ControlEvent::NoteOff { key, .. } => {
                for &(played_key, idx) in &self.note_queue {
                    if played_key == key {
                        self.voices[idx].handle(msg)
                    }
                }
            }
            _ => {
                for voice in &mut self.voices {
                    voice.handle(msg)
                }
            }
        }
    }
}
