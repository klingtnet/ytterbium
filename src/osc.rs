use std::f64;
use std::f64::consts::PI as PI64;

pub trait Osc {
    fn new(usize, f64, f64) -> Self;
    fn tick(&mut self) -> f64;
    fn freq(&mut self, f64);
}

// angular frequency
fn pulsatance(fs: usize, f: f64) -> f64 {
    2.0 * f * PI64 / fs as f64
}

/// A simple oscillator using the sine function from the stl
pub struct Simple {
    w: f64,
    fs: usize,
    phase: f64,
    // position in unit-circle
    pos: f64,
}
impl Osc for Simple {
    fn new(fs: usize, f: f64, phase: f64) -> Self {
        Simple {
            w: pulsatance(fs, f),
            fs: fs,
            phase: phase,
            pos: 0.0,
        }
    }
    fn freq(&mut self, f: f64) {
        self.w = pulsatance(self.fs, f);
    }
    fn tick(&mut self) -> f64 {
        self.pos += self.w;
        f64::sin(self.pos)
    }
}

pub struct PhaseAccu {
    w: f64,
    fs: usize,
    phase: f64,
    // position in unit-circle
    a: f64,
    z: [f64; 2],
}
impl Osc for PhaseAccu {
    fn new(fs: usize, f: f64, phase: f64) -> Self {
        let w = pulsatance(fs, f);
        PhaseAccu {
            w: w,
            fs: fs,
            phase: phase,
            a: 2.0 * f64::cos(w),
            z: [f64::sin(phase), f64::sin(phase + w)],
        }
    }
    fn freq(&mut self, f: f64) {
        self.w = pulsatance(self.fs, f);
        self.a = 2.0 * f64::cos(self.w);
    }
    fn tick(&mut self) -> f64 {
        let last = self.z[1];
        self.z[1] = self.a * self.z[1] - self.z[0];
        self.z[0] = last;
        self.z[1]
    }
}

pub struct RotMat {
    w: f64,
    fs: usize,
    phase: f64,
    r: [[f64; 2]; 2],
    v: [f64; 2],
}
impl Osc for RotMat {
    fn new(fs: usize, f: f64, phase: f64) -> Self {
        let w = pulsatance(fs, f);
        RotMat {
            w: w,
            fs: fs,
            phase: phase,
            r: [[f64::cos(w), -f64::sin(w)], [f64::sin(w), f64::cos(w)]],
            v: [f64::cos(phase), f64::sin(phase)],
        }
    }
    fn freq(&mut self, f: f64) {
        self.w = pulsatance(self.fs, f);
        self.r = [[f64::cos(self.w), -f64::sin(self.w)], [f64::sin(self.w), f64::cos(self.w)]];
    }
    fn tick(&mut self) -> f64 {
        self.v = [self.r[0][0] * self.v[0] + self.r[0][1] * self.v[1], self.r[1][0] * self.v[0] + self.r[1][1] * self.v[1]];
        self.v[1]
    }
}
