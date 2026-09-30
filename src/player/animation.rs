use std::time::Duration;
use bevy::prelude::*;
use avian3d::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::player::{Animations, Player, PlayerPhysics};
use crate::wire::WireState;

/// アニメーション関連のシステムを登録するプラグイン
pub struct PlayerAnimationPlugin;

impl Plugin for PlayerAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            animate_player_by_physics.run_if(resource_exists::<Animations>),
        );
    }
}

/// 3Dモデル生成時にアニメーションの初期設定を行う（Observe用関数）
pub fn setup_player_scene(
    _ready: On<WorldInstanceReady>,
    mut commands: Commands,
    animations: Res<Animations>,
    model: Single<(Entity, &mut AnimationPlayer)>,
) {
    let (entity, mut model) = model.into_inner();
    let mut transitions = AnimationTransitions::new();

    // 最初の待機モーション（Idle）を再生
    transitions
        .play(&mut model, animations.animations[0], Duration::ZERO)
        .repeat();

    commands
        .entity(entity)
        .insert(AnimationGraphHandle(animations.graph_handle.clone()))
        .insert(transitions);
}

fn animate_player_by_physics(
    player_query: Query<(&PlayerPhysics, &LinearVelocity, Option<&WireState>), With<Player>>,
    mut anim_query: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<Option<usize>>,
) {
    let Ok((physics, linear_velocity, wire_state)) = player_query.single() else {
        return;
    };

    // 水平方向（XZ平面）の移動速度
    let horizontal_velocity = Vec2::new(linear_velocity.0.x, linear_velocity.0.z);
    let is_moving = horizontal_velocity.length_squared() > 0.1;

    // 地上ジャンプ判定：着地状態かつ上昇中のときのみ Jump とする
    let is_ground_jumping_up = physics.is_grounded && linear_velocity.0.y > 0.1;

    // ワイヤー使用中・浮遊中か確認
    let is_using_wire = wire_state.map_or(false, |w| w.target_point.is_some()) || physics.is_float_mode;

    // モーション判定
    let target_animation = if is_using_wire {
        3 // Hang (ワイヤーアクション中・浮遊中)
    } else if is_ground_jumping_up {
        2 // Jump (地上から上昇中のジャンプ開始時)
    } else if !physics.is_grounded {
        3 // Hang (落下中、ワイヤージャンプ中、段差から落ちたとき)
    } else if is_moving {
        1 // Run (地上移動)
    } else {
        0 // Idle (地上待機)
    };

    // アニメーションの切り替え処理
    if *current_animation != Some(target_animation) {
        *current_animation = Some(target_animation);

        for (mut player, mut transitions) in &mut anim_query {
            let transition_duration = match target_animation {
                3 => Duration::from_millis(150), // Hang への補間
                2 => Duration::from_millis(60),  // ジャンプの素早い切り替え
                _ => Duration::from_millis(200), // Run / Idle 間の補間
            };

            if target_animation == 2 {
                // Jumpモーションは1回のみ再生
                transitions.play(
                    &mut player,
                    animations.animations[target_animation],
                    transition_duration,
                );
            } else {
                // その他のモーションはループ再生
                transitions
                    .play(
                        &mut player,
                        animations.animations[target_animation],
                        transition_duration,
                    )
                    .repeat();
            }
        }
    }
}
