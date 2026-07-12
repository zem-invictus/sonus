use std::f32::consts::FRAC_1_SQRT_2;
use crate::spatial_audio::control::{BiquadControl, FilterControl};
use crate::spatial_audio::filter::AudioFilter;
use std::sync::Arc;

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

pub struct BiquadFilter<C: FilterControl> {
    pub control: Arc<C>,
    pub channel_states: Vec<BiquadState>,
    pub current_cutoff_hz: f32,
    pub coeffs: BiquadCoefficients,
}

impl<C: FilterControl> BiquadFilter<C> {
    pub fn new_lowpass_filter(control: Arc<C>, channels: u16, sample_rate: u32, cutoff_hz: u32) -> Self {
        let current_cutoff_hz = cutoff_hz.get();
        let coeffs = BiquadCoefficients::low_pass(
            current_cutoff_hz,
            sample_rate as f32,
            FRAC_1_SQRT_2,
        );
        BiquadFilter {
            control,
            channel_states: vec![BiquadState::default(); channels as usize],
            current_cutoff_hz,
            coeffs,
        }
    }
}

impl<C: FilterControl> AudioFilter for BiquadFilter<C> {
    fn process(&mut self, samples: &mut [f32], channels: u16) {
        let volume = self.control.volume.get();
        let channels_count = self.channel_states.len();

        for (i, sample) in samples.iter_mut().enumerate() {
            let channel = i % channels_count;
            let input = *sample * volume;
            *sample = self.channel_states[channel].process_sample(input, &self.coeffs);
        }
    }

    fn update(&mut self, sample_rate: u32) {
        let target_cutoff = self.control.cutoff_hz.get();
        if (target_cutoff - self.current_cutoff_hz).abs() > 1. {
            self.current_cutoff_hz = target_cutoff;
            self.coeffs = BiquadCoefficients::low_pass(target_cutoff, sample_rate as f32, FRAC_1_SQRT_2);
        }
    }
}
