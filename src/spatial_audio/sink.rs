use std::sync::atomic::Ordering::Relaxed;
use crate::spatial_audio::control::PlaybackControl;
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::Component;
use rodio::source::SeekError;
use std::time::Duration;

#[derive(Component)]
pub struct SonusSink {
    pub(crate) sink: PlaybackControl,
}

impl SonusSink {
    pub fn volume(&self) -> f32 {
        self.sink.volume.get()
    }

    pub fn set_volume(&mut self, volume: f32) {
        let clamped_volume = volume.clamp(0.0, 1.0);
        self.sink.volume.set(clamped_volume);
    }

    fn play(&self) {
        self.sink.is_paused.store(false, Relaxed);
    }

    fn position(&self) -> Duration {
        todo!()
    }

    fn try_seek(&self, pos: Duration) -> Result<(), SeekError> {
        todo!()
    }

    fn pause(&self) {
        todo!()
    }

    fn is_paused(&self) -> bool {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn empty(&self) -> bool {
        todo!()
    }

    fn is_muted(&self) -> bool {
        todo!()
    }

    fn mute(&mut self) {
        todo!()
    }

    fn unmute(&mut self) {
        todo!()
    }
}
