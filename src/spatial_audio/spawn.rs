use crate::SpatialAudioController;
use crate::spatial_audio::source::SonusSource;
use bevy::prelude::*;

#[derive(Event)]
pub struct SpawnSound {
    pub position: Vec3,
    pub sound: Handle<AudioSource>,
}

#[derive(Component)]
pub(crate) struct SpatialAudioIntent {
    source: Handle<AudioSource>,
    lowpass_filter: f32,
}

pub(crate) fn on_spawn_sound(
    trigger: On<SpawnSound>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.8),
            ..default()
        })),
        Transform::from_translation(trigger.position),
        SpatialAudioIntent {
            source: trigger.sound.clone(),
            lowpass_filter: 400.0,
        },
    ));
}

pub(crate) fn process_spatial_audio_intents(
    mut commands: Commands,
    query: Query<(Entity, &SpatialAudioIntent)>,
    assets: Res<Assets<AudioSource>>,
    mut spatial_assets: ResMut<Assets<SonusSource>>,
) {
    for (entity, intent) in &query {
        let Some(source) = assets.get(&intent.source) else {
            continue;
        };
        let (spatial_handle, spatial_control) = SonusSource::from_audio_source(source)
            .with_lowpass_filter(intent.lowpass_filter)
            .prepare(spatial_assets.as_mut());
        commands
            .entity(entity)
            .insert((
                AudioPlayer(spatial_handle),
                SpatialAudioController {
                    control: spatial_control,
                },
            ))
            .remove::<SpatialAudioIntent>();
    }
}
