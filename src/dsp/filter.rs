use dsp::ControllableLink;
use event::ControlEvent;
use types::{Float, Stereo, PI};

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    LP,
    HP,
    BP,
    Notch,
}

pub struct Filter {
    sample_rate: usize,
    filter_type: FilterType,
    fc: Float,
    q: Float,
    w: Float,
    a: Float,
    coeffs: ([Float; 2], [Float; 3]),
    x_s: [Stereo; 2],
}
impl Filter {
    pub fn new(sample_rate: usize) -> Self {
        let fc = sample_rate as Float / 2.;
        let w = 2.0 * PI * fc / sample_rate as Float;
        let q = 1.0;
        let filter_type = FilterType::LP;
        let (a_s, b_s) = Filter::coeffs(w, q, filter_type);
        Filter {
            sample_rate,
            filter_type,
            fc,
            q,
            w,
            a: 1.0, // unity gain
            coeffs: (a_s, b_s),
            x_s: [Stereo::default(); 2],
        }
    }

    fn coeffs(w: Float, q: Float, filter_type: FilterType) -> ([Float; 2], [Float; 3]) {
        let (sinw, cosw) = (Float::sin(w), Float::cos(w));
        let (mut a_s, mut b_s) = ([0.; 2], [0.; 3]);
        let alpha = sinw / (2.0 * q);

        let a0 = 1. + alpha;
        a_s[0] = -2. * cosw;
        a_s[1] = 1. - alpha;
        // only zeros differ
        match filter_type {
            FilterType::LP => {
                b_s[0] = (1. - cosw) / 2.;
                b_s[1] = 1. - cosw;
                b_s[2] = (1. - cosw) / 2.;
            }
            FilterType::HP => {
                b_s[0] = (1. + cosw) / 2.;
                b_s[1] = -1. - cosw;
                b_s[2] = (1. + cosw) / 2.;
            }
            FilterType::BP => {
                b_s[0] = alpha;
                b_s[1] = 0.;
                b_s[2] = -alpha;
            }
            FilterType::Notch => {
                b_s[0] = 1.;
                b_s[1] = -2. * cosw;
                b_s[2] = 1.;
            }
        }
        // normalize by dividing through a0
        for x in b_s.iter_mut().chain(a_s.iter_mut()) {
            *x /= a0;
        }
        (a_s, b_s)
    }

    fn set_freq(&mut self, freq: Float) {
        self.w = 2.0 * PI * freq / self.sample_rate as Float;
        self.update_coeffs()
    }

    fn set_q(&mut self, q: Float) {
        self.q = q;
        self.update_coeffs()
    }

    fn update_coeffs(&mut self) {
        self.coeffs = Filter::coeffs(self.w, self.q, self.filter_type)
    }

    fn set_filter_type(&mut self, filter_type: FilterType) {
        self.filter_type = filter_type;
        self.update_coeffs()
    }
}

impl ControllableLink for Filter {
    fn tick(&mut self, input: Stereo) -> Stereo {
        let (a_s, b_s) = self.coeffs;
        let fw = input - self.x_s[0] * a_s[0] - self.x_s[1] * a_s[1];
        let out = fw * b_s[0] + self.x_s[0] * b_s[1] + self.x_s[1] * b_s[2];
        self.x_s[1] = self.x_s[0];
        self.x_s[0] = fw;
        out
    }
    fn handle(&mut self, msg: &ControlEvent) {
        if let ControlEvent::Filter {
            filter_type,
            freq,
            q,
        } = *msg
        {
            if let Some(some_type) = filter_type {
                self.set_filter_type(some_type)
            }
            if let Some(some_freq) = freq {
                self.set_freq(some_freq);
            }
            if let Some(some_q) = q {
                self.set_q(some_q)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate hound;
    extern crate rand;

    use self::rand::distributions::{IndependentSample, Range};
    use super::super::ControllableLink;
    use super::{Filter, FilterType};
    use types::{Float, Stereo, MINUS_THREE_DB};

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
        for filter_type in &[
            FilterType::LP,
            FilterType::HP,
            FilterType::BP,
            FilterType::Notch,
        ] {
            let filename = format!(
                "ytterbium-{}-{:?}-filter.wav",
                env!("CARGO_PKG_VERSION"),
                filter_type
            );
            let mut writer = hound::WavWriter::create(filename, wave_spec).unwrap();

            const MINUS_SIX_DB: Float = MINUS_THREE_DB * MINUS_THREE_DB;
            let range = Range::new(-MINUS_SIX_DB, MINUS_SIX_DB);
            let mut rng = rand::thread_rng();

            let (start_freq, end_freq) = (100.0, 18_000.0);
            let mut freq = match *filter_type {
                FilterType::HP => end_freq,
                _ => start_freq,
            };
            let mut filter = Filter::new(SAMPLE_RATE);
            filter.set_freq(start_freq);
            let q = 1.0;
            filter.set_q(q);
            filter.set_filter_type(*filter_type);

            let multiplier =
                1.0 + ((end_freq as Float).ln() - (start_freq).ln()) / num_samples as Float;

            for idx in 0..num_samples {
                let r = range.ind_sample(&mut rng);
                let out = filter.tick(Stereo(r, r));
                writer.write_sample((out.0 * scale) as i32).unwrap();
                writer.write_sample((out.1 * scale) as i32).unwrap();
                freq = match *filter_type {
                    FilterType::LP | FilterType::BP | FilterType::Notch => freq * multiplier,
                    FilterType::HP => freq / multiplier,
                };
                filter.set_freq(freq);
            }
            writer.finalize().unwrap();
        }
    }
}
