use bevy::app::App;
use bevy::audio::AddAudioSource;
use bevy::prelude::Plugin;
use crate::spatial_audio::source::SpatialAudioSource;

pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SpatialAudioSource>();
    }
}