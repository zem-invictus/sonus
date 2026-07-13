use bevy::app::App;
use bevy::audio::AddAudioSource;
use bevy::prelude::{Plugin, Update};
use crate::spatial_audio::occlusion::{audio_occlusion_system};
use crate::spatial_audio::source::SpatialAudioSource;

pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SpatialAudioSource>().add_systems(Update, audio_occlusion_system);
    }
}