//! Digital Signal Processing (DSP) primitives and filters.

use crate::sonus::config::{AttenuationControl, OcclusionControl, PanningControl};
use std::f32::consts::FRAC_1_SQRT_2;
use std::num::NonZero;
use std::sync::Arc;

/// Filter operational mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiquadMode {
    LowPass,
    HighPass,
}

/// Internal biquad filter transfer function coefficients.
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

/// Internal delay-line state of a biquad filter channel.
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

/// Biquad filter for static frequency filtering.
pub struct BiquadFilter {
    channel_states: Vec<BiquadState>,
    coeffs: BiquadCoefficients,
}

impl BiquadFilter {
    /// Creates a new `BiquadFilter` with a fixed cutoff frequency.
    pub fn new(channels: u16, sample_rate: f32, mode: BiquadMode, frequency_hz: f32) -> Self {
        let coeffs = match mode {
            BiquadMode::LowPass => {
                BiquadCoefficients::low_pass(frequency_hz, sample_rate, FRAC_1_SQRT_2)
            }
            BiquadMode::HighPass => {
                BiquadCoefficients::high_pass(frequency_hz, sample_rate, FRAC_1_SQRT_2)
            }
        };

        Self {
            channel_states: vec![BiquadState::default(); channels as usize],
            coeffs,
        }
    }

    #[inline(always)]
    pub fn process_channel_sample(&mut self, channel: usize, input: f32) -> f32 {
        self.channel_states[channel].process_sample(input, &self.coeffs)
    }
}

/// DSP gain processor supporting real-time per-sample volume interpolation.
pub struct AttenuationFilter {
    gain: f32,
    target_gain: f32,
}

impl AttenuationFilter {
    /// Creates a new `AttenuationFilter` with an initial gain factor.
    pub fn new(initial_gain: f32) -> Self {
        Self {
            gain: initial_gain,
            target_gain: initial_gain,
        }
    }

    /// Sets the target gain value for interpolation.
    pub fn set_target(&mut self, target_gain: f32) {
        self.target_gain = target_gain;
    }

    /// Processes an audio buffer in-place using per-sample linear gain interpolation.
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

        let gain_step = (self.target_gain - self.gain) * inv_frames;
        let mut current_gain = self.gain;

        for frame_chunk in buffer.frames_mut() {
            for sample in frame_chunk.iter_mut() {
                *sample *= current_gain;
            }
            current_gain += gain_step;
        }

        self.gain = self.target_gain;
    }
}

/// 3-Band crossover occlusion processing chain reading atomic band gain targets.
pub(crate) struct OcclusionChain {
    filter_low: BiquadFilter,
    filter_mid_hp: BiquadFilter,
    filter_mid_lp: BiquadFilter,
    filter_high: BiquadFilter,
    control: Arc<OcclusionControl>,
    gain_low: f32,
    target_gain_low: f32,
    gain_mid: f32,
    target_gain_mid: f32,
    gain_high: f32,
    target_gain_high: f32,
}

impl OcclusionChain {
    pub(crate) fn new(channels: u16, sample_rate: f32, control: Arc<OcclusionControl>) -> Self {
        Self {
            filter_low: BiquadFilter::new(channels, sample_rate, BiquadMode::LowPass, 500.0),
            filter_mid_hp: BiquadFilter::new(channels, sample_rate, BiquadMode::HighPass, 500.0),
            filter_mid_lp: BiquadFilter::new(channels, sample_rate, BiquadMode::LowPass, 4000.0),
            filter_high: BiquadFilter::new(channels, sample_rate, BiquadMode::HighPass, 4000.0),
            control,
            gain_low: 1.0,
            target_gain_low: 1.0,
            gain_mid: 1.0,
            target_gain_mid: 1.0,
            gain_high: 1.0,
            target_gain_high: 1.0,
        }
    }

    pub(crate) fn update(&mut self) {
        self.target_gain_low = self.control.gain_low.get();
        self.target_gain_mid = self.control.gain_mid.get();
        self.target_gain_high = self.control.gain_high.get();
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

        let step_low = (self.target_gain_low - self.gain_low) * inv_frames;
        let step_mid = (self.target_gain_mid - self.gain_mid) * inv_frames;
        let step_high = (self.target_gain_high - self.gain_high) * inv_frames;

        let mut curr_low = self.gain_low;
        let mut curr_mid = self.gain_mid;
        let mut curr_high = self.gain_high;

        for frame_chunk in buffer.frames_mut() {
            for (channel, sample) in frame_chunk.iter_mut().enumerate() {
                let input = *sample;

                let s_low = self.filter_low.process_channel_sample(channel, input);
                let s_rest = self.filter_mid_hp.process_channel_sample(channel, input);
                let s_mid = self.filter_mid_lp.process_channel_sample(channel, s_rest);
                let s_high = self.filter_high.process_channel_sample(channel, s_rest);

                *sample = (s_low * curr_low) + (s_mid * curr_mid) + (s_high * curr_high);
            }

            curr_low += step_low;
            curr_mid += step_mid;
            curr_high += step_high;
        }

        self.gain_low = self.target_gain_low;
        self.gain_mid = self.target_gain_mid;
        self.gain_high = self.target_gain_high;
    }
}

/// Attenuation processing chain reading atomic volume targets.
pub(crate) struct AttenuationChain {
    filter: AttenuationFilter,
    control: Arc<AttenuationControl>,
}

impl AttenuationChain {
    /// Creates a new `AttenuationChain` bound to atomic attenuation parameters.
    pub fn new(control: Arc<AttenuationControl>) -> Self {
        Self {
            filter: AttenuationFilter::new(1.0),
            control,
        }
    }

    pub(crate) fn update(&mut self) {
        let target_gain = self.control.gain.get();
        self.filter.set_target(target_gain);
    }

    pub fn process(&mut self, buffer: &mut BlockBuffer) {
        self.filter.process(buffer);
    }
}

/// DSP stereo panner applying Equal-Power gains per sample.
pub struct PanningFilter {
    left_gain: f32,
    target_left_gain: f32,
    right_gain: f32,
    target_right_gain: f32,
}

impl PanningFilter {
    pub fn new(initial_left: f32, initial_right: f32) -> Self {
        Self {
            left_gain: initial_left,
            target_left_gain: initial_left,
            right_gain: initial_right,
            target_right_gain: initial_right,
        }
    }

    pub fn set_targets(&mut self, target_left: f32, target_right: f32) {
        self.target_left_gain = target_left;
        self.target_right_gain = target_right;
    }

    pub fn process_stereo(&mut self, buffer: &mut BlockBuffer) {
        let frames_count = buffer.frames_count();
        if frames_count == 0 {
            return;
        }

        let inv_frames = if frames_count > 1 {
            1.0 / (frames_count - 1) as f32
        } else {
            1.0
        };

        let step_left = (self.target_left_gain - self.left_gain) * inv_frames;
        let step_right = (self.target_right_gain - self.right_gain) * inv_frames;

        let mut curr_left = self.left_gain;
        let mut curr_right = self.right_gain;

        for frame_chunk in buffer.frames_mut() {
            frame_chunk[0] *= curr_left;
            frame_chunk[1] *= curr_right;

            curr_left += step_left;
            curr_right += step_right;
        }

        self.left_gain = self.target_left_gain;
        self.right_gain = self.target_right_gain;
    }

    pub fn process_mono_to_stereo(&mut self, input: &BlockBuffer, output: &mut BlockBuffer) {
        output.clear();
        let samples_count = input.as_slice().len();
        if samples_count == 0 {
            return;
        }

        let inv_frames = if samples_count > 1 {
            1.0 / (samples_count - 1) as f32
        } else {
            1.0
        };

        let step_left = (self.target_left_gain - self.left_gain) * inv_frames;
        let step_right = (self.target_right_gain - self.right_gain) * inv_frames;

        let mut curr_left = self.left_gain;
        let mut curr_right = self.right_gain;

        for &sample in input.as_slice() {
            output.push(sample * curr_left);
            output.push(sample * curr_right);

            curr_left += step_left;
            curr_right += step_right;
        }

        self.left_gain = self.target_left_gain;
        self.right_gain = self.target_right_gain;
    }
}

/// Stereo panning processing chain reading atomic gain targets.
pub(crate) struct PanningChain {
    filter: PanningFilter,
    control: Arc<PanningControl>,
}

impl PanningChain {
    pub fn new(control: Arc<PanningControl>) -> Self {
        Self {
            filter: PanningFilter::new(std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2),
            control,
        }
    }

    pub(crate) fn update(&mut self) {
        let target_left = self.control.left_gain.get();
        let target_right = self.control.right_gain.get();
        self.filter.set_targets(target_left, target_right);
    }

    pub fn process_stereo(&mut self, buffer: &mut BlockBuffer) {
        self.filter.process_stereo(buffer);
    }

    pub fn process_mono_to_stereo(&mut self, input: &BlockBuffer, output: &mut BlockBuffer) {
        self.filter.process_mono_to_stereo(input, output);
    }
}

/// Contiguous audio block buffer managing multichannel sample storage.
pub(crate) struct BlockBuffer {
    data: Vec<f32>,
    read_index: usize,
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
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    #[inline]
    pub fn push(&mut self, sample: f32) {
        debug_assert!(
            self.data.len() < self.capacity(),
            "BlockBuffer capacity exceeded"
        );
        self.data.push(sample);
    }

    #[inline]
    pub fn pop(&mut self) -> f32 {
        let sample = self.data[self.read_index];
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
        self.read_index >= self.data.len()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panning_mono_upmix() {
        let mut mono_buffer = BlockBuffer::new(4, NonZero::new(1).unwrap());
        let mut stereo_buffer = BlockBuffer::new(4, NonZero::new(2).unwrap());
        let mut mono_samples = vec![1.0, 1.0, 1.0, 1.0].into_iter();
        mono_buffer.fill_from_iter(&mut mono_samples);

        assert_eq!(mono_buffer.channels().get(), 1);
        assert_eq!(mono_buffer.frames_count(), 4);

        let mut panner = PanningFilter::new(1.0, 0.0);
        panner.set_targets(1.0, 0.0);
        panner.process_mono_to_stereo(&mono_buffer, &mut stereo_buffer);

        assert_eq!(stereo_buffer.channels().get(), 2);
        assert_eq!(stereo_buffer.frames_count(), 4);

        let mut samples = Vec::new();
        while !stereo_buffer.is_exhausted() {
            samples.push(stereo_buffer.pop());
        }

        // Output should be interleaved stereo: [L, R, L, R, L, R, L, R]
        // Left should be ~1.0, Right should be ~0.0
        assert_eq!(samples.len(), 8);
        assert!((samples[0] - 1.0).abs() < 1e-5);
        assert!((samples[1] - 0.0).abs() < 1e-5);
        assert!((samples[6] - 1.0).abs() < 1e-5);
        assert!((samples[7] - 0.0).abs() < 1e-5);
    }
}
