//! Bevy ECS integration components, systems, and plugins for spatial audio.

use crate::sonus::config::{
    AttenuationControl, AttenuationModel, AudioParam, OcclusionControl, SonusControl,
};
use crate::sonus::source::SonusSource;
use bevy::app::App;
use bevy::asset::Handle;
use bevy::audio::{AddAudioSource, AudioSource};
use bevy::math::bounding::{Aabb2d, RayCast2d};
use bevy::prelude::*;
use std::sync::Arc;

/// Marker component for the active spatial audio listener entity.
#[derive(Component)]
pub struct AudioListener;

/// Physical acoustic properties of an obstacle entity.
#[derive(Component, Clone, Copy, Debug)]
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

/// System for computing raycast intersections between audio emitters and acoustic obstacles.
pub fn sonus_occlusion_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<AudioListener>>,
    wall_query: Query<(&Transform, &AcousticMaterial)>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation.xz();

    for (emitter_transform, emitter) in emitter_query.iter() {
        let Some(occlusion_control) = &emitter.control.occlusion_control else {
            continue;
        };

        let emitter_pos = emitter_transform.translation.xz();
        let delta = listener_pos - emitter_pos;
        let max_dist = delta.length();

        let Ok(dir) = Dir2::new(delta) else { continue };
        let ray = RayCast2d::new(emitter_pos, dir, max_dist);

        let mut target_low = 1.0f32;
        let mut target_mid = 1.0f32;
        let mut target_high = 1.0f32;

        for (wall_transform, material) in wall_query.iter() {
            let wall_pos = wall_transform.translation.xz();
            let wall_half_extent = material.half_extends.xz();

            let aabb = Aabb2d::new(wall_pos, wall_half_extent);

            if let Some(hit_dist) = ray.aabb_intersection_at(&aabb)
                && hit_dist <= max_dist
            {
                target_low *= material.low_transmission;
                target_mid *= material.mid_transmission;
                target_high *= material.high_transmission;
            }
        }

        if (occlusion_control.gain_low.get() - target_low).abs() > 0.0001 {
            occlusion_control.gain_low.set(target_low);
        }
        if (occlusion_control.gain_mid.get() - target_mid).abs() > 0.0001 {
            occlusion_control.gain_mid.set(target_mid);
        }
        if (occlusion_control.gain_high.get() - target_high).abs() > 0.0001 {
            occlusion_control.gain_high.set(target_high);
        }
    }
}

/// System for computing distance-based audio attenuation and updating target volume gain.
pub fn sonus_attenuation_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<AudioListener>>,
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

/// Bevy plugin registering spatial audio components and processing systems.
pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SonusSource>().add_systems(
            Update,
            (
                sonus_audio_system,
                sonus_occlusion_system,
                sonus_attenuation_system,
            ),
        );
    }
}
