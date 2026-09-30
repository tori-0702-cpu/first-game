use std::time::Duration;
use bevy::prelude::*;
use avian3d::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use crate::player::{Animations, PlayerPhysics}; // mod.rs から必要なデータをインポート

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

/// 3Dモデル生成時にアニメーションの初期設定を行う（observe用関数）
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
    // クエリに LinearVelocity を追加します
    mut physics_query: Query<(&PlayerPhysics, &LinearVelocity, &mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<Option<usize>>,
) {
    for (physics, linear_velocity, mut player, mut transitions) in &mut physics_query {
        let target_animation = if physics.is_float_mode {
            3 // Hang
        } else if !physics.is_grounded {
            2 // Jump
        } else if linear_velocity.0.length_squared() > 0.01 { // linear_velocity を参照
            1 // Run
        } else {
            0 // Idle
        };

        if *current_animation != Some(target_animation) {
            *current_animation = Some(target_animation);

            if target_animation == 2 {
                transitions.play(
                    &mut player,
                    animations.animations[target_animation],
                    Duration::from_millis(150),
                );
            } else {
                transitions
                    .play(
                        &mut player,
                        animations.animations[target_animation],
                        Duration::from_millis(250),
                    )
                    .repeat();
             }
         }
    }
}

