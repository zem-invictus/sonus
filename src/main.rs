mod spatial_audio;

use crate::spatial_audio::control::{PlaybackControl, PlaybackRegistration};
use crate::spatial_audio::source::SpatialAudioSource;
use bevy::audio::AddAudioSource;
use bevy::prelude::*;
use rodio::Source;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

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

#[derive(Resource)]
pub struct AudioPlaybackSender {
    pub sender: Sender<PlaybackRegistration>,
}

#[derive(Resource)]
pub struct AudioPlaybackReceiver {
    pub receiver: Mutex<Receiver<PlaybackRegistration>>,
}

#[derive(Component)]
pub struct SpatialAudioEmitter {
    pub playback_id: u64,
    pub control: Option<PlaybackControl>,
}

fn main() {
    let (sender, receiver) = channel::<PlaybackRegistration>();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_audio_source::<SpatialAudioSource>()
        .insert_resource(AudioPlaybackSender { sender })
        .insert_resource(AudioPlaybackReceiver {
            receiver: Mutex::new(receiver),
        })
        .add_systems(Startup, setup_game)
        .add_systems(Update, movement_system)
        .add_systems(Update, log_positions)
        .add_systems(
            Update,
            (trigger_sound, sync_audio_controls.after(trigger_sound)),
        )
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

fn sync_audio_controls(
    receiver: Res<AudioPlaybackReceiver>,
    mut query: Query<&mut SpatialAudioEmitter>,
) {
    if let Ok(receiver_guard) = receiver.receiver.try_lock() {
        while let Ok(registration) = receiver_guard.try_recv() {
            let mut matched = false;
            for mut emitter in query.iter_mut() {
                if emitter.playback_id == registration.playback_id {
                    emitter.control = Some(registration.control);
                    matched = true;
                    println!(
                        "Успешно связан аудио-контроллер для звука ID: {}",
                        registration.playback_id
                    );
                    break;
                }
            }

            if !matched {
                println!(
                    "Внимание: прилетел аудио-контроллер для ID {}, но сущность еще не создана",
                    registration.playback_id
                );
            }
        }
    }
}

fn trigger_sound(
    mut commands: Commands,
    audio_assets: Res<Assets<SpatialAudioSource>>,
    sound_handle: Option<Res<TestSoundHandle>>,
    standard_assets: Res<Assets<AudioSource>>,
    mut spatial_assets: ResMut<Assets<SpatialAudioSource>>,
    playback_sender: Res<AudioPlaybackSender>,
    mut playback_counter: Local<u64>,
) {
    let Some(handle) = sound_handle else {
        return;
    };

    if let Some(audio_source) = standard_assets.get(&handle.handle) {
        let playback_id = *playback_counter;
        *playback_counter += 1;

        let spatial_source = SpatialAudioSource {
            bytes: audio_source.bytes.clone(),
            playback_id,
            registration_sender: playback_sender.sender.clone(),
            config: HashMap::from([("low_pass".to_string(), true)]),
        };

        let spatial_handle = spatial_assets.add(spatial_source);

        commands.spawn((
            SpatialAudioEmitter {
                playback_id,
                control: None,
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
