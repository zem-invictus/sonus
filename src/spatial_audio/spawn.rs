use crate::spatial_audio::emitter::{SonusEmitter, SonusSourceInput};
use crate::spatial_audio::source::SonusSource;
use bevy::prelude::*;

pub(crate) fn sonus_audio_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<(Entity, &mut SonusEmitter), Without<AudioPlayer<SonusSource>>>,
    audio_assets: Res<Assets<AudioSource>>,
    mut sonus_assets: ResMut<Assets<SonusSource>>,
) {
    for (entity, mut emitter) in &mut query {
        let audio_handle = match &emitter.source {
            SonusSourceInput::Path(path) => {
                let handle = asset_server.load(path);
                emitter.update_handle_status(handle.clone());
                handle
            }
            SonusSourceInput::AudioHandle(handle) => handle.clone(),
        };

        let Some(audio_source) = audio_assets.get(&audio_handle) else {
            continue;
        };

        let sonus_source = SonusSource::new(audio_source.bytes.clone(), emitter.control.clone());
        let sonus_handle = sonus_assets.add(sonus_source);

        // Вставляем только компонент воспроизведения звука
        commands.entity(entity).insert(AudioPlayer(sonus_handle));
    }
}