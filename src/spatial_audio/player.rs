use bevy::prelude::{Component, Handle};
use crate::spatial_audio::control::PlaybackControl;
use crate::spatial_audio::source::SpatialAudioSource;

#[derive(Component)]
pub struct SpatialAudioPlayer {
    source: Handle<SpatialAudioSource>,
    control: PlaybackControl,

    pub volume: f32,
    pub pitch: f32,
    pub paused: bool,
    pub looping: bool,
}