use std::time::Duration;
use bevy::prelude::*;
use crate::camera::CameraSettings;
use crate::wire::WireState;

// 新しいアニメーションファイルを読み込む
pub mod animation;

// 定数定義
pub const PLAYER_SPEED: f32 = 12.0;
pub const AIR_CONTROL_SPEED: f32 = 22.0;
pub const NORMAL_JUMP_FORCE: f32 = 10.0;
pub const WIRE_JUMP_FORCE: f32 = 18.0;
pub const MODEL_PATH: &str = "model/test/test.glb";

// コンポーネント定義
#[derive(Resource)]
pub struct Model(pub Handle<Gltf>);

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerPhysics {
    pub velocity: Vec3,
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
        // アニメーション用のプラグインも一緒にここで登録します
        app.add_plugins(animation::PlayerAnimationPlugin)
            .add_systems(
                Update,
                spawn_player_asset_when_ready.run_if(not(resource_exists::<Animations>)),
            )
            .add_systems(Update, player_movement_and_jump);
    }
}

pub fn player_movement_and_jump(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    mut player_query: Single<(&mut Transform, &mut PlayerPhysics, &mut WireState), With<Player>>,
) {
    let (ref mut transform, ref mut physics, ref mut wire_state) = *player_query;

    let input = crate::input::get_player_input(&keyboard);
    let input_dir = input.move_dir;

    let camera_yaw_rotation = Quat::from_rotation_y(settings.yaw);
    let forward = camera_yaw_rotation * Vec3::NEG_Z;
    let right = camera_yaw_rotation * Vec3::X;

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
            physics.velocity.x = move_dir.x * PLAYER_SPEED;
            physics.velocity.z = move_dir.z * PLAYER_SPEED;
            transform.translation += move_dir * PLAYER_SPEED * time.delta_secs();
        } else {
            physics.velocity.x += move_dir.x * PLAYER_SPEED * 3.0 * time.delta_secs();
            physics.velocity.z += move_dir.z * PLAYER_SPEED * 3.0 * time.delta_secs();
        }
    } else if physics.is_grounded {
        physics.velocity.x = 0.0;
        physics.velocity.z = 0.0;
    }

    if input.jump_just_pressed {
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
            PlayerPhysics {
                velocity: Vec3::ZERO,
                is_grounded: true,
                is_float_mode: false,
                float_timer: 0.0,
            },
            WireState::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            WorldAssetRoot(
                model.default_scene
                    .clone()
                    .expect("a default scene exists in this file"),
            ),
        ))
        // 修正点：animation.rs に引っ越した初期化関数を呼び出します
        .observe(animation::setup_player_scene);
}

