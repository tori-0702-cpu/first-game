use std::time::Duration;
use bevy::prelude::*;
use avian3d::prelude::*; // Avian3D を読み込みます
use crate::camera::CameraSettings;
use crate::wire::WireState;

// 新しいアニメーションファイルを読み込む
pub mod animation;

// 定数定義
pub const PLAYER_SPEED: f32 = 12.0;
pub const PLAYER_MAX_SPEED: f32 = 20.0;
pub const AIR_CONTROL_SPEED: f32 = 22.0;
pub const NORMAL_JUMP_FORCE: f32 = 5.0; // Avian3Dの重力に合わせて適切な数値（12.0）に調整
pub const WIRE_JUMP_FORCE: f32 = 8.0;
pub const MODEL_PATH: &str = "models/test/test.glb";

// コンポーネント定義
#[derive(Resource)]
pub struct Model(pub Handle<Gltf>);

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerPhysics {
    pub is_grounded: bool,
    pub is_float_mode: bool,
    pub float_timer: f32,
}

#[derive(Resource)]
pub struct Animations {
    pub animations: Vec<AnimationNodeIndex>,
    pub graph_handle: Handle<AnimationGraph>,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(animation::PlayerAnimationPlugin)
            .add_systems(
                Update,
                spawn_player_asset_when_ready.run_if(not(resource_exists::<Animations>)),
            )
            .add_systems(
                Update,
                (
                    update_grounded_status, // 地面センサーの更新
                    player_movement_and_jump,
                ).chain(), // 必ず着地判定を計算してから移動処理を走らせます
            );
    }
}

/// 自分自身を完全に除外して、本物の床やビルに触れているか判定するシステム
fn update_grounded_status(
    mut query: Query<(Entity, &mut PlayerPhysics, &RayHits)>,
) {
    for (player_entity, mut physics, hits) in &mut query {
        // レーザーの当たった相手が「自分自身」ではない、かつ距離が0.15m以内の時だけ着地とみなす
        let has_ground_hit = hits.iter().any(|hit| {
            hit.entity != player_entity && hit.distance <= 1.7
        });

        physics.is_grounded = has_ground_hit;
    }
}

pub fn player_movement_and_jump(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    // 【修正】ref mut 展開時のバグを防ぐため、安全なクエリ指定を行います
    mut player_query: Single<(&mut Transform, &mut LinearVelocity, &mut PlayerPhysics, &mut WireState), With<Player>>,
) {
    let (ref mut transform, ref mut linear_velocity, ref mut physics, ref mut wire_state) = *player_query;

    let input = crate::input::get_player_input(&keyboard);
    let input_dir = input.move_dir;

    let camera_yaw_rotation = Quat::from_rotation_y(settings.yaw);
    let forward = camera_yaw_rotation * Vec3::NEG_Z;
    let right = camera_yaw_rotation * Vec3::X;

    // 振り向き処理
    if input_dir != Vec3::ZERO {
        let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();

        let target_rotation = Transform::default()
            .looking_to(move_dir, Vec3::Y)
            .rotation
            * Quat::from_rotation_y(std::f32::consts::PI);

        let rotation_speed = 15.0;
        transform.rotation = transform
            .rotation
            .slerp(target_rotation, rotation_speed * time.delta_secs());
    }

    // 移動速度の物理演算処理（Avian3D の .0 ベクトルを操作）
    if physics.is_float_mode {
        if input_dir != Vec3::ZERO {
            let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();
            linear_velocity.0.x = move_dir.x * AIR_CONTROL_SPEED;
            linear_velocity.0.z = move_dir.z * AIR_CONTROL_SPEED;
        } else {
            linear_velocity.0.x = 0.0;
            linear_velocity.0.z = 0.0;
        }
    } else if input_dir != Vec3::ZERO {
        let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();

        if physics.is_grounded {
            // 地上移動
            linear_velocity.0.x = move_dir.x * PLAYER_SPEED;
            linear_velocity.0.z = move_dir.z * PLAYER_SPEED;
        } else {
            // 【超加速防止】空中制御の限界速度を PLAYER_MAX_SPEED にクランプします
            let added_x = linear_velocity.0.x + move_dir.x * PLAYER_SPEED * 3.0 * time.delta_secs();
            let added_z = linear_velocity.0.z + move_dir.z * PLAYER_SPEED * 3.0 * time.delta_secs();

            linear_velocity.0.x = added_x.clamp(-PLAYER_MAX_SPEED, PLAYER_MAX_SPEED);
            linear_velocity.0.z = added_z.clamp(-PLAYER_MAX_SPEED, PLAYER_MAX_SPEED);
        }
    } else if physics.is_grounded {
        // 接地状態で入力がないときはピタッと止める
        linear_velocity.0.x = 0.0;
        linear_velocity.0.z = 0.0;
    }

    // ジャンプの処理
    if input.jump_just_pressed {
        if wire_state.target_point.is_some() {
            wire_state.target_point = None;
            linear_velocity.0.y = WIRE_JUMP_FORCE; // ワイヤージャンプ
            physics.is_float_mode = true;
            physics.float_timer = 1.5;
        } else if physics.is_grounded {
            linear_velocity.0.y = NORMAL_JUMP_FORCE; // 地上ジャンプ
            physics.is_grounded = false;
        }
    }
}

fn spawn_player_asset_when_ready(
    mut commands: Commands,
    model_handle: Res<Model>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if !asset_server.is_loaded_with_dependencies(&model_handle.0) {
        return;
    }

    let model = gltfs
        .get(&model_handle.0)
        .expect("a loaded asset should exist in the glTF assets collection");

    let (graph, node_indices) = AnimationGraph::from_clips([
        model.named_animations["Idle"].clone(),
        model.named_animations["Run"].clone(),
        model.named_animations["Jump"].clone(),
        model.named_animations["Hang"].clone(),
    ]);

    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        animations: node_indices,
        graph_handle,
    });

    commands
        .spawn((
            Player,
            RigidBody::Dynamic,            // 物理演算の本体
            Collider::capsule(0.4, 1.6),   // 中心(0,0)から上0.8m、下0.8mに広がる判定
            LockedAxes::ROTATION_LOCKED,   // 倒れるのを防止
            
            // 【センサー修正】判定が直るため、腰の位置（ZERO）から下へ1.0m探る設定に戻せます
            RayCaster::new(Vec3::ZERO, Dir3::NEG_Y)
            .with_max_distance(1.65), 

            PlayerPhysics {
                is_grounded: true,
                is_float_mode: false,
                float_timer: 0.0,
            },
            WireState::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            WorldAssetRoot(
                model.default_scene
                    .clone()
                    .expect("a default scene exist in this file"),
            ),
        ))
        .observe(animation::setup_player_scene);
}

