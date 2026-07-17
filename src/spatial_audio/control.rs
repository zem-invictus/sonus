use crate::spatial_audio::biquad::BiquadMode;
use crate::spatial_audio::config::AudioParam;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct BiquadControl {
    pub mode: BiquadMode,

    pub cutoff_hz: AudioParam,
    pub q: AudioParam,
    pub gain_db: Option<AudioParam>,
}


pub(crate) struct PlaybackControl {
    pub(crate) volume: AudioParam,
    pub(crate) is_paused: AtomicBool,
    
    pub low_pass: Option<Arc<BiquadControl>>,
}

impl PlaybackControl {
    pub fn new() -> Self {
        Self {
            volume: AudioParam::new(1.0),
            low_pass: None,
            is_paused: AtomicBool::new(false),
        }
    }
}
