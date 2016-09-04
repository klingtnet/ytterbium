use types::{Float, Stereo, PI};
use dsp::SignalLink;

enum FilterType {
    LP,
    HP,
    BP,
    Notch,
}

pub struct Filter {
    sample_rate: usize,
    fc: Float,
    q: Float,
    w: Float,
    a: Float,
    coeffs: ([Float; 2], [Float; 3]),
    Xs: [Stereo; 2],
    Ys: [Stereo; 2],
}
impl Filter {
    fn new(sample_rate: usize, fc: Float) -> Self {
        let w = 2.0 * PI * fc / sample_rate as Float;
        let q = 1.0;
        let (As, Bs) = Filter::coeffs(w, q, FilterType::LP);
        Filter {
            sample_rate: sample_rate,
            fc: fc,
            q: q,
            w: w,
            a: 1.0, // unity gain
            coeffs: (As, Bs),
            Xs: [Stereo::default(); 2],
            Ys: [Stereo::default(); 2],
        }
    }

    fn coeffs(w: Float, q: Float, filter_type: FilterType) -> ([Float; 2], [Float; 3]) {
        let (sinw, cosw) = (Float::sin(w), Float::cos(w));
        let (mut As, mut Bs) = ([0.; 2], [0.; 3]);
        let alpha = sinw / (2.0 * q);

        let a0 = 1. + alpha;
        // normalize by dividing through a0
        match filter_type {
            FilterType::LP => {
                As[0] = -2. * cosw / a0;
                As[1] = (1. - alpha) / a0;
                Bs[0] = (1. - cosw) * 0.5 / a0;
                Bs[1] = (1. - cosw) / a0;
                Bs[2] = (1. - cosw) * 0.5 / a0;
            }
            _ => unimplemented!(),
        }
        (As, Bs)
    }
}

impl SignalLink for Filter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        let (As, Bs) = self.coeffs;
        let fw = input - self.Xs[0] * As[0] - self.Xs[1] * As[1];
        let out = fw * Bs[0] + self.Xs[0] * Bs[1] + self.Xs[1] * Bs[2];
        self.Xs[1] = self.Xs[0];
        self.Xs[0] = fw;
        self.Ys[1] = self.Ys[0];
        self.Ys[0] = out;
        out
    }
}

#[cfg(test)]
mod tests {
    extern crate hound;
    extern crate rand;

    use super::*;
    use super::super::SignalLink;
    use types::{Float,Stereo,MINUS_THREE_DB};
    use self::rand::distributions::{IndependentSample, Range};

    #[test]
    fn test_filter() {
        const SAMPLE_RATE: usize = 48_000;
        let mut filter = Filter::new(SAMPLE_RATE, 4400.0);
        let num_samples = SAMPLE_RATE * 1;

        let wave_spec = hound::WavSpec {
            channels: 2,
            sample_format: hound::SampleFormat::Int,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 32,
        };
        let scale = ::std::i32::MAX as Float;
        let filename = format!("ytterbium-{}-filter.wav", env!("CARGO_PKG_VERSION"));
        let mut writer = hound::WavWriter::create(filename, wave_spec).unwrap();

        let range = Range::new(-MINUS_THREE_DB, MINUS_THREE_DB);
        let mut rng = rand::thread_rng();

        for idx in 0..num_samples {
            let r = range.ind_sample(&mut rng);
            let out = filter.tick(Stereo(r, r));
            writer.write_sample((out.0 * scale) as i32).unwrap();
            writer.write_sample((out.1 * scale) as i32).unwrap();
        }
        writer.finalize().unwrap();
    }
}
