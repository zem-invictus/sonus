use crate::spatial_audio::buffer::BlockBuffer;
use crate::spatial_audio::control::{BiquadControl, FilterControl};
use rodio::Source;
use std::num::NonZero;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterType {
    LowPass,
    HighPass,
    Reverb,
}

pub trait AudioFilter: Send + Sync {
    fn process(&mut self, samples: &mut [f32]);
    fn update(&mut self, sample_rate: u32);
}
