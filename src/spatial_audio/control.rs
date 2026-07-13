use std::any::TypeId;
use crate::spatial_audio::filter::{AudioFilter, FilterType};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use crate::spatial_audio::biquad::BiquadFilter;

pub trait FilterControl : Send + Sync + 'static {
    fn build_filter(self: Arc<Self>, channels: u16, sample_rate: u32) -> Box<dyn AudioFilter>;
}

pub struct AudioParam {
    pub value: AtomicU32,
}

impl AudioParam {
    pub fn new(val: f32) -> Self {
        Self {
            value: AtomicU32::new(val.to_bits()),
        }
    }

    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.value.load(Relaxed))
    }

    #[inline]
    pub fn set(&self, val: f32) {
        self.value.store(val.to_bits(), Relaxed)
    }
}

pub struct BiquadControl {
    pub cutoff_hz: AudioParam,
    pub volume: AudioParam,
}

impl BiquadControl {
    pub fn new(cutoff_hz: f32, volume: f32) -> Self {
        Self {
            cutoff_hz: AudioParam::new(cutoff_hz),
            volume: AudioParam::new(volume),
        }
    }
}

pub struct LowPassControl {
    pub cutoff_hz: AudioParam,
    pub volume: AudioParam,
}

impl LowPassControl {
    pub fn new(cutoff_hz: f32, volume: f32) -> Self {
        Self {
            cutoff_hz: AudioParam::new(cutoff_hz),
            volume: AudioParam::new(volume),
        }
    }
}

impl FilterControl for LowPassControl {
    fn build_filter(self: Arc<Self>, channels: u16, sample_rate: u32) -> Box<dyn AudioFilter> {
        Box::new(BiquadFilter::new(self, channels, sample_rate))
    }
}

pub struct ReverbControl {
    pub room_size: AudioParam,
    pub wet_mix: AudioParam,
}

impl ReverbControl {
    pub fn new(room_size: f32, wet_mix: f32) -> Self {
        Self {
            room_size: AudioParam::new(room_size),
            wet_mix: AudioParam::new(wet_mix),
        }
    }
}

#[derive(Clone)]
pub struct PlaybackControl {
    pub filters: HashMap<TypeId, Arc<dyn FilterControl>>,
}

impl PlaybackControl {
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
        }
    }
}
