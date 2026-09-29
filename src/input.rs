use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy::app::AppExit;
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_cursor_lock, 
                exit_game,          
            ),
        );
    }
}

/// プレイヤーの入力をひとまとめにする構造体
pub struct PlayerInputState {
    pub move_dir: Vec3,
    pub jump_just_pressed: bool,
}

/// キーボードの入力状態をすべて集約して返します
pub fn get_player_input(keyboard: &Res<ButtonInput<KeyCode>>) -> PlayerInputState {
    let mut input_dir = Vec3::ZERO;
    
    // 移動入力
    if keyboard.pressed(KeyCode::KeyW) { input_dir.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { input_dir.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { input_dir.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { input_dir.x += 1.0; }

    PlayerInputState {
        move_dir: input_dir,
        jump_just_pressed: keyboard.just_pressed(KeyCode::Space),
    }
}

pub fn toggle_cursor_lock(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cursor_options_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
            if cursor_options.grab_mode == CursorGrabMode::Locked {
                cursor_options.grab_mode = CursorGrabMode::None;
                cursor_options.visible = true;
            } else {
                cursor_options.grab_mode = CursorGrabMode::Locked;
                cursor_options.visible = false;
            }
        }
    }
}

pub fn exit_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::KeyQ) {
        exit.write(AppExit::Success);
    }
}
