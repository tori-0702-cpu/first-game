use bevy::app::AppExit;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

const PLAYER_SIZE: Vec3 = Vec3::new(1.0, 2.0, 1.0);
const PLAYER_SPEED: f32 = 12.0;
const AIR_CONTROL_SPEED: f32 = 22.0;
const GRAVITY: f32 = -25.0;

const NORMAL_JUMP_FORCE: f32 = 10.0;
const WIRE_JUMP_FORCE: f32 = 18.0;

const MOUSE_SENSITIVITY: f32 = 0.003;
const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 3.0, 10.0);

const WIRE_REEL_SPEED: f32 = 20.0;       // 1秒間に短くなるワイヤーの長さ(m)
const WIRE_INITIAL_BOOST: f32 = 35.0;   // ワイヤー射出時の初速（一気に加速）
const WIRE_PULL_FORCE: f32 = 18.0;      // 継続的な引き寄せ力
const WIRE_FORWARD_BOOST: f32 = 8.0;    // 前方推進アシスト
const WIRE_MAX_DISTANCE: f32 = 180.0;
const WIRE_HIT_MARGIN: Vec3 = Vec3::new(1.0, 1.0, 1.0);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Drone;

#[derive(Component)]
struct PrimaryCamera;

#[derive(Component)]
struct PlayerPhysics {
    velocity: Vec3,
    is_grounded: bool,
    is_float_mode: bool,
    float_timer: f32,
}

#[derive(Component)]
struct TargetObstacle {
    size: Vec3,
}

#[derive(Component, Default)]
struct WireState {
    target_point: Option<Vec3>,
    aim_hit_point: Option<Vec3>,
    current_length: f32,
}

#[derive(Resource)]
struct CameraSettings {
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            sensitivity: MOUSE_SENSITIVITY,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CameraSettings>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_cursor_lock,
                rotate_camera,
                update_wire_target,
                player_movement_and_jump,
                apply_wire_physics,
                update_drone_position,
                draw_wire_and_marker,
                update_camera,
                exit_game,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_options_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(50.0, 120.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 地面 (1000 x 1000)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1000.0, 1.0, 1000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.4, 0.2))),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    let table_mat = materials.add(Color::srgb(0.3, 0.6, 0.8));
    let block_mat = materials.add(Color::srgb(0.8, 0.5, 0.2));

    let mut spawn_table = |cmd: &mut Commands,
                           meshes: &mut ResMut<Assets<Mesh>>,
                           center: Vec3,
                           table_size: Vec2,
                           table_height: f32,
                           top_thickness: f32,
                           leg_thickness: f32| {
        let top_size = Vec3::new(table_size.x, top_thickness, table_size.y);
        let top_pos = center + Vec3::new(0.0, table_height - top_thickness * 0.5, 0.0);
        cmd.spawn((
            TargetObstacle { size: top_size },
            Mesh3d(meshes.add(Cuboid::from_size(top_size))),
            MeshMaterial3d(table_mat.clone()),
            Transform::from_translation(top_pos),
        ));

        let leg_h = table_height - top_thickness;
        let leg_size = Vec3::new(leg_thickness, leg_h, leg_thickness);
        let leg_mesh = meshes.add(Cuboid::from_size(leg_size));

        let offset_x = (table_size.x - leg_thickness) * 0.5 - 0.5;
        let offset_z = (table_size.y - leg_thickness) * 0.5 - 0.5;
        let leg_y = center.y + leg_h * 0.5;

        let leg_offsets = [
            Vec3::new(offset_x, 0.0, offset_z),
            Vec3::new(-offset_x, 0.0, offset_z),
            Vec3::new(offset_x, 0.0, -offset_z),
            Vec3::new(-offset_x, 0.0, -offset_z),
        ];

        for offset in leg_offsets {
            cmd.spawn((
                TargetObstacle { size: leg_size },
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(table_mat.clone()),
                Transform::from_translation(Vec3::new(center.x + offset.x, leg_y, center.z + offset.z)),
            ));
        }
    };

    let grid_size = 7;
    let spacing = 60.0;
    let offset = (grid_size as f32 - 1.0) * spacing * 0.5;

    for i in 0..grid_size {
        for j in 0..grid_size {
            if i == 3 && j == 3 { continue; }

            let base_x = (i as f32) * spacing - offset;
            let base_z = (j as f32) * spacing - offset;
            let shift_x = ((i * 17 + j * 31) % 20) as f32 - 10.0;
            let shift_z = ((i * 23 + j * 13) % 20) as f32 - 10.0;
            let center = Vec3::new(base_x + shift_x, 0.0, base_z + shift_z);

            if (i + j) % 2 == 0 {
                let height = 18.0 + ((i + j * 3) % 4) as f32 * 4.0;
                spawn_table(&mut commands, &mut meshes, center, Vec2::new(22.0, 16.0), height, 1.5, 1.5);
                if (i * j) % 3 == 0 {
                    let tier2_center = Vec3::new(center.x, height, center.z);
                    spawn_table(&mut commands, &mut meshes, tier2_center, Vec2::new(14.0, 10.0), 12.0, 1.2, 1.2);
                }
            } else {
                let block_w = 12.0 + ((i * 7) % 8) as f32;
                let block_h = 15.0 + ((j * 11) % 25) as f32;
                let block_d = 12.0 + ((i + j) % 8) as f32;
                let block_size = Vec3::new(block_w, block_h, block_d);

                commands.spawn((
                    TargetObstacle { size: block_size },
                    Mesh3d(meshes.add(Cuboid::from_size(block_size))),
                    MeshMaterial3d(block_mat.clone()),
                    Transform::from_translation(center + Vec3::new(0.0, block_h * 0.5, 0.0)),
                ));
            }
        }
    }

    // 主人公
    commands.spawn((
        Player,
        PlayerPhysics {
            velocity: Vec3::ZERO,
            is_grounded: false,
            is_float_mode: false,
            float_timer: 0.0,
        },
        WireState::default(),
        Mesh3d(meshes.add(Cuboid::from_size(PLAYER_SIZE))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));

    // 多機能ドローン
    commands.spawn((
        Drone,
        Mesh3d(meshes.add(Sphere::new(0.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.8, 0.9))),
        Transform::from_xyz(0.0, 3.2, 0.0),
    ));

    // カメラ
    commands.spawn((
        PrimaryCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn toggle_cursor_lock(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cursor_options_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
            cursor_options.visible = !cursor_options.visible;
            cursor_options.grab_mode = if cursor_options.visible {
                CursorGrabMode::None
            } else {
                CursorGrabMode::Locked
            };
        }
    }
}

fn rotate_camera(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut settings: ResMut<CameraSettings>,
    cursor_options_query: Query<&CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(cursor_options) = cursor_options_query.single() {
        if cursor_options.visible { return; }
    }

    if mouse_motion.delta != Vec2::ZERO {
        settings.yaw -= mouse_motion.delta.x * settings.sensitivity;
        settings.pitch -= mouse_motion.delta.y * settings.sensitivity;
        settings.pitch = settings.pitch.clamp(-85.0f32.to_radians(), 85.0f32.to_radians());
    }
}

fn update_wire_target(
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

            // ワイヤー接続開始時に一気に初速を加算（スイングの初速を大きく）
            let pull_dir = (hit - player_transform.translation).normalize_or_zero();
            physics.velocity += pull_dir * WIRE_INITIAL_BOOST;
        }
    }
}

fn player_movement_and_jump(
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

fn apply_wire_physics(
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

        // 時間経過でワイヤーの目標長さを短くする
        wire_state.current_length = (wire_state.current_length - WIRE_REEL_SPEED * time.delta_secs()).max(2.0);

        // ワイヤー移動中は重力を大幅に軽減（通常の15%程度）
        physics.velocity.y += GRAVITY * 0.15 * time.delta_secs();

        // ワイヤーの長さを超えそうになった場合、外側へ離れる速度成分を除去する（拘束）
        if current_dist > wire_state.current_length {
            let vel_dot = physics.velocity.dot(pull_dir);
            if vel_dot < 0.0 {
                physics.velocity -= pull_dir * vel_dot;
            }
            transform.translation = target - pull_dir * wire_state.current_length;
        }

        // 継続的な磁力引き寄せ力
        physics.velocity += pull_dir * WIRE_PULL_FORCE * time.delta_secs();

        // 前方への進行アシスト
        let camera_forward = Quat::from_rotation_y(settings.yaw) * Vec3::NEG_Z;
        physics.velocity += camera_forward * WIRE_FORWARD_BOOST * time.delta_secs();

        // 空気抵抗によるゆるやかな減衰
        physics.velocity *= 0.985;

        transform.translation += physics.velocity * time.delta_secs();

        // 目標地点に十分接近したら自動解除
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

fn update_drone_position(
    time: Res<Time>,
    player_query: Single<(&Transform, &WireState), With<Player>>,
    mut drone_query: Single<&mut Transform, (With<Drone>, Without<Player>)>,
) {
    let (player_transform, wire_state) = *player_query;

    let target_drone_pos = if wire_state.target_point.is_some() {
        player_transform.translation + Vec3::new(0.0, 1.2, -0.3)
    } else {
        player_transform.translation + Vec3::new(0.8, 1.2, 0.8)
    };

    drone_query.translation = drone_query
        .translation
        .lerp(target_drone_pos, 15.0 * time.delta_secs());
}

fn draw_wire_and_marker(
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

fn update_camera(
    settings: Res<CameraSettings>,
    player_query: Single<&Transform, (With<Player>, Without<PrimaryCamera>)>,
    mut camera_transform: Single<&mut Transform, (With<PrimaryCamera>, Without<Player>)>,
) {
    let player_transform = *player_query;

    let camera_rotation = Quat::from_euler(EulerRot::YXZ, settings.yaw, settings.pitch, 0.0);
    camera_transform.rotation = camera_rotation;

    let mut desired_camera_pos = player_transform.translation + camera_rotation * CAMERA_OFFSET;

    if desired_camera_pos.y < 0.5 {
        desired_camera_pos.y = 0.5;
    }

    camera_transform.translation = desired_camera_pos;
}

fn exit_game(keyboard: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
