use bevy::app::AppExit;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

const PLAYER_SIZE: Vec3 = Vec3::new(1.0, 2.0, 1.0);
const PLAYER_SPEED: f32 = 5.0;
const GRAVITY: f32 = -20.0;

const POLE_SIZE: Vec3 = Vec3::new(1.0, 10.0, 1.0);
const MOUSE_SENSITIVITY: f32 = 0.003;
const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 3.0, 10.0);

const WIRE_PULL_FORCE: f32 = 40.0;
const WIRE_MAX_DISTANCE: f32 = 30.0;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PrimaryCamera;

#[derive(Component)]
struct PlayerPhysics {
    velocity: Vec3,
}

#[derive(Component)]
struct Platform {
    size: Vec3,
}

#[derive(Component)]
struct Pole {
    size: Vec3,
}

#[derive(Component, Default)]
struct WireState {
    target_point: Option<Vec3>,
    aim_hit_point: Option<Vec3>,
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
                apply_wire_physics,
                draw_wire_and_marker,
                update_camera_and_player,
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

    // ライト
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 地面
    commands.spawn((
        Platform {
            size: Vec3::new(50.0, 1.0, 50.0),
        },
        Mesh3d(meshes.add(Cuboid::new(50.0, 1.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    // 柱
    let pole_mesh = meshes.add(Cuboid::from_size(POLE_SIZE));
    let pole_material = materials.add(Color::srgb(0.2, 0.6, 0.8));

    let pole_positions = [
        Vec3::new(0.0, 5.0, -10.0),
        Vec3::new(10.0, 5.0, -15.0),
        Vec3::new(-10.0, 5.0, -15.0),
        Vec3::new(0.0, 5.0, -25.0),
    ];

    for pos in pole_positions {
        commands.spawn((
            Pole { size: POLE_SIZE },
            Platform { size: POLE_SIZE },
            Mesh3d(pole_mesh.clone()),
            MeshMaterial3d(pole_material.clone()),
            Transform::from_translation(pos),
        ));
    }

    // プレイヤー
    commands.spawn((
        Player,
        PlayerPhysics { velocity: Vec3::ZERO },
        WireState::default(),
        Mesh3d(meshes.add(Cuboid::from_size(PLAYER_SIZE))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));

    // カメラ
    commands.spawn((
        PrimaryCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        if cursor_options.visible {
            return;
        }
    }

    if mouse_motion.delta != Vec2::ZERO {
        settings.yaw -= mouse_motion.delta.x * settings.sensitivity;
        settings.pitch -= mouse_motion.delta.y * settings.sensitivity;
        settings.pitch = settings.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }
}

fn update_wire_target(
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_transform: Single<&Transform, With<PrimaryCamera>>,
    mut player_query: Single<&mut WireState, With<Player>>,
    pole_query: Query<(&Transform, &Pole)>,
) {
    let ref mut wire_state = *player_query;
    let ray_origin = camera_transform.translation;
    let ray_dir = camera_transform.forward();

    let mut closest_hit: Option<Vec3> = None;
    let mut min_dist = WIRE_MAX_DISTANCE;

    for (pole_transform, pole) in pole_query.iter() {
        let min = pole_transform.translation - pole.size * 0.5;
        let max = pole_transform.translation + pole.size * 0.5;

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

    // 右クリックでトグル処理（ONなら解除、OFFかつターゲットがあれば設置）
    if mouse_button.just_pressed(MouseButton::Right) {
        if wire_state.target_point.is_some() {
            wire_state.target_point = None;
        } else {
            wire_state.target_point = closest_hit;
        }
    }
}

fn apply_wire_physics(
    time: Res<Time>,
    mut player_query: Single<(&mut Transform, &mut PlayerPhysics, &WireState), With<Player>>,
    pole_query: Query<(&Transform, &Pole), Without<Player>>,
) {
    let (ref mut transform, ref mut physics, wire_state) = *player_query;

    if let Some(target) = wire_state.target_point {
        let pull_dir = (target - transform.translation).normalize_or_zero();
        physics.velocity += pull_dir * WIRE_PULL_FORCE * time.delta_secs();
    }

    physics.velocity.y += GRAVITY * time.delta_secs();
    transform.translation += physics.velocity * time.delta_secs();

    // 1. 地面との当たり判定
    if transform.translation.y < 1.0 {
        transform.translation.y = 1.0;
        physics.velocity.y = 0.0;
    }

    // 2. 柱との当たり判定（壁判定）
    let player_radius = PLAYER_SIZE.x * 0.5;
    let player_half_height = PLAYER_SIZE.y * 0.5;

    for (pole_transform, pole) in pole_query.iter() {
        let pole_half = pole.size * 0.5;
        let pole_pos = pole_transform.translation;

        // Y軸の重なり判定
        let y_overlap = (transform.translation.y - pole_pos.y).abs() < (player_half_height + pole_half.y);

        if y_overlap {
            // XZ平面での最近接点を計算
            let closest_x = transform.translation.x.clamp(pole_pos.x - pole_half.x, pole_pos.x + pole_half.x);
            let closest_z = transform.translation.z.clamp(pole_pos.z - pole_half.z, pole_pos.z + pole_half.z);

            let diff = Vec2::new(transform.translation.x - closest_x, transform.translation.z - closest_z);
            let dist = diff.length();

            // めり込んでいる場合、外側に押し出す
            if dist < player_radius && dist > 0.0 {
                let normal = diff / dist;
                let push_out = normal * (player_radius - dist);

                transform.translation.x += push_out.x;
                transform.translation.z += push_out.y;

                // 柱の壁方向への速度成分を減衰
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

    physics.velocity.x *= 0.98;
    physics.velocity.z *= 0.98;
}

fn draw_wire_and_marker(
    player_query: Single<(&Transform, &WireState), With<Player>>,
    mut gizmos: Gizmos,
) {
    let (player_transform, wire_state) = *player_query;

    if let Some(target) = wire_state.target_point {
        gizmos.line(player_transform.translation, target, Color::srgb(1.0, 0.9, 0.2));
        gizmos.sphere(Isometry3d::from_translation(target), 0.3, Color::srgb(1.0, 0.2, 0.2));
    } else if let Some(aim_point) = wire_state.aim_hit_point {
        gizmos.sphere(Isometry3d::from_translation(aim_point), 0.2, Color::srgba(1.0, 1.0, 1.0, 0.5));
    }
}

fn update_camera_and_player(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    mut player_query: Single<&mut Transform, (With<Player>, Without<PrimaryCamera>)>,
    mut camera_transform: Single<&mut Transform, (With<PrimaryCamera>, Without<Player>)>,
) {
    let ref mut player_transform = *player_query;

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
        player_transform.translation += move_dir * PLAYER_SPEED * time.delta_secs();
    }

    let camera_rotation = Quat::from_euler(EulerRot::YXZ, settings.yaw, settings.pitch, 0.0);
    camera_transform.rotation = camera_rotation;
    camera_transform.translation = player_transform.translation + camera_rotation * CAMERA_OFFSET;
}

fn exit_game(keyboard: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
