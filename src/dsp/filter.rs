use types::{Float, Stereo, PI};
use dsp::SignalLink;

#[derive(Debug)]
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
}
impl Filter {
    fn new(sample_rate: usize, fc: Float, filter_type: &FilterType) -> Self {
        let w = 2.0 * PI * fc / sample_rate as Float;
        let q = 1.0;
        let (As, Bs) = Filter::coeffs(w, q, filter_type);
        Filter {
            sample_rate: sample_rate,
            fc: fc,
            q: q,
            w: w,
            a: 1.0, // unity gain
            coeffs: (As, Bs),
            Xs: [Stereo::default(); 2],
        }
    }

    fn coeffs(w: Float, q: Float, filter_type: &FilterType) -> ([Float; 2], [Float; 3]) {
        let (sinw, cosw) = (Float::sin(w), Float::cos(w));
        let (mut As, mut Bs) = ([0.; 2], [0.; 3]);
        let alpha = sinw / (2.0 * q);

        let a0 = 1. + alpha;
        As[0] = -2. * cosw;
        As[1] = 1. - alpha;
        // only zeros differ
        match *filter_type {
            FilterType::LP => {
                Bs[0] = (1. - cosw) / 2.;
                Bs[1] = 1. - cosw;
                Bs[2] = (1. - cosw) / 2.;
            }
            FilterType::HP => {
                Bs[0] = (1. + cosw) / 2.;
                Bs[1] = -1. - cosw;
                Bs[2] = (1. + cosw) / 2.;
            }
            FilterType::BP => {
                Bs[0] = alpha;
                Bs[1] = 0.;
                Bs[2] = -alpha;
            }
            FilterType::Notch => {
                Bs[0] = 1.;
                Bs[1] = -2. * cosw;
                Bs[2] = 1.;
            }
        }
        // normalize by dividing through a0
        for x in Bs.iter_mut().chain(As.iter_mut()) {
            *x /= a0;
        }
        (As, Bs)
    }

    fn set_freq(&mut self, freq: Float, filter_type: &FilterType) {
        self.coeffs = Filter::coeffs(2.0 * PI * freq / self.sample_rate as Float,
                                     self.q,
                                     filter_type)
    }

    fn set_q(&mut self, q: Float) {
        unimplemented!()
    }
}

impl SignalLink for Filter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        let (As, Bs) = self.coeffs;
        let fw = input - self.Xs[0] * As[0] - self.Xs[1] * As[1];
        let out = fw * Bs[0] + self.Xs[0] * Bs[1] + self.Xs[1] * Bs[2];
        self.Xs[1] = self.Xs[0];
        self.Xs[0] = fw;
        out
    }
}

#[cfg(test)]
mod tests {
    extern crate hound;
    extern crate rand;

    use super::{Filter, FilterType};
    use super::super::SignalLink;
    use types::{Float, Stereo, MINUS_THREE_DB};
    use self::rand::distributions::{IndependentSample, Range};

    #[test]
    fn test_filter() {
        const SAMPLE_RATE: usize = 48_000;
        let num_samples = SAMPLE_RATE * 10;

        let wave_spec = hound::WavSpec {
            channels: 2,
            sample_format: hound::SampleFormat::Int,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 32,
        };
        let scale = ::std::i32::MAX as Float;
        for filter_type in &[FilterType::LP, FilterType::HP, FilterType::BP, FilterType::Notch] {
            let filename = format!("ytterbium-{}-{:?}-filter.wav",
                                   env!("CARGO_PKG_VERSION"),
                                   filter_type);
            let mut writer = hound::WavWriter::create(filename, wave_spec).unwrap();

            const MINUS_SIX_DB: Float = MINUS_THREE_DB * MINUS_THREE_DB;
            let range = Range::new(-MINUS_SIX_DB, MINUS_SIX_DB);
            let mut rng = rand::thread_rng();

            let (start_freq, end_freq) = (100.0, 18_000.0);
            let mut freq = match *filter_type {
                FilterType::HP => end_freq,
                _ => start_freq,
            };
            let mut filter = Filter::new(SAMPLE_RATE, freq, filter_type);

            let multiplier = 1.0 +
                             ((end_freq as Float).ln() - (start_freq).ln()) / num_samples as Float;

            for idx in 0..num_samples {
                let r = range.ind_sample(&mut rng);
                let out = filter.tick(Stereo(r, r));
                writer.write_sample((out.0 * scale) as i32).unwrap();
                writer.write_sample((out.1 * scale) as i32).unwrap();
                freq = match *filter_type {
                    FilterType::LP | FilterType::BP | FilterType::Notch => freq * multiplier,
                    FilterType::HP => freq / multiplier,
                };
                filter.set_freq(freq, filter_type);
            }
            writer.finalize().unwrap();
        }
    }
}
