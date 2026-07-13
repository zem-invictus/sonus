use crate::spatial_audio::biquad::BiquadMode;
use crate::spatial_audio::config::AudioParam;
use std::sync::Arc;

pub struct BiquadControl {
    pub mode: BiquadMode,

    pub cutoff_hz: AudioParam,
    pub q: AudioParam,
    pub gain_db: Option<AudioParam>,
}


pub struct PlaybackControl {
    pub volume: AudioParam,
    pub low_pass: Option<Arc<BiquadControl>>,
}

impl PlaybackControl {
    pub fn new() -> Self {
        Self {
            volume: AudioParam::new(1.0),
            low_pass: None,
        }
    }
}
