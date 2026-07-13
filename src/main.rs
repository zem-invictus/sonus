mod spatial_audio;

use crate::spatial_audio::control::PlaybackControl;
use crate::spatial_audio::plugin::SpatialAudioPlugin;
use crate::spatial_audio::source::SpatialAudioSource;
use bevy::prelude::*;
use std::sync::Arc;

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
pub struct SpatialAudioController {
    pub playback_id: u64,
    pub control: Arc<PlaybackControl>,
}

#[derive(Component)]
struct AudioListener;

#[derive(Component)]
struct Wall {
    half_extents: Vec3,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SpatialAudioPlugin))
        .add_systems(Startup, setup_game)
        .add_systems(Update, trigger_sound)
        .add_systems(Update, movement_system)
        .run();
}

#[derive(Resource)]
pub struct TestSoundHandle {
    handle: Handle<AudioSource>,
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let handle: Handle<AudioSource> = asset_server.load("input.wav");
    commands.insert_resource(TestSoundHandle { handle });

    // 1. Направление света
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. 3D Камера с видом сверху-сбоку
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 12.0, 12.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // 3. Зеленая земля (пол)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.2),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // 4. Препятствие (красная стена) посередине
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 3.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2), // Красная
            ..default()
        })),
        Transform::from_xyz(0.0, 1.5, 0.0),
        Wall {
            half_extents: Vec3::new(1.0, 1.5, 5.0),
        },
    ));

    // 5. Игрок-Слушатель (белая сфера)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0), // Белый игрок
            ..default()
        })),
        Transform::from_xyz(5.0, 1.0, 0.0),
        Position { x: 5.0, y: 0.0 },
        Velocity { x: 0.0, y: 0.0 },
        Name("Player 1".to_string()),
        AudioListener,
    ));
}

fn movement_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Position), With<AudioListener>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
        direction.z -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
        direction.z += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction != Vec3::ZERO {
        let speed = 5.0;
        for (mut transform, mut position) in query.iter_mut() {
            let offset = direction.normalize() * speed * time.delta_secs();
            transform.translation += offset;
            position.x = transform.translation.x;
            position.y = transform.translation.z; // position.y хранит координату Z плоскости в логах
        }
    }
}

fn trigger_sound(
    mut commands: Commands,
    sound_handle: Option<Res<TestSoundHandle>>,
    standard_assets: Res<Assets<AudioSource>>,
    mut spatial_assets: ResMut<Assets<SpatialAudioSource>>,
    mut playback_counter: Local<u64>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(handle) = sound_handle else {
        return;
    };

    if let Some(audio_source) = standard_assets.get(&handle.handle) {
        let playback_id = *playback_counter;
        *playback_counter += 1;

        let spatial_source = SpatialAudioSource::new(audio_source.bytes.clone(), playback_id)
            .with_lowpass_filter(400.0);

        let p_control = spatial_source.control.clone();
        let spatial_handle = spatial_assets.add(spatial_source);

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.2, 0.8), // Синий
                ..default()
            })),
            Transform::from_xyz(-5.0, 1.0, 0.0),
            AudioPlayer(spatial_handle),
            SpatialAudioController {
                playback_id,
                control: p_control,
            },
        ));

        commands.remove_resource::<TestSoundHandle>();
        println!(
            "Звук успешно сконвертирован в Spatial и отправлен на воспроизведение. ID: {}",
            playback_id
        );
    }
}
