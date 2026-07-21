use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

/// Модель затухания пространственного звука от расстояния
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttenuationModel {
    None,
    Linear {
        min_dist: f32,
        max_dist: f32,
    },
    InverseDistance {
        ref_dist: f32,
        rolloff_factor: f32,
        max_dist: f32,
    },
}

pub struct SonusControl {
    pub occlusion_control: Option<Arc<OcclusionControl>>,
}

impl SonusControl {
    pub fn new() -> Self {
        Self {
            occlusion_control: None,
        }
    }
}

pub struct OcclusionControl {
    pub lowpass_hz: AudioParam,
    pub highpass_hz: AudioParam,
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