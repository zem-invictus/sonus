use std::num::NonZero;
use crate::spatial_audio::buffer::BlockBuffer;
use rodio::Source;
use crate::spatial_audio::control::{BiquadControl, FilterControl};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterType {
    LowPass,
    HighPass,
    Reverb,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterParams {
    LowPass {
        cutoff: f32,
        resonance: f32,
    },
    HighPass {
        cutoff: f32,
    },
    Reverb {
        room_size: f32,
        damping: f32,
        wet: f32,
    },
}

pub trait FilterConfig: 'static {
    type Control: FilterControl;
    fn build_control(self) -> Self::Control;
}

#[derive(Clone, Copy)]
pub struct LowPassConfig {
    cutoff: f32,
    resonance: f32,
}

impl FilterConfig for LowPassConfig {
    type Control = BiquadControl;

    fn build_control(self) -> Self::Control {
        BiquadControl::new(self.cutoff, self.resonance)
    }
}

pub trait AudioFilter: Send + Sync {
    fn process(&mut self, samples: &mut [f32], channels: u16);
    fn update(&mut self, sample_rate: u32);
}
