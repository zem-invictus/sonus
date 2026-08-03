//! Bevy ECS integration components, systems, and plugins for spatial audio.

use crate::config::{
    AttenuationControl, AttenuationModel, AudioParam, OcclusionControl, PanningControl,
    SonusControl,
};
use crate::source::SonusSource;
use bevy::app::App;
use bevy::asset::Handle;
use bevy::audio::{AddAudioSource, AudioSource};
use bevy::camera::primitives::Aabb;
use bevy::math::bounding::{Aabb3d, RayCast3d};
use bevy::prelude::*;
use std::sync::Arc;

/// Marker component for the active spatial audio listener entity.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
pub struct SonusListener;

/// Preset materials with predefined acoustic transmission properties across low, mid, and high frequency bands.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
#[reflect(Component, Default)]
pub enum AcousticMaterialPreset {
    #[default]
    Wood,
    Stone,
    Concrete,
    Glass,
    Metal,
    ThinPlaster,
}

impl AcousticMaterialPreset {
    /// Returns the acoustic material properties corresponding to this preset.
    pub fn properties(&self) -> AcousticMaterial {
        match self {
            Self::Wood => AcousticMaterial {
                low_transmission: 0.7,
                mid_transmission: 0.4,
                high_transmission: 0.2,
            },
            Self::Stone => AcousticMaterial {
                low_transmission: 0.4,
                mid_transmission: 0.1,
                high_transmission: 0.02,
            },
            Self::Concrete => AcousticMaterial {
                low_transmission: 0.2,
                mid_transmission: 0.05,
                high_transmission: 0.01,
            },
            Self::Glass => AcousticMaterial {
                low_transmission: 0.9,
                mid_transmission: 0.7,
                high_transmission: 0.5,
            },
            Self::Metal => AcousticMaterial {
                low_transmission: 0.3,
                mid_transmission: 0.08,
                high_transmission: 0.02,
            },
            Self::ThinPlaster => AcousticMaterial {
                low_transmission: 0.85,
                mid_transmission: 0.6,
                high_transmission: 0.3,
            },
        }
    }
}

impl From<AcousticMaterialPreset> for AcousticMaterial {
    fn from(preset: AcousticMaterialPreset) -> Self {
        preset.properties()
    }
}

/// Physical acoustic properties of an obstacle entity.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct AcousticMaterial {
    pub low_transmission: f32,
    pub mid_transmission: f32,
    pub high_transmission: f32,
}

impl AcousticMaterial {
    /// Creates a new acoustic material with defined 3-band transmission coefficients.
    pub fn new(low_transmission: f32, mid_transmission: f32, high_transmission: f32) -> Self {
        Self {
            low_transmission,
            mid_transmission,
            high_transmission,
        }
    }

    /// Creates an acoustic material from a predefined preset.
    pub fn preset(preset: AcousticMaterialPreset) -> Self {
        preset.into()
    }

    pub fn wood() -> Self {
        Self::preset(AcousticMaterialPreset::Wood)
    }

    pub fn stone() -> Self {
        Self::preset(AcousticMaterialPreset::Stone)
    }

    pub fn concrete() -> Self {
        Self::preset(AcousticMaterialPreset::Concrete)
    }

    pub fn glass() -> Self {
        Self::preset(AcousticMaterialPreset::Glass)
    }

    pub fn metal() -> Self {
        Self::preset(AcousticMaterialPreset::Metal)
    }

    pub fn thin_plaster() -> Self {
        Self::preset(AcousticMaterialPreset::ThinPlaster)
    }
}

impl Default for AcousticMaterial {
    fn default() -> Self {
        Self {
            low_transmission: 1.0,
            mid_transmission: 1.0,
            high_transmission: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SonusSourceInput {
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
            perceived_dir_x: AudioParam::new(0.0),
            perceived_dir_y: AudioParam::new(0.0),
            perceived_dir_z: AudioParam::new(0.0),
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

/// Helper function for resolving AABB component from an entity or its children.
fn resolve_aabb(
    self_aabb: Option<&Aabb>,
    children: Option<&Children>,
    mesh_aabb_query: &Query<&Aabb>,
) -> Option<Aabb> {
    if let Some(aabb) = self_aabb {
        return Some(*aabb);
    }
    if let Some(children) = children {
        for &child in children {
            if let Ok(child_aabb) = mesh_aabb_query.get(child) {
                return Some(*child_aabb);
            }
        }
    }
    None
}

/// Helper function to compute a 5-ray cross pattern facing the target.
fn compute_cross_pattern_rays(origin: Vec3, target: Vec3, radius: f32) -> [Vec3; 5] {
    let delta = target - origin;
    let dir_to_target = delta.normalize_or_zero();
    let right = dir_to_target.cross(Vec3::Y).normalize_or_zero();
    let up = dir_to_target.cross(right).normalize_or_zero();

    [
        origin,
        origin + right * radius,
        origin - right * radius,
        origin + up * radius,
        origin - up * radius,
    ]
}

/// Helper function to count ray intersections against a local AABB obstacle.
fn count_wall_hits(
    rays: &[Vec3],
    local_listener: Vec3,
    local_aabb: &Aabb3d,
) -> usize {
    let mut hits = 0;
    for &ray in rays {
        let local_delta = local_listener - ray;
        let local_dist = local_delta.length();
        if let Ok(local_dir) = Dir3::new(local_delta) {
            let local_ray = RayCast3d::new(ray, local_dir, local_dist);
            if let Some(hit_dist) = local_ray.aabb_intersection_at(local_aabb)
                && hit_dist <= local_dist
            {
                hits += 1;
            }
        }
    }
    hits
}

/// System for computing raycast intersections in the obstacle's local coordinate space.
pub fn sonus_occlusion_system(
    time: Res<Time>,
    emitter_query: Query<(
        &GlobalTransform,
        &SonusEmitter,
        Option<&Children>,
        Option<&Aabb>,
    )>,
    listener_query: Query<&GlobalTransform, With<SonusListener>>,
    wall_query: Query<(
        &GlobalTransform,
        &AcousticMaterial,
        Option<&Children>,
        Option<&Aabb>,
    )>,
    mesh_aabb_query: Query<&Aabb>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation();

    let dt = time.delta_secs();
    let smooth_factor = (dt * 10.0).min(1.0);

    for (emitter_transform, emitter, emitter_children, self_emitter_aabb) in emitter_query.iter() {
        let Some(occlusion_control) = &emitter.control.occlusion_control else {
            continue;
        };

        let emitter_pos = emitter_transform.translation();

        let resolved_emitter_aabb =
            resolve_aabb(self_emitter_aabb, emitter_children, &mesh_aabb_query);

        let emitter_radius = resolved_emitter_aabb
            .map(|a| a.half_extents.max_element())
            .unwrap_or(0.5);

        let mut target_low = 1.0f32;
        let mut target_mid = 1.0f32;
        let mut target_high = 1.0f32;

        let rays_world = compute_cross_pattern_rays(emitter_pos, listener_pos, emitter_radius);
        let mut ray_weights = [1.0f32; 5];

        for (wall_transform, material, children, self_aabb) in wall_query.iter() {
            let Some(aabb) = resolve_aabb(self_aabb, children, &mesh_aabb_query) else {
                continue;
            };

            let center: Vec3 = aabb.center.into();
            let half_extends: Vec3 = aabb.half_extents.into();
            let local_aabb = Aabb3d::new(center, half_extends);

            let inv_matrix = wall_transform.to_matrix().inverse();
            let local_listener = inv_matrix.transform_point3(listener_pos);

            let mut wall_hits = 0;
            for (i, &ray_world_origin) in rays_world.iter().enumerate() {
                let local_ray_origin = inv_matrix.transform_point3(ray_world_origin);
                let local_delta = local_listener - local_ray_origin;
                let local_dist = local_delta.length();

                if let Ok(local_dir) = Dir3::new(local_delta) {
                    let local_ray = RayCast3d::new(local_ray_origin, local_dir, local_dist);
                    if let Some(hit_dist) = local_ray.aabb_intersection_at(&local_aabb)
                        && hit_dist <= local_dist
                    {
                        wall_hits += 1;
                        ray_weights[i] *= material.mid_transmission;
                    }
                }
            }

            if wall_hits > 0 {
                let obstruction_ratio = wall_hits as f32 / rays_world.len() as f32;

                target_low *= 1.0f32.lerp(material.low_transmission, obstruction_ratio);
                target_mid *= 1.0f32.lerp(material.mid_transmission, obstruction_ratio);
                target_high *= 1.0f32.lerp(material.high_transmission, obstruction_ratio);
            }
        }

        let mut weighted_dir_sum = Vec3::ZERO;
        let mut total_weight = 0.0f32;
        for (i, &ray_world_origin) in rays_world.iter().enumerate() {
            let weight = ray_weights[i];
            let dir_from_listener = (ray_world_origin - listener_pos).normalize_or_zero();
            weighted_dir_sum += dir_from_listener * weight;
            total_weight += weight;
        }

        let perceived_dir = if total_weight > 0.001 {
            weighted_dir_sum.normalize_or_zero()
        } else {
            (emitter_pos - listener_pos).normalize_or_zero()
        };

        occlusion_control.perceived_dir_x.set(perceived_dir.x);
        occlusion_control.perceived_dir_y.set(perceived_dir.y);
        occlusion_control.perceived_dir_z.set(perceived_dir.z);

        let prev_low = occlusion_control.gain_low.get();
        let prev_mid = occlusion_control.gain_mid.get();
        let prev_high = occlusion_control.gain_high.get();

        let next_low = prev_low + (target_low - prev_low) * smooth_factor;
        let next_mid = prev_mid + (target_mid - prev_mid) * smooth_factor;
        let next_high = prev_high + (target_high - prev_high) * smooth_factor;

        occlusion_control.gain_low.set(next_low);
        occlusion_control.gain_mid.set(next_mid);
        occlusion_control.gain_high.set(next_high);
    }
}

/// System for computing distance-based audio attenuation and updating target volume gain.
pub fn sonus_attenuation_system(
    emitter_query: Query<(&GlobalTransform, &SonusEmitter)>,
    listener_query: Query<&GlobalTransform, With<SonusListener>>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };

    for (emitter_transform, emitter) in emitter_query.iter() {
        let Some(attenuation_control) = &emitter.control.attenuation_control else {
            continue;
        };

        let dist = listener_transform
            .translation()
            .distance(emitter_transform.translation());

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
    emitter_query: Query<(&GlobalTransform, &SonusEmitter)>,
    listener_query: Query<&GlobalTransform, With<SonusListener>>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };

    let listener_pos = listener_transform.translation();
    let list_right = listener_transform.right();

    for (emitter_transform, emitter) in emitter_query.iter() {
        let Some(panning_control) = &emitter.control.panning_control else {
            continue;
        };

        let perceived_dir = if let Some(occlusion_control) = &emitter.control.occlusion_control {
            let px = occlusion_control.perceived_dir_x.get();
            let py = occlusion_control.perceived_dir_y.get();
            let pz = occlusion_control.perceived_dir_z.get();
            let dir = Vec3::new(px, py, pz);
            if dir.length_squared() > 0.001 {
                dir.normalize()
            } else {
                (emitter_transform.translation() - listener_pos).normalize_or_zero()
            }
        } else {
            (emitter_transform.translation() - listener_pos).normalize_or_zero()
        };

        let dist = listener_pos.distance(emitter_transform.translation());
        const MIN_FAR_EAR_GAIN: f32 = 0.25;

        let (_pan, left_gain, right_gain) = if dist < 0.001 {
            let center_gain = MIN_FAR_EAR_GAIN.lerp(1.0, std::f32::consts::FRAC_1_SQRT_2);
            (0.0, center_gain, center_gain)
        } else {
            let pan = perceived_dir.dot(*list_right);
            let normalized_pan = (pan + 1.0) / 2.0;

            let left_gain =
                MIN_FAR_EAR_GAIN.lerp(1.0, (1.0 - normalized_pan).sqrt().clamp(0.0, 1.0));
            let right_gain = MIN_FAR_EAR_GAIN.lerp(1.0, normalized_pan.sqrt().clamp(0.0, 1.0));

            (pan, left_gain, right_gain)
        };

        let current_left = panning_control.left_gain.get();
        let current_right = panning_control.right_gain.get();

        if (current_left - left_gain).abs() > 0.001 || (current_right - right_gain).abs() > 0.001 {
            panning_control.left_gain.set(left_gain);
            panning_control.right_gain.set(right_gain);
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

/// System detecting new `AcousticMaterialPreset` components and instantiating `AcousticMaterial` components.
pub fn sonus_material_preset_system(
    mut commands: Commands,
    query: Query<(Entity, &AcousticMaterialPreset), Added<AcousticMaterialPreset>>,
) {
    for (entity, preset) in query.iter() {
        commands
            .entity(entity)
            .insert(AcousticMaterial::from(*preset));
    }
}

/// System for rendering 3D spatial audio debug gizmos (attenuation spheres, raycast lines, wall AABBs).
pub fn sonus_debug_gizmos_system(
    mut gizmos: Gizmos,
    emitter_query: Query<(
        &GlobalTransform,
        &SonusEmitter,
        Option<&Children>,
        Option<&Aabb>,
    )>,
    listener_query: Query<&GlobalTransform, With<SonusListener>>,
    wall_query: Query<(
        &GlobalTransform,
        &AcousticMaterial,
        Option<&Children>,
        Option<&Aabb>,
    )>,
    mesh_aabb_query: Query<&Aabb>,
) {
    let Some(listener_transform) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation();

    for (wall_transform, _material, children, self_aabb) in wall_query.iter() {
        let Some(aabb) = resolve_aabb(self_aabb, children, &mesh_aabb_query) else {
            continue;
        };

        let center: Vec3 = aabb.center.into();
        let half_extents: Vec3 = aabb.half_extents.into();

        let world_center = wall_transform.to_matrix().transform_point3(center);
        let (wall_scale, wall_rot, _) = wall_transform.to_scale_rotation_translation();

        gizmos.primitive_3d(
            &Cuboid::from_size(half_extents * 2.0 * wall_scale),
            Isometry3d::new(world_center, wall_rot),
            Color::srgb(0.7, 0.2, 0.8),
        );
    }

    for (emitter_transform, emitter, emitter_children, self_emitter_aabb) in emitter_query.iter() {
        let emitter_pos = emitter_transform.translation();

        if let Some(attenuation_control) = &emitter.control.attenuation_control
            && let AttenuationModel::Linear { min_dist, max_dist } = attenuation_control.model
        {
            gizmos.sphere(
                Isometry3d::from_translation(emitter_pos),
                min_dist,
                Color::srgb(0.0, 1.0, 0.0),
            );
            gizmos.sphere(
                Isometry3d::from_translation(emitter_pos),
                max_dist,
                Color::srgb(1.0, 0.8, 0.0),
            );
        }

        let mut max_obstruction_ratio = 0.0f32;
        if emitter.control.occlusion_control.is_some() {
            let resolved_emitter_aabb =
                resolve_aabb(self_emitter_aabb, emitter_children, &mesh_aabb_query);

            let emitter_radius = resolved_emitter_aabb
                .map(|a| a.half_extents.max_element())
                .unwrap_or(0.5);

            for (wall_transform, _material, children, self_aabb) in wall_query.iter() {
                let Some(aabb) = resolve_aabb(self_aabb, children, &mesh_aabb_query) else {
                    continue;
                };

                let center: Vec3 = aabb.center.into();
                let half_extents: Vec3 = aabb.half_extents.into();
                let local_aabb = Aabb3d::new(center, half_extents);

                let inv_matrix = wall_transform.to_matrix().inverse();
                let local_emitter = inv_matrix.transform_point3(emitter_pos);
                let local_listener = inv_matrix.transform_point3(listener_pos);

                let rays =
                    compute_cross_pattern_rays(local_emitter, local_listener, emitter_radius);
                let wall_hits = count_wall_hits(&rays, local_listener, &local_aabb);

                let ratio = wall_hits as f32 / rays.len() as f32;
                if ratio > max_obstruction_ratio {
                    max_obstruction_ratio = ratio;
                }
            }
        }

        let line_color = if max_obstruction_ratio == 0.0 {
            Color::srgb(0.0, 1.0, 0.0) // Clear: Green
        } else if max_obstruction_ratio >= 1.0 {
            Color::srgb(1.0, 0.0, 0.0) // Fully blocked: Red
        } else {
            // Partially blocked: Orange / Yellow gradient
            Color::srgb(1.0, 0.6, 0.0)
        };
        gizmos.line(emitter_pos, listener_pos, line_color);
    }
}

/// Bevy plugin registering spatial audio components and processing systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SonusAudioPlugin {
    pub debug: bool,
}

impl SonusAudioPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }
}

impl Plugin for SonusAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<SonusSource>().add_systems(
            Update,
            (
                sonus_emitter_config_system,
                sonus_material_preset_system,
                sonus_audio_system,
                sonus_occlusion_system,
                sonus_attenuation_system,
                sonus_panning_system,
            ),
        );

        if self.debug {
            app.add_systems(Update, sonus_debug_gizmos_system);
        }
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

    #[test]
    fn test_acoustic_material_presets() {
        let wood = AcousticMaterial::wood();
        let wood_from_preset: AcousticMaterial = AcousticMaterialPreset::Wood.into();
        assert_eq!(wood.low_transmission, wood_from_preset.low_transmission);
        assert_eq!(wood.mid_transmission, wood_from_preset.mid_transmission);
        assert_eq!(wood.high_transmission, wood_from_preset.high_transmission);

        let concrete = AcousticMaterial::concrete();
        assert_eq!(concrete.low_transmission, 0.2);
    }

    #[test]
    fn test_sonus_material_preset_spawning() {
        let mut app = App::new();
        app.add_systems(Update, sonus_material_preset_system);

        let entity = app.world_mut().spawn(AcousticMaterialPreset::Stone).id();
        app.update();

        let mat = app.world().entity(entity).get::<AcousticMaterial>();
        assert!(mat.is_some());
        assert_eq!(mat.unwrap().low_transmission, 0.4);
    }

    #[test]
    fn test_diffraction_panning_bending() {
        let mut app = App::new();
        app.add_systems(Update, sonus_panning_system);

        let listener_transform = Transform::from_xyz(0.0, 0.0, 0.0);
        app.world_mut().spawn((
            SonusListener,
            listener_transform,
            GlobalTransform::from(listener_transform),
        ));

        let emitter = SonusEmitter::new("sound.wav")
            .with_panning()
            .with_occlusion();

        // Simulate occlusion system setting a bent perceived direction (pointing right: +X)
        if let Some(occ) = &emitter.control.occlusion_control {
            occ.perceived_dir_x.set(1.0);
            occ.perceived_dir_y.set(0.0);
            occ.perceived_dir_z.set(0.0);
        }

        let emitter_transform = Transform::from_xyz(0.0, 0.0, -10.0);
        let emitter_entity = app
            .world_mut()
            .spawn((
                emitter,
                emitter_transform,
                GlobalTransform::from(emitter_transform),
            ))
            .id();

        app.update();

        let emitter_ref = app
            .world()
            .entity(emitter_entity)
            .get::<SonusEmitter>()
            .unwrap();
        let panning = emitter_ref.control.panning_control.as_ref().unwrap();

        // Since perceived direction was bent to +X (Right), right ear gain should be higher than left
        assert!(panning.right_gain.get() > panning.left_gain.get());
    }
}
