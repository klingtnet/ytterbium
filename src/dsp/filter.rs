use types::{Float, Stereo};
use dsp::SignalLink;

pub struct Filter {}

impl SignalLink for Filter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        input
    }
}

#[cfg(test)]
mod tests {
    extern crate hound;
    extern crate rand;

    use super::*;
    use types::Float;
    use self::rand::distributions::{IndependentSample, Range};

    #[test]
    fn test_filter() {
        const SAMPLE_RATE: usize = 48_000;
        let num_samples = SAMPLE_RATE * 10;

        let wave_spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 32,
        };
        let scale = ::std::i32::MAX as Float;
        let filename = format!("ytterbium-{}-filter.wav", env!("CARGO_PKG_VERSION"));
        let mut writer = hound::WavWriter::create(filename, wave_spec).unwrap();

        let range = Range::new(-1. as Float, 1.);
        let mut rng = rand::thread_rng();

        for idx in 0..num_samples {
            writer.write_sample((range.ind_sample(&mut rng) * scale) as i32).unwrap();
        }
        writer.finalize().unwrap();
    }
}
