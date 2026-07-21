use crate::spatial_audio::config::OcclusionControl;
use std::f32::consts::FRAC_1_SQRT_2;
use std::num::NonZero;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    frequency_hz: f32,
    sample_rate: f32,
    mode: BiquadMode,
    pub channel_states: Vec<BiquadState>,
    coeffs: BiquadCoefficients,
    target_coeffs: BiquadCoefficients,
}

impl BiquadFilter {
    pub fn new(channels: u16, sample_rate: f32, mode: BiquadMode) -> Self {
        let frequency_hz: f32 = match mode {
            BiquadMode::LowPass => 20000.0,
            BiquadMode::HighPass => 20.0,
        };
        let coeffs = match mode {
            BiquadMode::LowPass => {
                BiquadCoefficients::low_pass(20000.0, sample_rate, FRAC_1_SQRT_2)
            }
            BiquadMode::HighPass => BiquadCoefficients::high_pass(20.0, sample_rate, FRAC_1_SQRT_2),
        };

        Self {
            frequency_hz,
            sample_rate,
            mode,
            channel_states: vec![BiquadState::default(); channels as usize],
            coeffs,
            target_coeffs: coeffs,
        }
    }

    #[inline]
    pub fn frequency_hz(&self) -> f32 {
        self.frequency_hz
    }

    pub fn set_target(&mut self, target: BiquadCoefficients) {
        self.target_coeffs = target;
    }

    pub fn update_cutoff(&mut self, target_hz: f32, speed_factor: f32) {
        if (target_hz - self.frequency_hz).abs() > 0.1 {
            self.frequency_hz += (target_hz - self.frequency_hz) * speed_factor;
        } else {
            self.frequency_hz = target_hz;
        }

        let target_coeffs = match self.mode {
            BiquadMode::LowPass => {
                BiquadCoefficients::low_pass(self.frequency_hz, self.sample_rate, FRAC_1_SQRT_2)
            }
            BiquadMode::HighPass => {
                BiquadCoefficients::high_pass(self.frequency_hz, self.sample_rate, FRAC_1_SQRT_2)
            }
        };
        self.set_target(target_coeffs);
    }

    pub fn process(&mut self, buffer: &mut BlockBuffer) {
        let frames_count = buffer.frames_count();
        if frames_count == 0 {
            return;
        }

        let inv_frames = if frames_count > 1 {
            1.0 / (frames_count - 1) as f32
        } else {
            1.0
        };

        let b0_step = (self.target_coeffs.b0 - self.coeffs.b0) * inv_frames;
        let b1_step = (self.target_coeffs.b1 - self.coeffs.b1) * inv_frames;
        let b2_step = (self.target_coeffs.b2 - self.coeffs.b2) * inv_frames;
        let a1_step = (self.target_coeffs.a1 - self.coeffs.a1) * inv_frames;
        let a2_step = (self.target_coeffs.a2 - self.coeffs.a2) * inv_frames;

        let mut current_coeffs = self.coeffs;

        for frame_chunk in buffer.frames_mut() {
            for (channel, sample) in frame_chunk.iter_mut().enumerate() {
                let input = *sample;
                *sample = self.channel_states[channel].process_sample(input, &current_coeffs);
            }

            current_coeffs.b0 += b0_step;
            current_coeffs.b1 += b1_step;
            current_coeffs.b2 += b2_step;
            current_coeffs.a1 += a1_step;
            current_coeffs.a2 += a2_step;
        }

        self.coeffs = self.target_coeffs;
    }
}

pub(crate) struct OcclusionAudioChain {
    lowpass_filter: BiquadFilter,
    highpass_filter: BiquadFilter,
    control: Arc<OcclusionControl>,
}

impl OcclusionAudioChain {
    pub(crate) fn new(channels: u16, sample_rate: f32, control: Arc<OcclusionControl>) -> Self {
        Self {
            lowpass_filter: BiquadFilter::new(channels, sample_rate, BiquadMode::LowPass),
            highpass_filter: BiquadFilter::new(channels, sample_rate, BiquadMode::HighPass),
            control,
        }
    }

    pub(crate) fn update(&mut self) {
        let target_lpf = self.control.lowpass_hz.get();
        let speed_lpf = if target_lpf < self.lowpass_filter.frequency_hz() { 0.25 } else { 0.15 };
        self.lowpass_filter.update_cutoff(target_lpf, speed_lpf);

        let target_hpf = self.control.highpass_hz.get();
        self.highpass_filter.update_cutoff(target_hpf, 0.15);
    }

    pub fn process(&mut self, buffer: &mut BlockBuffer) {
        self.lowpass_filter.process(buffer);
        self.highpass_filter.process(buffer);
    }
}

pub(crate) struct BlockBuffer {
    data: Vec<f32>,
    read_index: u32,
    block_size: u16,
    channels: NonZero<u16>,
}

impl BlockBuffer {
    pub fn new(block_size: u16, channels: NonZero<u16>) -> Self {
        Self {
            data: Vec::with_capacity((block_size * channels.get()) as usize),
            read_index: 0,
            block_size,
            channels,
        }
    }

    #[inline]
    pub fn frames_count(&self) -> usize {
        self.data.len() / self.channels.get() as usize
    }

    #[inline]
    pub fn frames_mut(&mut self) -> impl Iterator<Item = &mut [f32]> {
        self.data.chunks_exact_mut(self.channels.get() as usize)
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        (self.block_size * self.channels.get()) as usize
    }

    #[inline]
    pub fn channels(&self) -> NonZero<u16> {
        self.channels
    }

    #[inline]
    pub fn push(&mut self, sample: f32) {
        debug_assert!(
            self.data.len() < self.data.capacity(),
            "Попытка переполнить BlockBuffer!"
        );
        self.data.push(sample);
    }

    #[inline]
    pub fn pop(&mut self) -> f32 {
        let sample = self.data[self.read_index as usize];
        self.read_index += 1;
        sample
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
        self.read_index = 0;
    }

    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.read_index as usize >= self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn fill_from_iter(&mut self, iter: &mut impl Iterator<Item = f32>) {
        let cap = self.capacity();
        for _ in 0..cap {
            if let Some(sample) = iter.next() {
                self.push(sample);
            } else {
                break;
            }
        }
    }
}
