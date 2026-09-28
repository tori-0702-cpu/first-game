use bevy::prelude::*;
use crate::camera::{CameraSettings, PrimaryCamera};
use crate::player::{Player, PlayerPhysics};
use crate::drone::Drone;

// 定数定義
pub const WIRE_REEL_SPEED: f32 = 20.0;
pub const WIRE_INITIAL_BOOST: f32 = 35.0;
pub const WIRE_PULL_FORCE: f32 = 18.0;
pub const WIRE_FORWARD_BOOST: f32 = 8.0;
pub const WIRE_MAX_DISTANCE: f32 = 180.0;
pub const WIRE_HIT_MARGIN: Vec3 = Vec3::new(1.0, 1.0, 1.0);
pub const GRAVITY: f32 = -25.0;
pub const PLAYER_SIZE: Vec3 = Vec3::new(1.0, 2.0, 1.0);

// コンポーネント定義
#[derive(Component)]
pub struct TargetObstacle {
    pub size: Vec3,
}

#[derive(Component, Default)]
pub struct WireState {
    pub target_point: Option<Vec3>,
    pub aim_hit_point: Option<Vec3>,
    pub current_length: f32,
}

pub struct WirePlugin;

impl Plugin for WirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_wire_target,
                apply_wire_physics,
                draw_wire_and_marker,
            ).chain(),
        );
    }
}

pub fn update_wire_target(
    mouse_button: Res<ButtonInput<MouseButton>>,
        camera_transform: Single<&Transform, With<PrimaryCamera>>,
    mut player_query: Single<(&Transform, &mut PlayerPhysics, &mut WireState), With<Player>>,
    obstacle_query: Query<(&Transform, &TargetObstacle)>,
) {
    let (player_transform, ref mut physics, ref mut wire_state) = *player_query;
    let ray_origin = camera_transform.translation;
    let ray_dir = camera_transform.forward();

    let mut closest_hit: Option<Vec3> = None;
    let mut min_dist = WIRE_MAX_DISTANCE;

    for (obs_transform, obs) in obstacle_query.iter() {
        let expanded_size = obs.size + WIRE_HIT_MARGIN;
        let min = obs_transform.translation - expanded_size * 0.5;
        let max = obs_transform.translation + expanded_size * 0.5;

        let mut tmin = (min.x - ray_origin.x) / ray_dir.x;
        let mut tmax = (max.x - ray_origin.x) / ray_dir.x;
        if tmin > tmax { std::mem::swap(&mut tmin, &mut tmax); }

        let mut tymin = (min.y - ray_origin.y) / ray_dir.y;
        let mut tymax = (max.y - ray_origin.y) / ray_dir.y;
        if tymin > tymax { std::mem::swap(&mut tymin, &mut tymax); }

        if (tmin > tymax) || (tymin > tmax) { continue; }
        if tymin > tmin { tmin = tymin; }
        if tymax < tmax { tmax = tymax; }

        let mut tzmin = (min.z - ray_origin.z) / ray_dir.z;
        let mut tzmax = (max.z - ray_origin.z) / ray_dir.z;
        if tzmin > tzmax { std::mem::swap(&mut tzmin, &mut tzmax); }

        if (tmin > tzmax) || (tzmin > tmax) { continue; }
        if tzmin > tmin { tmin = tzmin; }

        if tmin > 0.0 && tmin < min_dist {
            min_dist = tmin;
            closest_hit = Some(ray_origin + *ray_dir * tmin);
        }
    }

    wire_state.aim_hit_point = closest_hit;

    if mouse_button.just_pressed(MouseButton::Right) {
        if wire_state.target_point.is_some() {
            wire_state.target_point = None;
        } else if let Some(hit) = closest_hit {
            wire_state.target_point = Some(hit);
            wire_state.current_length = (hit - player_transform.translation).length();

            let pull_dir = (hit - player_transform.translation).normalize_or_zero();
            physics.velocity += pull_dir * WIRE_INITIAL_BOOST;
        }
    }
}

pub fn apply_wire_physics(
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut player_query: Single<(&mut Transform, &mut PlayerPhysics, &mut WireState), With<Player>>,
    obstacle_query: Query<(&Transform, &TargetObstacle), Without<Player>>,
) {
    let (ref mut transform, ref mut physics, ref mut wire_state) = *player_query;
    let is_wiring = wire_state.target_point.is_some();

    if let Some(target) = wire_state.target_point {
        let current_pos = transform.translation;
        let to_target = target - current_pos;
        let current_dist = to_target.length();
        let pull_dir = to_target.normalize_or_zero();

        wire_state.current_length = (wire_state.current_length - WIRE_REEL_SPEED * time.delta_secs()).max(2.0);
        physics.velocity.y += GRAVITY * 0.15 * time.delta_secs();

        if current_dist > wire_state.current_length {
            let vel_dot = physics.velocity.dot(pull_dir);
            if vel_dot < 0.0 {
                physics.velocity -= pull_dir * vel_dot;
            }
            transform.translation = target - pull_dir * wire_state.current_length;
        }

        physics.velocity += pull_dir * WIRE_PULL_FORCE * time.delta_secs();
        let camera_forward = Quat::from_rotation_y(settings.yaw) * Vec3::NEG_Z;
        physics.velocity += camera_forward * WIRE_FORWARD_BOOST * time.delta_secs();
        physics.velocity *= 0.985;
        transform.translation += physics.velocity * time.delta_secs();

        if current_dist < 2.5 {
            wire_state.target_point = None;
        }
    } else {
        let current_gravity = if physics.is_float_mode {
            physics.float_timer -= time.delta_secs();
            if physics.float_timer <= 0.0 {
                physics.is_float_mode = false;
            }
            GRAVITY * 0.25
        } else {
            GRAVITY
        };

        physics.velocity.y += current_gravity * time.delta_secs();
        transform.translation += physics.velocity * time.delta_secs();
    }

    let player_radius = PLAYER_SIZE.x * 0.5;
    let player_half_height = PLAYER_SIZE.y * 0.5;
    let player_bottom = transform.translation.y - player_half_height;

    let mut grounded = false;

    if player_bottom <= 0.0 {
        transform.translation.y = player_half_height;
        physics.velocity.y = 0.0;
        grounded = true;
    }

    for (obs_transform, obs) in obstacle_query.iter() {
        let obs_half = obs.size * 0.5;
        let obs_pos = obs_transform.translation;

        let in_xz = (transform.translation.x - obs_pos.x).abs() <= (obs_half.x + player_radius)
            && (transform.translation.z - obs_pos.z).abs() <= (obs_half.z + player_radius);

        if in_xz {
            let obs_top = obs_pos.y + obs_half.y;
            let prev_bottom = player_bottom - physics.velocity.y * time.delta_secs();

            if physics.velocity.y <= 0.0 && prev_bottom >= obs_top - 0.5 && player_bottom <= obs_top {
                transform.translation.y = obs_top + player_half_height;
                physics.velocity.y = 0.0;
                grounded = true;
                continue;
            }

            let y_overlap = (transform.translation.y - obs_pos.y).abs() < (player_half_height + obs_half.y - 0.1);
            if y_overlap {
                let closest_x = transform.translation.x.clamp(obs_pos.x - obs_half.x, obs_pos.x + obs_half.x);
                let closest_z = transform.translation.z.clamp(obs_pos.z - obs_half.z, obs_pos.z + obs_half.z);

                let diff = Vec2::new(transform.translation.x - closest_x, transform.translation.z - closest_z);
                let dist = diff.length();

                if dist < player_radius && dist > 0.0 {
                    let normal = diff / dist;
                    let push_out = normal * (player_radius - dist);

                    transform.translation.x += push_out.x;
                    transform.translation.z += push_out.y;

                    let vel_xz = Vec2::new(physics.velocity.x, physics.velocity.z);
                    let dot = vel_xz.dot(normal);
                    if dot < 0.0 {
                        let new_vel = vel_xz - normal * dot;
                        physics.velocity.x = new_vel.x;
                        physics.velocity.z = new_vel.y;
                    }
                }
            }
        }
    }

    physics.is_grounded = grounded;

    if grounded {
        physics.is_float_mode = false;
        if !is_wiring {
            physics.velocity.x *= 0.1;
            physics.velocity.z *= 0.1;
        }
    } else if !is_wiring && !physics.is_float_mode {
        physics.velocity.x *= 0.985;
        physics.velocity.z *= 0.985;
    }
}

pub fn draw_wire_and_marker(
    player_query: Single<(&Transform, &WireState), With<Player>>,
    drone_query: Single<&Transform, With<Drone>>,
    mut gizmos: Gizmos,
) {
    let (_, wire_state) = *player_query;
    let drone_transform = *drone_query;

    if let Some(target) = wire_state.target_point {
        gizmos.line(drone_transform.translation, target, Color::srgb(0.2, 0.9, 1.0));
        gizmos.sphere(Isometry3d::from_translation(target), 0.5, Color::srgb(1.0, 0.2, 0.2));
    } else if let Some(aim_point) = wire_state.aim_hit_point {
        gizmos.sphere(Isometry3d::from_translation(aim_point), 0.4, Color::srgba(0.2, 1.0, 0.2, 0.6));
    }
}
