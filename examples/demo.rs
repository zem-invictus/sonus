use bevy_sonus::{SonusAudioPlugin, SonusListener};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_skein::SkeinPlugin;

/// Component tag for the on-screen FPS display UI text.
#[derive(Component)]
struct FpsText;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SonusAudioPlugin::default().with_debug(),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
            SkeinPlugin::default(),
        ))
        .add_systems(Startup, setup_game)
        .add_systems(
            Update,
            (
                movement_system,
                mouse_look_system,
                cursor_toggle_system,
                fps_update_system,
            ),
        )
        .run();
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    commands.spawn(WorldAssetRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("scenes/test.glb")),
    ));

    if let Ok(mut cursor) = cursor_options.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(0.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 1.0),
                ..default()
            })),
            Transform::from_xyz(-10.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            SonusListener,
        ))
        .with_child((
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
    mut query: Query<&mut Transform, With<SonusListener>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;

    for mut transform in query.iter_mut() {
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
            let flat_dir = Vec3::new(direction.x, 0.0, direction.z).normalize();
            let delta = flat_dir * 5.0 * time.delta_secs();
            transform.translation += delta;
        }
    }
}

/// First-person mouse look system rotating player body (Yaw) and camera pitch.
fn mouse_look_system(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut player_query: Query<&mut Transform, (With<SonusListener>, Without<Camera3d>)>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    let delta = mouse_motion.delta;
    if delta != Vec2::ZERO {
        let sensitivity = 0.002;

        if let Ok(mut player_transform) = player_query.single_mut() {
            player_transform.rotate_y(-delta.x * sensitivity);
        }

        if let Ok(mut camera_transform) = camera_query.single_mut() {
            let (yaw, mut pitch, roll) = camera_transform.rotation.to_euler(EulerRot::YXZ);
            pitch -= delta.y * sensitivity;
            pitch = pitch.clamp(-1.54, 1.54);
            camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        }
    }
}

/// Cursor lock toggle system on Escape key press.
fn cursor_toggle_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape)
        && let Ok(mut cursor) = cursor_options.single_mut()
    {
        if cursor.grab_mode == CursorGrabMode::Locked {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        } else {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

/// Diagnostic system updating the on-screen FPS display text.
fn fps_update_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(value) = fps.smoothed()
        {
            **text = format!("FPS: {:.0}", value);
        }
    }
}
