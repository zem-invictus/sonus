use bevy::app::App;
use bevy::audio::AddAudioSource;
use bevy::prelude::{Plugin, Update};
use crate::spatial_audio::occlusion::audio_occlusion_system;
use crate::spatial_audio::source::SonusSource;
use crate::spatial_audio::spawn::sonus_audio_system;

pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SonusSource>()
            .add_systems(Update, (sonus_audio_system, audio_occlusion_system));
    }
}