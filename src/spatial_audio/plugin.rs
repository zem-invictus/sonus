use bevy::app::App;
use bevy::audio::AddAudioSource;
use bevy::prelude::{Plugin, Update};
use crate::spatial_audio::occlusion::audio_occlusion_system;
use crate::spatial_audio::source::SonusSource;
use crate::spatial_audio::spawn::{on_spawn_sound, process_spatial_audio_intents};

pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SonusSource>()
            .add_observer(on_spawn_sound)
            .add_systems(Update, (audio_occlusion_system, process_spatial_audio_intents));
    }
}