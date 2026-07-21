use crate::spatial_audio::config::{AudioParam, OcclusionControl, SonusControl};
use crate::spatial_audio::source::SonusSource;
use bevy::app::App;
use bevy::asset::Handle;
use bevy::audio::{AddAudioSource, AudioSource};
use bevy::math::bounding::{Aabb2d, RayCast2d};
use bevy::prelude::*;
use std::sync::Arc;

// === КОМПОНЕНТЫ ===

#[derive(Component)]
pub struct AudioListener;

/// Акустический материал стены, задающий размеры и частоты среза при окклюзии
#[derive(Component, Clone, Copy, Debug)]
pub struct AcousticMaterial {
    pub half_extends: Vec3,
    pub lowpass_cutoff_hz: f32,
    pub highpass_cutoff_hz: f32,
}

impl AcousticMaterial {
    pub fn new(size: Vec3, lowpass_cutoff_hz: f32, highpass_cutoff_hz: f32) -> Self {
        Self {
            half_extends: size * 0.5,
            lowpass_cutoff_hz,
            highpass_cutoff_hz,
        }
    }
}

impl Default for AcousticMaterial {
    fn default() -> Self {
        Self {
            half_extends: Vec3::splat(0.5),
            lowpass_cutoff_hz: 20000.0,
            highpass_cutoff_hz: 20.0,
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

#[derive(Component)]
pub struct SonusEmitter {
    pub(crate) source: SonusSourceInput,
    pub(crate) control: Arc<SonusControl>,
    pub(crate) use_occlusion: bool,
}

impl SonusEmitter {
    pub fn new(source: impl Into<SonusSourceInput>) -> Self {
        Self {
            source: source.into(),
            control: Arc::new(SonusControl::new()),
            use_occlusion: false,
        }
    }

    pub(crate) fn update_handle_status(&mut self, source: impl Into<SonusSourceInput>) {
        self.source = source.into();
    }

    pub fn with_occlusion(mut self) -> Self {
        self.use_occlusion = true;
        self.control = Arc::new(SonusControl {
            occlusion_control: Some(Arc::new(OcclusionControl {
                lowpass_hz: AudioParam::new(20000.0),
                highpass_hz: AudioParam::new(20.0),
            })),
        });
        self
    }
}

// === ECS СИСТЕМЫ ===

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

pub fn audio_occlusion_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<AudioListener>>,
    wall_query: Query<(&Transform, &AcousticMaterial)>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation.xz();

    for (emitter_transform, emitter) in emitter_query.iter() {
        let emitter_pos = emitter_transform.translation.xz();

        let delta = listener_pos - emitter_pos;
        let max_dist = delta.length();

        let Ok(dir) = Dir2::new(delta) else { continue };

        let ray = RayCast2d::new(emitter_pos, dir, max_dist);

        let mut target_lpf = 20000.0f32;
        let mut target_hpf = 20.0f32;

        if let Some(occlusion_control) = &emitter.control.occlusion_control {
            for (wall_transform, material) in wall_query.iter() {
                let wall_pos = wall_transform.translation.xz();
                let wall_half_extent = material.half_extends.xz();

                let aabb = Aabb2d::new(wall_pos, wall_half_extent);

                if let Some(hit_dist) = ray.aabb_intersection_at(&aabb)
                    && hit_dist <= max_dist
                {
                    target_lpf = target_lpf.min(material.lowpass_cutoff_hz);
                    target_hpf = target_hpf.max(material.highpass_cutoff_hz);
                }
            }

            // Мгновенная установка целевых частот в атомики при изменении состояния
            let current_lpf = occlusion_control.lowpass_hz.get();
            if current_lpf != target_lpf {
                occlusion_control.lowpass_hz.set(target_lpf);
            }

            let current_hpf = occlusion_control.highpass_hz.get();
            if current_hpf != target_hpf {
                occlusion_control.highpass_hz.set(target_hpf);
            }
        }
    }
}

// === PLUG-IN ===

pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SonusSource>()
            .add_systems(Update, (sonus_audio_system, audio_occlusion_system));
    }
}
