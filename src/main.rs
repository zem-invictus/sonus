mod sonus;

use crate::sonus::{AcousticMaterial, AudioListener, SonusEmitter, SpatialAudioPlugin};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

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

/// Component tag for the on-screen FPS display UI text.
#[derive(Component)]
struct FpsText;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SpatialAudioPlugin,
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup_game)
        .add_systems(
            Update,
            (
                movement_system,
                mouse_look_system,
                cursor_toggle_system,
                debug_visualize_occlusion,
                fps_update_system,
            ),
        )
        .run();
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(mut cursor) = cursor_options.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.8),
            ..default()
        })),
        Transform::from_xyz(-5.0, 1.0, 0.0),
        SonusEmitter::new("input.wav").with_occlusion(),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.2),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 3.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            ..default()
        })),
        Transform::from_xyz(0.0, 1.5, 0.0),
        AcousticMaterial::new(Vec3::new(2.0, 3.0, 10.0), 300.0, 20.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0),
            ..default()
        })),
        Transform::from_xyz(5.0, 1.0, 0.0),
        Position { x: 5.0, y: 0.0 },
        Velocity { x: 0.0, y: 0.0 },
        Name("Player 1".to_string()),
        AudioListener,
    )).with_child((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.3, 0.0).looking_to(Dir3::NEG_Z, Vec3::Y),
    ));

    commands.spawn((
        Text::new("FPS: --"),
        TextFont {
            font_size: 20.0.into(),
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        FpsText,
    ));
}

/// Movement system updating player position relative to facing orientation.
fn movement_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Position), With<AudioListener>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;

    for (mut transform, mut position) in query.iter_mut() {
        let forward = transform.forward();
        let right = transform.right();

        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            direction += *forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            direction -= *forward;
        }
        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            direction += *right;
        }
        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction -= *right;
        }

        if direction != Vec3::ZERO {
            let speed = 5.0;
            let flat_dir = Vec3::new(direction.x, 0.0, direction.z).normalize();
            let offset = flat_dir * speed * time.delta_secs();
            transform.translation += offset;
            position.x = transform.translation.x;
            position.y = transform.translation.z;
        }
    }
}

/// First-person mouse look system rotating player body (Yaw) and camera pitch.
fn mouse_look_system(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut player_query: Query<&mut Transform, (With<AudioListener>, Without<Camera3d>)>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    let delta = mouse_motion.delta;

    if delta != Vec2::ZERO {
        let sensitivity = 0.002;

        for mut transform in &mut player_query {
            transform.rotate_y(-delta.x * sensitivity);
        }

        for mut transform in &mut camera_query {
            transform.rotate_local_x(-delta.y * sensitivity);
        }
    }
}

/// System for toggling mouse cursor lock mode when pressing Escape.
fn cursor_toggle_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Ok(mut cursor) = cursor_options.single_mut() {
            if cursor.grab_mode == CursorGrabMode::Locked {
                cursor.grab_mode = CursorGrabMode::None;
                cursor.visible = true;
            } else {
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
    }
}

/// Visual debug system changing listener mesh color to yellow when occlusion filtering is active.
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
            mat.base_color = Color::srgb(1.0, 1.0, 0.0);
        } else {
            mat.base_color = Color::srgb(1.0, 1.0, 1.0);
        }
    }
}

/// Diagnostic system updating the on-screen FPS display text.
fn fps_update_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                **text = format!("FPS: {:.0}", value);
            }
        }
    }
}
