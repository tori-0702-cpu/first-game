use bevy::prelude::*;
use crate::camera::CameraSettings;
use crate::wire::WireState; // main.rs 側にある WireState を参照

// 定数定義
pub const PLAYER_SIZE: Vec3 = Vec3::new(1.0, 2.0, 1.0);
pub const PLAYER_SPEED: f32 = 12.0;
pub const AIR_CONTROL_SPEED: f32 = 22.0;
pub const NORMAL_JUMP_FORCE: f32 = 10.0;
pub const WIRE_JUMP_FORCE: f32 = 18.0;

// コンポーネント定義
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerPhysics {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub is_float_mode: bool,
    pub float_timer: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement_and_jump);
    }
}

pub fn player_movement_and_jump(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    mut player_query: Single<(&mut Transform, &mut PlayerPhysics, &mut WireState), With<Player>>,
) {
    let (ref mut transform, ref mut physics, ref mut wire_state) = *player_query;

    let mut input_dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { input_dir.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { input_dir.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { input_dir.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { input_dir.x += 1.0; }

    let camera_yaw_rotation = Quat::from_rotation_y(settings.yaw);
    let forward = camera_yaw_rotation * Vec3::NEG_Z;
    let right = camera_yaw_rotation * Vec3::X;

    if input_dir != Vec3::ZERO {
        let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();

        let target_rotation = Transform::default()
            .looking_to(move_dir, Vec3::Y)
            .rotation;

        let rotation_speed = 15.0;
        transform.rotation = transform
            .rotation
            .slerp(target_rotation, rotation_speed * time.delta_secs());
    }

    if physics.is_float_mode {
        if input_dir != Vec3::ZERO {
            let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();
            physics.velocity.x = move_dir.x * AIR_CONTROL_SPEED;
            physics.velocity.z = move_dir.z * AIR_CONTROL_SPEED;
        } else {
            physics.velocity.x = 0.0;
            physics.velocity.z = 0.0;
        }
    } else if input_dir != Vec3::ZERO {
        let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();

        if physics.is_grounded {
            transform.translation += move_dir * PLAYER_SPEED * time.delta_secs();
        } else {
            physics.velocity.x += move_dir.x * PLAYER_SPEED * 3.0 * time.delta_secs();
            physics.velocity.z += move_dir.z * PLAYER_SPEED * 3.0 * time.delta_secs();
        }
    }

    if keyboard.just_pressed(KeyCode::Space) {
        if wire_state.target_point.is_some() {
            wire_state.target_point = None;
            physics.velocity.y = WIRE_JUMP_FORCE;
            physics.is_float_mode = true;
            physics.float_timer = 1.5;
        } else if physics.is_grounded {
            physics.velocity.y = NORMAL_JUMP_FORCE;
            physics.is_grounded = false;
        }
    }
}
