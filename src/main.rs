mod spatial_audio;

use crate::spatial_audio::emitter::SonusEmitter;
use crate::spatial_audio::occlusion::{AcousticMaterial, AudioListener, Wall};
use crate::spatial_audio::plugin::SpatialAudioPlugin;
use bevy::prelude::*;

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

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SpatialAudioPlugin))
        .add_systems(Startup, setup_game)
        .add_systems(Update, (movement_system, debug_visualize_occlusion))
        .run();
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.8), // Синий
            ..default()
        })),
        Transform::from_xyz(-5.0, 1.0, 0.0),
        SonusEmitter::new("input.wav").with_occlusion(),
    ));

    // 2. Направление света
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 3. 3D Камера с видом сверху-сбоку
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 12.0, 12.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // 4. Зеленая земля (пол)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.2),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // 5. Препятствие (красная стена) с компонентом акустического материала
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
        AcousticMaterial {
            lowpass_cutoff_hz: 20000.0,   // Глухая стена, оставляет только глухие басы (LPF срез 300 Гц)
            highpass_cutoff_hz: 1000.0,  // Немного приглушает суб-бас (HPF срез 100 Гц)
        },
    ));

    // 6. Игрок-Слушатель (белая сфера)
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
            position.y = transform.translation.z;
        }
    }
}

/// Система визуальной отладки: красит игрока в желтый цвет, 
/// если LPF срез любого активного эмиттера упал ниже 19000 Гц (звук окклюдирован)
fn debug_visualize_occlusion(
    emitter_query: Query<&SonusEmitter>,
    listener_query: Query<&MeshMaterial3d<StandardMaterial>, With<AudioListener>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(material_handle) = listener_query.iter().next() else {
        return;
    };

    let mut is_any_occluded = false;
    for emitter in emitter_query.iter() {
        if let Some(occlusion_control) = &emitter.control.occlusion_control {
            if occlusion_control.lowpass_hz.get() < 19000.0 {
                is_any_occluded = true;
                break;
            }
        }
    }

    if let Some(mut mat) = materials.get_mut(&material_handle.0) {
        if is_any_occluded {
            mat.base_color = Color::srgb(1.0, 1.0, 0.0); // Желтый
        } else {
            mat.base_color = Color::srgb(1.0, 1.0, 1.0); // Белый
        }
    }
}
