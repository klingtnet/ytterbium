use types::{Float, Stereo, PI};
use dsp::SignalLink;

pub struct Filter {
    sample_rate: usize,
    fc: Float,
    q: Float,
    w: Float,
    a: Float,
    alpha: Float,
    coeffs: ([Float; 3], [Float; 3]),
    Xs: [Stereo; 2],
    Ys: [Stereo; 2],
}
impl Filter {
    fn new(sample_rate: usize, fc: Float) -> Self {
        let w = 2.0 * PI * fc / sample_rate as Float;
        let q = 1.0;
        let (sinw, cosw) = (Float::sin(w), Float::cos(w));
        let (mut As, mut Bs) = ([0.; 3], [0.; 3]);
        let alpha = sinw / (2.0 * q);
        // divide others through As[0]
        As[0] = 1. + alpha;
        As[1] = -2. * cosw;
        As[2] = 1. - alpha;
        Bs[0] = (1. - cosw) / 2.;
        Bs[1] = 1. - cosw;
        Bs[2] = (1. - cosw) / 2.;
        Filter {
            sample_rate: sample_rate,
            fc: fc,
            q: q,
            w: w,
            a: 1.0, // unity gain
            alpha: alpha,
            coeffs: (As, Bs),
            Xs: [Stereo::default(); 2],
            Ys: [Stereo::default(); 2],
        }
    }
}

impl SignalLink for Filter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        let (As, Bs) = self.coeffs;
        let fw = input - self.Xs[0]*(As[1]/As[0]) - self.Xs[1]*(As[2]/As[0]);
        let out = fw*(Bs[0]/As[0]) + self.Xs[0]*(Bs[1]/As[0]) + self.Xs[1]*(Bs[2]/As[0]);
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
