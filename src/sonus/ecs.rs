//! Bevy ECS integration components, systems, and plugins for spatial audio.

use std::f32::consts::PI;
use crate::sonus::config::{
    AttenuationControl, AttenuationModel, AudioParam, OcclusionControl, PanningControl,
    SonusControl,
};
use crate::sonus::source::SonusSource;
use bevy::app::App;
use bevy::asset::Handle;
use bevy::audio::{AddAudioSource, AudioSource};
use bevy::math::bounding::{Aabb3d, RayCast3d};
use bevy::prelude::*;
use std::sync::Arc;
use bevy::math::ops::{cos, sin};

/// Marker component for the active spatial audio listener entity.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
pub struct SonusListener;

/// Physical acoustic properties of an obstacle entity.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct AcousticMaterial {
    pub half_extends: Vec3,
    pub low_transmission: f32,
    pub mid_transmission: f32,
    pub high_transmission: f32,
}

impl AcousticMaterial {
    /// Creates a new acoustic material with defined dimensions and 3-band transmission coefficients.
    pub fn new(
        size: Vec3,
        low_transmission: f32,
        mid_transmission: f32,
        high_transmission: f32,
    ) -> Self {
        Self {
            half_extends: size * 0.5,
            low_transmission,
            mid_transmission,
            high_transmission,
        }
    }
}

impl Default for AcousticMaterial {
    fn default() -> Self {
        Self {
            half_extends: Vec3::splat(0.5),
            low_transmission: 1.0,
            mid_transmission: 1.0,
            high_transmission: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SonusSourceInput {
    Path(String),
    AudioHandle(Handle<AudioSource>),
}

impl From<&str> for SonusSourceInput {
    fn from(path: &str) -> Self {
        Self::Path(path.to_string())
    }
}

impl From<String> for SonusSourceInput {
    fn from(path: String) -> Self {
        Self::Path(path)
    }
}

impl From<Handle<AudioSource>> for SonusSourceInput {
    fn from(handle: Handle<AudioSource>) -> Self {
        Self::AudioHandle(handle)
    }
}

/// Emitter component attached to spatial audio sources in the Bevy scene.
#[derive(Component)]
pub struct SonusEmitter {
    pub(crate) source: SonusSourceInput,
    pub(crate) control: Arc<SonusControl>,
}

/// Declarative configuration for spatial sound emitters loaded from Blender / Skein.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct SonusEmitterConfig {
    pub source_path: String,
    pub occlusion: bool,
    pub panning: bool,
    pub min_dist: f32,
    pub max_dist: f32,
}

impl Default for SonusEmitterConfig {
    fn default() -> Self {
        Self {
            source_path: "input.wav".to_string(),
            occlusion: true,
            panning: true,
            min_dist: 2.0,
            max_dist: 20.0,
        }
    }
}

impl SonusEmitter {
    /// Creates a new sound emitter from an asset path or handle.
    pub fn new(source: impl Into<SonusSourceInput>) -> Self {
        Self {
            source: source.into(),
            control: Arc::new(SonusControl::new()),
        }
    }

    pub(crate) fn update_handle_status(&mut self, source: impl Into<SonusSourceInput>) {
        self.source = source.into();
    }

    /// Enables real-time 3-band occlusion filtering for this sound emitter.
    pub fn with_occlusion(mut self) -> Self {
        Arc::make_mut(&mut self.control).occlusion_control = Some(Arc::new(OcclusionControl {
            gain_low: AudioParam::new(1.0),
            gain_mid: AudioParam::new(1.0),
            gain_high: AudioParam::new(1.0),
        }));
        self
    }

    /// Enables real-time distance attenuation with a specified attenuation model.
    pub fn with_attenuation(mut self, model: AttenuationModel) -> Self {
        Arc::make_mut(&mut self.control).attenuation_control = Some(Arc::new(AttenuationControl {
            model,
            gain: AudioParam::new(1.0),
        }));
        self
    }

    pub fn with_panning(mut self) -> Self {
        Arc::make_mut(&mut self.control).panning_control = Some(Arc::new(PanningControl {
            right_gain: AudioParam::new(0.5),
            left_gain: AudioParam::new(0.5),
        }));
        self
    }
}

/// System for instantiating and attaching custom `SonusSource` audio players to entities.
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

        commands.entity(entity).insert(AudioPlayer(sonus_handle));
    }
}

/// System for computing raycast intersections in the obstacle's local coordinate space.
pub fn sonus_occlusion_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<SonusListener>>,
    wall_query: Query<(&Transform, &AcousticMaterial)>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation;

    for (emitter_transform, emitter) in emitter_query.iter() {
        let Some(occlusion_control) = &emitter.control.occlusion_control else {
            continue;
        };

        let emitter_pos = emitter_transform.translation;

        let mut target_low = 1.0f32;
        let mut target_mid = 1.0f32;
        let mut target_high = 1.0f32;

        let mut hit_count = 0;

        for (wall_transform, material) in wall_query.iter() {
            // Inverse world matrix converts world coordinates to obstacle local space
            let inv_matrix = wall_transform.to_matrix().inverse();

            let local_emitter = inv_matrix.transform_point3(emitter_pos);
            let local_listener = inv_matrix.transform_point3(listener_pos);

            let local_delta = local_listener - local_emitter;
            let local_dist = local_delta.length();

            let Ok(local_dir) = Dir3::new(local_delta) else {
                continue;
            };
            let local_ray = RayCast3d::new(local_emitter, local_dir, local_dist);

            // Local AABB is centered at (0, 0, 0) in the obstacle's coordinate system
            let local_aabb = Aabb3d::new(Vec3::ZERO, material.half_extends);

            if let Some(hit_dist) = local_ray.aabb_intersection_at(&local_aabb)
                && hit_dist <= local_dist
            {
                hit_count += 1;
                target_low *= material.low_transmission;
                target_mid *= material.mid_transmission;
                target_high *= material.high_transmission;
            }
        }

        let prev_low = occlusion_control.gain_low.get();
        let prev_mid = occlusion_control.gain_mid.get();
        let prev_high = occlusion_control.gain_high.get();

        if (prev_low - target_low).abs() > 0.01
            || (prev_mid - target_mid).abs() > 0.01
            || (prev_high - target_high).abs() > 0.01
        {
            occlusion_control.gain_low.set(target_low);
            occlusion_control.gain_mid.set(target_mid);
            occlusion_control.gain_high.set(target_high);

            info!(
                "[Occlusion State Changed] Hits: {} | Targets: Low={:.2}, Mid={:.2}, High={:.2}",
                hit_count, target_low, target_mid, target_high
            );
        }
    }
}

/// System for computing distance-based audio attenuation and updating target volume gain.
pub fn sonus_attenuation_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<SonusListener>>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };

    for (emitter_transform, emitter) in emitter_query.iter() {
        let Some(attenuation_control) = &emitter.control.attenuation_control else {
            continue;
        };

        let dist = listener_transform
            .translation
            .distance(emitter_transform.translation);

        let target_gain = match attenuation_control.model {
            AttenuationModel::None => 1.0,
            AttenuationModel::Linear { min_dist, max_dist } => {
                if dist <= min_dist {
                    1.0
                } else if dist >= max_dist {
                    0.0
                } else {
                    1.0 - (dist - min_dist) / (max_dist - min_dist)
                }
            }
            AttenuationModel::InverseDistance {
                ref_dist,
                rolloff_factor,
                max_dist,
            } => {
                if dist >= max_dist {
                    0.0
                } else if dist <= ref_dist {
                    1.0
                } else {
                    ref_dist / (ref_dist + rolloff_factor * (dist - ref_dist))
                }
            }
        };

        let current_gain = attenuation_control.gain.get();
        if (current_gain - target_gain).abs() > 0.0001 {
            attenuation_control.gain.set(target_gain);
        }
    }
}

pub fn sonus_panning_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<SonusListener>>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };

    let listener_pos = listener_transform.translation;
    let list_right = listener_transform.right().as_vec3();

    for (emitter_transform, emitter) in emitter_query.iter() {
        let Some(panning_control) = &emitter.control.panning_control else {
            continue;
        };

        let to_emitter = emitter_transform.translation - listener_pos;
        let dist = to_emitter.length();

        const MIN_FAR_EAR_GAIN: f32 = 0.25;

        let (pan, left_gain, right_gain) = if dist < 0.001 {
            let center_gain = MIN_FAR_EAR_GAIN.lerp(1.0, std::f32::consts::FRAC_1_SQRT_2);
            (0.0, center_gain, center_gain)
        } else {
            let dir = to_emitter / dist;
            let pan = dir.dot(list_right).clamp(-1.0, 1.0);
            let phi = (pan + 1.0) * PI * 0.25;
            let raw_left = cos(phi);
            let raw_right = sin(phi);
            let left = MIN_FAR_EAR_GAIN.lerp(1.0, raw_left);
            let right = MIN_FAR_EAR_GAIN.lerp(1.0, raw_right);
            (pan, left, right)
        };

        let prev_left = panning_control.left_gain.get();
        let prev_right = panning_control.right_gain.get();

        if (prev_left - left_gain).abs() > 0.05 || (prev_right - right_gain).abs() > 0.05 {
            panning_control.left_gain.set(left_gain);
            panning_control.right_gain.set(right_gain);

            info!(
                "[Panning] Pos L: {:.1?}, E: {:.1?} | Pan: {:.2} | Left: {:.2}, Right: {:.2}",
                listener_pos, emitter_transform.translation, pan, left_gain, right_gain
            );
        }
    }
}

/// System detecting new `SonusEmitterConfig` components and instantiating runtime `SonusEmitter` components.
pub fn sonus_emitter_config_system(
    mut commands: Commands,
    query: Query<(Entity, &SonusEmitterConfig), Added<SonusEmitterConfig>>,
) {
    for (entity, config) in query.iter() {
        let mut emitter = SonusEmitter::new(&*config.source_path);
        if config.occlusion {
            emitter = emitter.with_occlusion();
        }
        if config.panning {
            emitter = emitter.with_panning();
        }
        if config.max_dist > 0.0 {
            emitter = emitter.with_attenuation(AttenuationModel::Linear {
                min_dist: config.min_dist,
                max_dist: config.max_dist,
            });
        }
        commands.entity(entity).insert(emitter);
    }
}

/// Bevy plugin registering spatial audio components and processing systems.
pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AcousticMaterial>()
            .register_type::<SonusListener>()
            .register_type::<SonusEmitterConfig>()
            .add_audio_source::<SonusSource>()
            .add_systems(
                Update,
                (
                    sonus_emitter_config_system,
                    sonus_audio_system,
                    sonus_occlusion_system,
                    sonus_attenuation_system,
                    sonus_panning_system,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sonus_emitter_config_spawning() {
        let mut app = App::new();
        app.add_systems(Update, sonus_emitter_config_system);

        let entity = app
            .world_mut()
            .spawn(SonusEmitterConfig {
                source_path: "input.wav".to_string(),
                occlusion: true,
                panning: true,
                min_dist: 2.0,
                max_dist: 20.0,
            })
            .id();

        app.update();

        assert!(app.world().entity(entity).contains::<SonusEmitter>());
    }
}
