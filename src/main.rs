mod spatial_audio;

use crate::spatial_audio::control::{PlaybackControl, PlaybackRegistration};
use crate::spatial_audio::source::SpatialAudioSource;
use bevy::audio::AddAudioSource;
use bevy::prelude::*;
use rodio::Source;
use std::collections::HashMap;
#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}
#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}
#[derive(Component)]
struct Name(String);

#[derive(Component)]
pub struct SpatialAudioEmitter {
    pub playback_id: u64,
    pub control: PlaybackControl,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_audio_source::<SpatialAudioSource>()
        .add_systems(Startup, setup_game)
        .add_systems(Update, movement_system)
        .add_systems(Update, log_positions)
        .run();
}

#[derive(Resource)]
pub struct TestSoundHandle {
    handle: Handle<AudioSource>,
}

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<AudioSource> = asset_server.load("input.wav");

    commands.insert_resource(TestSoundHandle { handle });

    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.5 },
        Name("Player 1".to_string()),
    ));
}
fn movement_system(mut query: Query<(&mut Position, &Velocity)>, time: Res<Time>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.x * time.delta_secs();
        pos.y += vel.y * time.delta_secs();
    }
}
fn log_positions(query: Query<(&Name, &Position)>) {
    for (name, pos) in query.iter() {
        println!(
            "Сущность: {} находится на ({:.2}, {:.2})",
            name.0, pos.x, pos.y
        );
    }
}

fn trigger_sound(
    mut commands: Commands,
    audio_assets: Res<Assets<SpatialAudioSource>>,
    sound_handle: Option<Res<TestSoundHandle>>,
    standard_assets: Res<Assets<AudioSource>>,
    mut spatial_assets: ResMut<Assets<SpatialAudioSource>>,
    mut playback_counter: Local<u64>,
) {
    let Some(handle) = sound_handle else {
        return;
    };

    if let Some(audio_source) = standard_assets.get(&handle.handle) {
        let playback_id = *playback_counter;
        *playback_counter += 1;

        let p_control = PlaybackControl {
            biquad: None,
            reverb: None,
        };

        let spatial_source = SpatialAudioSource {
            bytes: audio_source.bytes.clone(),
            playback_id,
            config: HashMap::from([("low_pass".to_string(), true)]),
            control_panel: p_control.clone(),
        };

        let spatial_handle = spatial_assets.add(spatial_source);

        commands.spawn((
            SpatialAudioEmitter {
                playback_id,
                control: p_control.clone(),
            },
            Transform::from_xyz(5.0, 0.0, 0.0),
            AudioPlayer(spatial_handle),
        ));

        commands.remove_resource::<TestSoundHandle>();
        println!(
            "Звук успешно сконвертирован в Spatial и отправлен на воспроизведение. ID: {}",
            playback_id
        );
    }
}
