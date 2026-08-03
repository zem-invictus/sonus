//! Shared control configuration and lock-free parameter types for spatial audio.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;

/// Distance-based attenuation models for spatial audio emitters.
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

/// Shared control structure attached to a spatial audio emitter.
#[derive(Clone, Default)]
pub struct SonusControl {
    pub occlusion_control: Option<Arc<OcclusionControl>>,
    pub attenuation_control: Option<Arc<AttenuationControl>>,
    pub panning_control: Option<Arc<PanningControl>>,
}

impl SonusControl {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parameters for real-time 3-band occlusion filtering and perceived diffraction direction.
pub struct OcclusionControl {
    pub gain_low: AudioParam,
    pub gain_mid: AudioParam,
    pub gain_high: AudioParam,
    pub perceived_dir_x: AudioParam,
    pub perceived_dir_y: AudioParam,
    pub perceived_dir_z: AudioParam,
}

/// Parameters for real-time distance attenuation.
pub struct AttenuationControl {
    pub model: AttenuationModel,
    pub gain: AudioParam,
}

pub struct PanningControl {
    pub right_gain: AudioParam,
    pub left_gain: AudioParam,
}

/// Lock-free floating-point audio parameter synchronized between ECS and audio threads.
pub struct AudioParam {
    value: AtomicU32,
}

impl AudioParam {
    /// Creates a new atomic audio parameter with an initial float value.
    pub fn new(val: f32) -> Self {
        Self {
            value: AtomicU32::new(val.to_bits()),
        }
    }

    /// Reads the current float value atomically.
    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.value.load(Relaxed))
    }

    /// Atomically updates the float value.
    #[inline]
    pub fn set(&self, val: f32) {
        self.value.store(val.to_bits(), Relaxed)
    }
}
