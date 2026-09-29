use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::world_serialization::WorldInstanceReady;
use crate::camera::CameraSettings;
use crate::wire::WireState; // main.rs 側にある WireState を参照

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
struct Animations {
    animations: Vec<AnimationNodeIndex>,
    graph_handle: Handle<AnimationGraph>,
}


pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_player_asset_when_ready.run_if(not(resource_exists::<Animations>)),
            )
        .add_systems(Update, player_movement_and_jump)
        .add_systems(Update, animate_player_by_physics.run_if(resource_exists::<Animations>));
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
    .expect("a loaded asset should exist in the glTF assets cokkection");

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
                    is_float_mode:false,
                    float_timer: 0.0,
                },
                WireState::default(),
                Transform::from_xyz(0.0, 0.0, 0.0),
                WorldAssetRoot(
                    model.default_scene
                        .clone()
                        .expect("a default scene exitsts in thisfile"),
                ),
        ))
        .observe(setup_scene);
}

fn setup_scene(
    _ready: On<WorldInstanceReady>,
    mut commands: Commands,
    animations: Res<Animations>,
    model: Single<(Entity, &mut AnimationPlayer)>,
) {
    let (entity, mut model) = model.into_inner();
    let mut transitions = AnimationTransitions::new();

    transitions
        .play(&mut model, animations.animations[0], Duration::ZERO)
        .repeat();

    commands
        .entity(entity)
        .insert(AnimationGraphHandle(animations.graph_handle.clone()))
        .insert(transitions);
}

fn animate_player_by_physics(
    mut physics_query: Query<(&PlayerPhysics, &mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<Option<usize>>, // 現在再生中のアニメーションを記憶
) {
    for (physics, mut player, mut transitions) in &mut physics_query {
        // 1. どの状況でどのアニメーションを流すかの条件分岐（優先度順）
        let target_animation = if physics.is_float_mode {
            // 浮遊モード中のアニメーション（なければ一旦Jumpなどで代用）
            3 
        } else if !physics.is_grounded {
            // 空中にいる（ジャンプ・落下中）
            2
        } else if physics.velocity.length_squared() > 0.01 {
            // 地地にいて、動いている（速度がある）
            1
        } else {
            // 地地にいて、止まっている
            0
        };

        // 2. 現在再生したいアニメーションが、すでに再生中のものと違う場合だけ切り替える
        if *current_animation != Some(target_animation) {
            *current_animation = Some(target_animation);

            // 0.25秒（250ミリ秒）かけて滑らかに次のアニメーションへ繋ぐ
            transitions
                .play(
                    &mut player,
                    animations.animations[target_animation],
                    Duration::from_millis(250),
                )
                .repeat(); // ループ再生
        }
    }
}

