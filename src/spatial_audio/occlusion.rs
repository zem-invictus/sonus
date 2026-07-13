use std::mem::swap;
use bevy::prelude::Vec2;

/// Проверяет, пересекает ли 2D-отрезок от `p1` до `p2` прямоугольник (AABB),
/// заданный минимальным (`min`) и максимальным (`max`) углами.
/// 
/// Метод плит (Slab Method) в параметрическом пространстве t ∈ [0, 1].
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