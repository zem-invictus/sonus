use crate::spatial_audio::biquad::BiquadMode::{HighPass, LowPass};
use crate::spatial_audio::control::OcclusionControl;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

pub enum BiquadMode {
    LowPass,
    HighPass,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoefficients {
    pub fn low_pass(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.49);
        let omega = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let a0 = 1.0 + alpha;

        Self {
            b0: ((1.0 - cos_w) / 2.0) / a0,
            b1: (1.0 - cos_w) / a0,
            b2: ((1.0 - cos_w) / 2.0) / a0,
            a1: (-2.0 * cos_w) / a0,
            a2: (1.0 - alpha) / a0,
        }
    }

    pub fn high_pass(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.49);
        let omega = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let a0 = 1.0 + alpha;

        Self {
            b0: ((1.0 + cos_w) / 2.0) / a0,
            b1: (-(1.0 + cos_w)) / a0,
            b2: ((1.0 + cos_w) / 2.0) / a0,
            a1: (-2.0 * cos_w) / a0,
            a2: (1.0 - alpha) / a0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    s1: f32,
    s2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32, coeffs: &BiquadCoefficients) -> f32 {
        let output = coeffs.b0 * input + self.s1;
        self.s1 = coeffs.b1 * input - coeffs.a1 * output + self.s2;
        self.s2 = coeffs.b2 * input - coeffs.a2 * output;
        output
    }
}

pub struct BiquadFilter {
    pub(crate) frequency_hz: f32,
    pub channel_states: Vec<BiquadState>,
    pub coeffs: BiquadCoefficients,
}

impl BiquadFilter {
    pub fn new(channels: u16, sample_rate: f32, mode: BiquadMode) -> Self {
        let frequency_hz: f32 = match mode {
            LowPass => 20000.0,
            HighPass => 20.0,
        };
        let coeffs = match mode {
            LowPass => BiquadCoefficients::low_pass(20000.0, sample_rate, FRAC_1_SQRT_2),
            HighPass => BiquadCoefficients::high_pass(20.0, sample_rate, FRAC_1_SQRT_2),
        };

        Self {
            frequency_hz,
            channel_states: vec![BiquadState::default(); channels as usize],
            coeffs,
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        let channels_count = self.channel_states.len();

        for (i, sample) in samples.iter_mut().enumerate() {
            let channel = i % channels_count;
            let input = *sample;
            *sample = self.channel_states[channel].process_sample(input, &self.coeffs);
        }
    }
}

pub(crate) struct OcclusionAudioChain {
    lowpass_filter: BiquadFilter,
    highpass_filter: BiquadFilter,
    control: Arc<OcclusionControl>,
    sample_rate: f32
}

impl OcclusionAudioChain {
    pub(crate) fn new(
        channels: u16,
        sample_rate: f32,
        control: Arc<OcclusionControl>,
    ) -> Self {
        Self {
            lowpass_filter: BiquadFilter::new(channels, sample_rate, LowPass),
            highpass_filter: BiquadFilter::new(channels, sample_rate, HighPass),
            control,
            sample_rate
        }
    }

    pub(crate) fn update(&mut self) {
        let target_lpf = self.control.lowpass_hz.get();
        let current_lpf = self.lowpass_filter.frequency_hz;
        
        let next_lpf = current_lpf + (target_lpf - current_lpf) * 0.15;

        if (next_lpf - current_lpf).abs() > 0.1 {
            self.lowpass_filter.frequency_hz = next_lpf;
            self.lowpass_filter.coeffs = BiquadCoefficients::low_pass(next_lpf, self.sample_rate, FRAC_1_SQRT_2);
        }

        let target_hpf = self.control.highpass_hz.get();
        let current_hpf = self.highpass_filter.frequency_hz;
        let next_hpf = current_hpf + (target_hpf - current_hpf) * 0.15;

        if (next_hpf - current_hpf).abs() > 0.1 {
            self.highpass_filter.frequency_hz = next_hpf;
            self.highpass_filter.coeffs = BiquadCoefficients::high_pass(next_hpf, self.sample_rate, FRAC_1_SQRT_2);
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        self.lowpass_filter.process(samples);
        self.highpass_filter.process(samples);
    }
}
