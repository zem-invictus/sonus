use crate::spatial_audio::biquad::BiquadMode;
use crate::spatial_audio::config::AudioParam;
use std::sync::Arc;

pub struct BiquadControl {
    pub mode: BiquadMode,
    pub cutoff_hz: AudioParam,
    pub q: AudioParam,
    pub gain_db: Option<AudioParam>,
}

pub(crate) struct SonusControl {
    pub occlusion_control: Option<Arc<OcclusionControl>>,
}

impl SonusControl {
    pub fn new() -> Self {
        Self {
            occlusion_control: None,
        }
    }
}

pub(crate) struct OcclusionControl {
    pub lowpass_hz: AudioParam,
    pub highpass_hz: AudioParam,
}
