use crate::{AudioListener, SpatialAudioController, Wall};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{Query, Res, ResMut, Time, Transform, Vec2, With};

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
    emitter_query: Query<(&Transform, &SpatialAudioController)>,
    listener_query: Query<(&Transform, &MeshMaterial3d<StandardMaterial>), With<AudioListener>>,
    wall_query: Query<(&Transform, &Wall)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let Some((listener_transform, material_handle)) = listener_query.iter().next() else {
        return;
    };
    let listener_pos = listener_transform.translation;

    for (emitter_transform, emitter) in emitter_query.iter() {
        let emitter_pos = emitter_transform.translation;
        let mut occluded = false;

        for (wall_transform, wall) in wall_query.iter() {
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

            if crate::spatial_audio::occlusion::line_segment_intersects_aabb(p1, p2, min, max) {
                occluded = true;
                break;
            }
        }

        if let Some(lpf) = &emitter.control.low_pass {
            let current_cutoff = lpf.cutoff_hz.get();
            let target_cutoff = if occluded { 150.0 } else { 20000.0 };

            let new_cutoff =
                current_cutoff + (target_cutoff - current_cutoff) * 8.0 * time.delta_secs();
            lpf.cutoff_hz.set(new_cutoff.clamp(20.0, 20000.0));

            println!(
                "[Occlusion Debug] Occluded: {}, Cutoff: {:.1}",
                occluded, new_cutoff
            );

            if let Some(mut mat) = materials.get_mut(&material_handle.0) {
                if occluded {
                    mat.base_color = Color::srgb(1.0, 1.0, 0.0); // Желтый — звук перекрыт
                } else {
                    mat.base_color = Color::srgb(1.0, 1.0, 1.0); // Белый — прямая видимость
                }
            }
        }
    }
}
