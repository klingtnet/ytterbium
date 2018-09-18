#![feature(test)]
extern crate test;
use test::Bencher;

extern crate ytterbium;
use ytterbium::osc::{Lookup, Osc, PhaseAccu, RotMat, Simple};

const FS: usize = 48000;
const F: f64 = 440.0;

#[bench]
fn bench_simple(b: &mut Bencher) {
    let mut osc = Simple::new(FS, F, 0.0);
    b.iter(|| {
        for _ in 0..FS {
            osc.tick();
        }
    });
}

#[bench]
fn bench_phase_accu(b: &mut Bencher) {
    let mut osc = PhaseAccu::new(FS, F, 0.0);
    b.iter(|| {
        for _ in 0..FS {
            osc.tick();
        }
    });
}

#[bench]
fn bench_rot_mat(b: &mut Bencher) {
    let mut osc = RotMat::new(FS, F, 0.0);
    b.iter(|| {
        for _ in 0..FS {
            osc.tick();
        }
    });
}

#[bench]
fn bench_lookup(b: &mut Bencher) {
    let mut osc = Lookup::new(FS, F, 0.0);
    b.iter(|| {
        for _ in 0..FS {
            osc.tick();
        }
    })
}
