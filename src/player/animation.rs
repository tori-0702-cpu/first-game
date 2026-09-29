use std::time::Duration;
use bevy::prelude::*;
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

/// 物理状態（PlayerPhysics）を「覗き見」して、アニメーションの見た目を合わせるシステム
fn animate_player_by_physics(
    mut physics_query: Query<(&PlayerPhysics, &mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<Option<usize>>,
) {
    // 物理コンポーネント（physics）は読み取り専用（&PlayerPhysics）なので、物理の計算を汚しません！
    for (physics, mut player, mut transitions) in &mut physics_query {
        let target_animation = if physics.is_float_mode {
            3 // Hang（浮遊）
        } else if !physics.is_grounded {
            2 // Jump
        } else if physics.velocity.length_squared() > 0.01 {
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

