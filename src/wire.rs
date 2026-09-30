use bevy::prelude::*;
use avian3d::prelude::*; // 【追加】Avian3Dの物理コンポーネントをインポート
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
    // 【修正】手動計算を排し、Avian3DのLinearVelocityを引数に追加
    mut player_query: Single<(&Transform, &mut LinearVelocity, &mut PlayerPhysics, &mut WireState), With<Player>>,
    obstacle_query: Query<(&Transform, &TargetObstacle)>,
) {
    let (player_transform, ref mut linear_velocity, ref mut physics, ref mut wire_state) = *player_query;
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
            // 【修正】手動物理から Avian3D のリニアベロシティに初期ブーストを加算
            linear_velocity.0 += pull_dir * WIRE_INITIAL_BOOST;
        }
    }
}

pub fn apply_wire_physics(
    time: Res<Time>,
    settings: Res<CameraSettings>,
    // 【修正】手動座標移動を撤廃し、Avian3D の LinearVelocity に一本化
    mut player_query: Single<(&Transform, &mut LinearVelocity, &mut PlayerPhysics, &mut WireState), With<Player>>,
) {
    let (transform, ref mut linear_velocity, ref mut physics, ref mut wire_state) = *player_query;
    let is_wiring = wire_state.target_point.is_some();

    if let Some(target) = wire_state.target_point {
        let current_pos = transform.translation;
        let to_target = target - current_pos;
        let current_dist = to_target.length();
        let pull_dir = to_target.normalize_or_zero();

        // 巻き取りによる最大射程を徐々に収縮
        wire_state.current_length = (wire_state.current_length - WIRE_REEL_SPEED * time.delta_secs()).max(2.0);

        // 【修正】ワイヤーを限界まで引っ張ったときの挙動
        if current_dist > wire_state.current_length {
            let vel_dot = linear_velocity.dot(pull_dir);
            if vel_dot < 0.0 {
                linear_velocity.0 -= pull_dir * vel_dot;
            }
        }

        // ワイヤーによる牽引力
        linear_velocity.0 += pull_dir * WIRE_PULL_FORCE * time.delta_secs();
        
        // カメラの向きへの前方ブースト
        let camera_forward = Quat::from_rotation_y(settings.yaw) * Vec3::NEG_Z;
        linear_velocity.0 += camera_forward * WIRE_FORWARD_BOOST * time.delta_secs();
        
        // 空気抵抗
        linear_velocity.0 *= 0.985;

        if current_dist < 2.5 {
            wire_state.target_point = None;
        }
    } else {
        // 浮遊状態のタイマー管理のみ行う
        if physics.is_float_mode {
            physics.float_timer -= time.delta_secs();
            if physics.float_timer <= 0.0 {
                physics.is_float_mode = false;
            }
        }
    }

    // =========================================================================
    // ❌ 【手動のコライダー・重力・接地判定計算を完全削除】
    // 
    // 地面(0.0)や障害物のループによる手動押し出し（インパルス）は全て削除しました。
    // 重力落下、床や障害物との衝突・侵入防止は Avian3D が自動計算します。
    // =========================================================================

    // 接地時の処理
    if physics.is_grounded {
        physics.is_float_mode = false;
        if !is_wiring {
            linear_velocity.x *= 0.1;
            linear_velocity.z *= 0.1;
        }
    } else if !is_wiring && !physics.is_float_mode {
        linear_velocity.x *= 0.985;
        linear_velocity.z *= 0.985;
    }
}

pub fn draw_wire_and_marker(
    player_query: Single<(&Transform, &WireState), With<Player>>,
    drone_query: Single<&Transform, With<Drone>>,
    mut gizmos: Gizmos,
) {
    let (player_transform, wire_state) = *player_query;
    let drone_transform = *drone_query;

    if let Some(target) = wire_state.target_point {
        gizmos.line(drone_transform.translation, target, Color::srgb(0.2, 0.9, 1.0));
        gizmos.sphere(Isometry3d::from_translation(target), 0.5, Color::srgb(1.0, 0.2, 0.2));
    } else if let Some(aim_point) = wire_state.aim_hit_point {
        gizmos.sphere(Isometry3d::from_translation(aim_point), 0.4, Color::srgba(0.2, 1.0, 0.2, 0.6));
    }
}

