mod osc;
use osc::{Osc, Simple, PhaseAccu, RotMat, Lookup};
use std::{f64, slice};

const FS: usize = 48000;
const F: f64 = 440.0;

fn main() {
    let mut simple_osc = Simple::new(FS, F, 0.0);
    let normal: Vec<f64> = (0..FS * 100).map(|_| simple_osc.tick()).collect();

    let mut pa_osc = PhaseAccu::new(FS, F, 0.0);
    let pa: Vec<f64> = (0..FS * 100).map(|_| pa_osc.tick()).collect();

    let mut rot_osc = RotMat::new(FS, F, 0.0);
    let rot: Vec<f64> = (0..FS * 100).map(|_| rot_osc.tick()).collect();

    let mut lookup_osc = Lookup::new(FS, F, 0.0);
    let lookup: Vec<f64> = (0..FS * 100).map(|_| lookup_osc.tick()).collect();

    let pa_err: f64 = normal.iter()
                            .zip(&pa)
                            .map(|(a, b)| a.abs() - b.abs())
                            .fold(0.0f64, |sum, v| sum + v);
    let rot_err: f64 = normal.iter()
                             .zip(&rot)
                             .map(|(a, b)| a.abs() - b.abs())
                             .fold(0.0f64, |sum, v| sum + v);
    let lookup_err: f64 = normal.iter()
                                .zip(&lookup)
                                .map(|(a, b)| a.abs() - b.abs())
                                .fold(0.0f64, |sum, v| sum + v);
    println!("PhaseAccu: min: {}\tmax: {}",
             min(pa.iter()),
             max(pa.iter()));
    println!("RotMat: min: {}\tmax: {}", min(rot.iter()), max(rot.iter()));
    println!("Loookup: min: {}\tmax: {}",
             min(lookup.iter()),
             max(lookup.iter()));
    println!("{}\t{}\t{}", pa_err, rot_err, lookup_err);
}

// there are no min and max values for floats,
// because they only implement PartialOrdering cause of NaN
fn min(it: slice::Iter<f64>) -> f64 {
    it.fold(f64::NAN, |l, &r| {
        if l < r {
            l
        } else {
            r
        }
    })
}

fn max(it: slice::Iter<f64>) -> f64 {
    it.fold(f64::NAN, |l, &r| {
        if l > r {
            l
        } else {
            r
        }
    })
}
