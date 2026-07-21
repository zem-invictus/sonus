use crate::spatial_audio::emitter::SonusEmitter;
use bevy::prelude::*;

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
        
        // 1. По умолчанию фильтры полностью открыты (нейтральное состояние)
        let mut target_lpf = 20000.0f32;
        let mut target_hpf = 20.0f32;

        // 2. Если для эмиттера настроена окклюзия, рассчитываем ее
        if let Some(occlusion_control) = &emitter.control.occlusion_control {
            // Перебираем все стены и ищем пересечения со звуковым лучом
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
                    // Выбираем самую жесткую частоту среза среди всех пересеченных препятствий
                    target_lpf = target_lpf.min(material.lowpass_cutoff_hz);
                    target_hpf = target_hpf.max(material.highpass_cutoff_hz);
                }
            }

            // Плавно сглаживаем текущие значения фильтров к целевым с помощью LER-фильтрации
            let dt = time.delta_secs();
            let lerp_factor = (8.0 * dt).min(1.0); // Ограничиваем сверху единицей для стабильности при низком FPS

            let current_lpf = occlusion_control.lowpass_hz.get();
            let new_lpf = current_lpf + (target_lpf - current_lpf) * lerp_factor;
            occlusion_control.lowpass_hz.set(new_lpf.clamp(20.0, 20000.0));

            let current_hpf = occlusion_control.highpass_hz.get();
            let new_hpf = current_hpf + (target_hpf - current_hpf) * lerp_factor;
            occlusion_control.highpass_hz.set(new_hpf.clamp(20.0, 20000.0));
        }
    }
}
