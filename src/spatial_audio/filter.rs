use std::time::Duration;
use rodio::{ChannelCount, SampleRate, Source};
use crate::spatial_audio::biquad::{BiquadCoefficients, BiquadState};
use crate::spatial_audio::control::BiquadControl;

pub struct BiquadFilter<I: Source> {
    pub inner: I,
    pub control: BiquadControl,
    pub channels: u16,
    pub channel_states: Vec<BiquadState>,
    pub sample_rate: u32,
    pub current_cutoff_hz: f32,
    pub coeffs: BiquadCoefficients,
    pub sample_counter: usize,
}

impl<I: Source> Source for BiquadFilter<I> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(self.channels).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(self.sample_rate).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

impl<I: Source> Iterator for BiquadFilter<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let raw_sample = self.inner.next()?;

        if self.sample_counter.is_multiple_of(64) {
            let target_cutoff = self.control.cutoff_hz.get();
            if (target_cutoff - self.current_cutoff_hz).abs() > 1.0 {
                self.current_cutoff_hz = target_cutoff;
                self.coeffs = BiquadCoefficients::low_pass(
                    target_cutoff,
                    self.sample_rate as f32,
                    std::f32::consts::FRAC_1_SQRT_2,
                );
            }
        }
        self.sample_counter += 1;

        let volume = self.control.volume.get();
        let float_sample = raw_sample * volume;

        let channel = (self.sample_counter - 1) % (self.channels as usize);

        let filtered_sample =
            self.channel_states[channel].process_sample(float_sample, &self.coeffs);

        Some(filtered_sample)
    }
}