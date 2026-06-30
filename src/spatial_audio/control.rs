use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;

#[derive(Clone)]
pub struct DecoderControl {
    cutoff_hz: Arc<AtomicU32>,
    volume: Arc<AtomicU32>,
}

impl DecoderControl {
    pub fn new(cutoff_hz: f32, volume: f32) -> Self {
        Self {
            cutoff_hz: Arc::new(AtomicU32::new(cutoff_hz.to_bits())),
            volume: Arc::new(AtomicU32::new(volume.to_bits())),
        }
    }
    pub fn set_cutoff(&self, cutoff_hz: f32) {
        self.cutoff_hz.store(cutoff_hz.to_bits(), Relaxed);
    }
    pub fn get_cutoff(&self) -> f32 {
        f32::from_bits(self.cutoff_hz.load(Relaxed))
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Relaxed);
    }
    pub fn get_volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Relaxed))
    }
}

pub struct PlaybackRegistration {
    pub playback_id: u64,
    pub control: DecoderControl,
}