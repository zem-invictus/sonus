use crate::spatial_audio::config::{AudioParam, OcclusionControl, SonusControl};
use crate::spatial_audio::source::SonusSource;
use bevy::app::App;
use bevy::asset::Handle;
use bevy::audio::{AddAudioSource, AudioSource};
use bevy::prelude::*;
use std::sync::Arc;

// === КОМПОНЕНТЫ ===

#[derive(Component)]
pub struct AudioListener;

#[derive(Component)]
pub struct Wall {
    pub half_extents: Vec3,
}

/// Акустический материал стены, задающий частоты среза при окклюзии
#[derive(Component, Clone, Copy, Debug)]
pub struct AcousticMaterial {
    pub lowpass_cutoff_hz: f32,
    pub highpass_cutoff_hz: f32,
}

impl Default for AcousticMaterial {
    fn default() -> Self {
        Self {
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

pub fn line_segment_intersects_aabb(p1: Vec2, p2: Vec2, min: Vec2, max: Vec2) -> bool {
    let d = p2 - p1;
    let mut t_min = 0.0f32;
    let mut t_max = 1.0f32;

    if d.x.abs() < 1e-6 {
        if p1.x < min.x || p1.x > max.x {
            return false;
        }
    } else {
        let ood = 1.0 / d.x;
        let mut t1 = (min.x - p1.x) * ood;
        let mut t2 = (max.x - p1.x) * ood;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return false;
        }
    }

    if d.y.abs() < 1e-6 {
        if p1.y < min.y || p1.y > max.y {
            return false;
        }
    } else {
        let ood = 1.0 / d.y;
        let mut t1 = (min.y - p1.y) * ood;
        let mut t2 = (max.y - p1.y) * ood;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return false;
        }
    }

    true
}

pub fn audio_occlusion_system(
    emitter_query: Query<(&Transform, &SonusEmitter)>,
    listener_query: Query<&Transform, With<AudioListener>>,
    wall_query: Query<(&Transform, &Wall, &AcousticMaterial)>,
    time: Res<Time>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation;

    for (emitter_transform, emitter) in emitter_query.iter() {
        let emitter_pos = emitter_transform.translation;

        let mut target_lpf = 20000.0f32;
        let mut target_hpf = 20.0f32;

        if let Some(occlusion_control) = &emitter.control.occlusion_control {
            for (wall_transform, wall, material) in wall_query.iter() {
                let wall_pos = wall_transform.translation;
                let min = Vec2::new(
                    wall_pos.x - wall.half_extents.x,
                    wall_pos.z - wall.half_extents.z,
                );
                let max = Vec2::new(
                    wall_pos.x + wall.half_extents.x,
                    wall_pos.z + wall.half_extents.z,
                );

                let p1 = Vec2::new(emitter_pos.x, emitter_pos.z);
                let p2 = Vec2::new(listener_pos.x, listener_pos.z);

                if line_segment_intersects_aabb(p1, p2, min, max) {
                    target_lpf = target_lpf.min(material.lowpass_cutoff_hz);
                    target_hpf = target_hpf.max(material.highpass_cutoff_hz);
                }
            }

            let dt = time.delta_secs();
            let lerp_factor = (8.0 * dt).min(1.0);

            let current_lpf = occlusion_control.lowpass_hz.get();
            let new_lpf = current_lpf + (target_lpf - current_lpf) * lerp_factor;
            occlusion_control.lowpass_hz.set(new_lpf.clamp(20.0, 20000.0));

            let current_hpf = occlusion_control.highpass_hz.get();
            let new_hpf = current_hpf + (target_hpf - current_hpf) * lerp_factor;
            occlusion_control.highpass_hz.set(new_hpf.clamp(20.0, 20000.0));
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