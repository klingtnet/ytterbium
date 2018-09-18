extern crate itertools;

use self::itertools::Zip;
use std::collections::{HashMap, VecDeque};

use dsp;
use dsp::env_gen::*;
use dsp::wavetable::*;
use event::{ControlEvent, Controllable};
use io::PitchConvert;
use std::rc::Rc;
use types::*;

use dsp::SignalSource;

const OSC_CNT: usize = 4;

pub struct Voice {
    fm_mod: Vec<Float>, // contains the mod indices
    levels: Vec<Float>, // oscillator levels
    pan: Vec<Stereo>,
    volume_envelopes: Vec<ADSR>,
    oscillators: Vec<WavetableOsc>,
}
impl Voice {
    fn new(
        sample_rate: usize,
        wavetables: Rc<HashMap<Waveform, Vec<Wavetable>>>,
        pitch_convert_handle: Rc<PitchConvert>,
    ) -> Self {
        let mut levels = Vec::with_capacity(OSC_CNT);
        let mut oscillators = Vec::with_capacity(OSC_CNT);
        let mut volume_envelopes = Vec::with_capacity(OSC_CNT);
        for idx in 0..OSC_CNT {
            levels.push(if idx == 0 { MINUS_THREE_DB } else { 0.0 });
            oscillators.push(WavetableOsc::with_id(
                format!("OSC{}", idx + 1),
                sample_rate,
                wavetables.clone(),
                pitch_convert_handle.clone(),
            ));
            volume_envelopes.push(ADSR::with_id(sample_rate, format!("ADSR-OSC{}", idx + 1)));
        }
        Voice {
            // use offset instead of nested vector
            fm_mod: vec![0.0; OSC_CNT * OSC_CNT],
            levels,
            pan: vec![Stereo(MINUS_THREE_DB, MINUS_THREE_DB); OSC_CNT],
            volume_envelopes,
            oscillators,
        }
    }
    fn running(&self) -> bool {
        self.volume_envelopes
            .iter()
            .all(|envelope| envelope.state() != ADSRState::Off)
    }
    fn tick(&mut self) -> Stereo {
        let mut samples = [0.0; OSC_CNT];
        let mut frame = Stereo::default();
        // tick each oscillator + apply env
        for (_idx, (sample, oscillator, envelope, level, pan)) in Zip::new((
            &mut samples,
            &mut self.oscillators,
            &mut self.volume_envelopes,
            &self.levels,
            &self.pan,
        )).take(OSC_CNT)
        .enumerate()
        {
            *sample = oscillator.tick() * envelope.tick();
            frame += Stereo(*sample, *sample) * *level * *pan;
        }
        for (idx, oscillator) in self.oscillators.iter_mut().enumerate() {
            let phase = Zip::new((&mut samples, self.fm_mod.iter().skip(idx * OSC_CNT)))
                .take(OSC_CNT)
                .fold(0.0, |acc, (sample, mod_index)| acc + *sample * mod_index);
            oscillator.set_phase(phase);
        }
        frame
    }
}
impl Controllable for Voice {
    fn handle(&mut self, msg: &ControlEvent) {
        match *msg {
            ControlEvent::Volume(ref volume) => {
                for (old_vol, new_vol) in self.levels.iter_mut().zip(volume.iter()) {
                    *old_vol = if *new_vol < -60.0 {
                        0.0
                    } else {
                        Float::from_db(*new_vol)
                    };
                }
            }
            ControlEvent::Pan(ref pan) => {
                for (old_pan, new_pan) in self.pan.iter_mut().zip(pan.iter()) {
                    *old_pan = if feq!(new_pan, 0.0) {
                        Stereo(MINUS_THREE_DB, MINUS_THREE_DB)
                    } else {
                        // use a quadratic panning
                        let pan_squared = new_pan * new_pan;
                        let scale = if new_pan.signum() < 0.0 {
                            Stereo(1.0 - MINUS_THREE_DB, MINUS_THREE_DB)
                        } else {
                            Stereo(MINUS_THREE_DB, 1.0 - MINUS_THREE_DB)
                        };
                        let delta = Stereo(-pan_squared, pan_squared) * scale * new_pan.signum();
                        Stereo(MINUS_THREE_DB, MINUS_THREE_DB) + delta
                    }
                }
            }
            ControlEvent::FM { ref id, ref levels } => {
                let offset = match id.as_ref() {
                    "OSC1" => 0,
                    "OSC2" => 1,
                    "OSC3" => 2,
                    "OSC4" => 3,
                    _ => self.fm_mod.len(), // offset is larger than the length of the vector -> no modification
                } * OSC_CNT;
                for (idx, (old_level, new_level)) in self
                    .fm_mod
                    .iter_mut()
                    .skip(offset)
                    .take(OSC_CNT)
                    .zip(levels.iter())
                    .enumerate()
                {
                    *old_level = if idx == offset {
                        *new_level * 0.1 // feedback modulation easily creates feedback
                    } else {
                        *new_level
                    };
                }
            }
            _ => {
                for osc in &mut self.oscillators {
                    osc.handle(msg);
                }
                for env in &mut self.volume_envelopes {
                    env.handle(msg);
                }
            }
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
            voices.push(Voice::new(
                sample_rate,
                wavetables.clone(),
                pitch_convert.clone(),
            ));
        }
        VoiceManager {
            voices,
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
