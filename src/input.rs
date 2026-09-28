use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

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
