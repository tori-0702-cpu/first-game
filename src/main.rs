use std::time::Duration;
use bevy::prelude::*;
use avian3d::prelude::*;

mod camera;
mod drone;
mod input;
mod player;
mod wire;
mod world;

use camera::CameraPlugin;
use drone::DronePlugin;
use input::InputPlugin;
use player::PlayerPlugin;
use player::Player;
use wire::WirePlugin;
use world::WorldPlugin;
use crate::wire::WireState;
use crate::player::PlayerPhysics;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(CameraPlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(WirePlugin)
        .add_plugins(DronePlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(InputPlugin)
        .add_systems(Update, log_player_status)
        .run();
}

/// プレイヤーの各種物理ステータスを0.5秒に1回ターミナルに表示するデバッグシステム
fn log_player_status(
    time: Res<Time>,
    // 0.5秒タイマーをローカル変数として保持
    mut timer: Local<Option<Timer>>, 
    player_query: Query<(&Transform, &LinearVelocity, &PlayerPhysics, &WireState), With<Player>>,
) {
    // タイマーの初期化（最初の1回だけ実行）
    let timer = timer.get_or_insert_with(|| Timer::new(Duration::from_millis(5000), TimerMode::Repeating));
    
    // タイマーを進める
    timer.tick(time.delta());

    // 0.5秒経った瞬間だけログを出力
    if timer.just_finished() {
        if let Ok((transform, linear_velocity, physics, wire_state)) = player_query.single() {
            let pos = transform.translation;
            let vel = linear_velocity.0;
            
            // ワイヤーが刺さっているかどうかの文字列
            let wire_status = if wire_state.target_point.is_some() { "接続中" } else { "未接続" };

            // ターミナルに見やすく色付きのような区切りで出力
            info!(
                "\n=== PLAYER DEBUG LOG ===\n\
                  位置: X:{:5.1}, Y:{:5.1}, Z:{:5.1}\n\
                  速度: X:{:5.1}, Y:{:5.1}, Z:{:5.1} (時速: {:5.1})\n\
                  地面: {:5} |  浮遊モード: {:5} (残り: {:.1}秒)\n\
                  ﾜｲｱｰ: {}\n\
                 ========================",
                pos.x, pos.y, pos.z,
                vel.x, vel.y, vel.z, vel.length(),
                physics.is_grounded, physics.is_float_mode, physics.float_timer,
                wire_status
            );
        }
    }
}

