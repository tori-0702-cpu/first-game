use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};

use crate::player::Player;

const MOUSE_SENSITIVITY: f32 = 0.003;

#[derive(Component)]
pub struct PrimaryCamera;

#[derive(Resource)]
pub struct CameraSettings {
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub zoom_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            sensitivity: MOUSE_SENSITIVITY,
            pitch: 0.0,
            yaw: 0.0,
            distance: 10.0,
            min_distance: 3.0,
            max_distance: 30.0,
            zoom_speed: 1.5,
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_systems(
                Update,
                (rotate_camera, zoom_camera).chain(),
            )
                .add_systems(
                    PostUpdate,
                    update_camera,
                );
    }
}

fn rotate_camera(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut settings: ResMut<CameraSettings>,
    cursor_options_query: Query<&CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(cursor_options) = cursor_options_query.single() {
        if cursor_options.visible { return; }
    }

    if mouse_motion.delta != Vec2::ZERO {
        settings.yaw -= mouse_motion.delta.x * settings.sensitivity;
        settings.pitch -= mouse_motion.delta.y * settings.sensitivity;
        settings.pitch = settings.pitch.clamp(-85.0f32.to_radians(), 85.0f32.to_radians());
    }
}

fn zoom_camera(
    accumulated_scroll: Res<AccumulatedMouseScroll>,
    mut settings: ResMut<CameraSettings>,
    cursor_options_query: Query<&CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(cursor_options) = cursor_options_query.single() {
        if cursor_options.visible {return; }
    }

    let scroll_y = accumulated_scroll.delta.y;
    if scroll_y != 0.0 {
        settings.distance -= scroll_y * settings.zoom_speed;
        settings.distance = settings.distance.clamp(settings.min_distance, settings.max_distance);
    }
}

fn update_camera(
    settings: Res<CameraSettings>,
    player_query: Single<&Transform, (With<Player>, Without<PrimaryCamera>)>,
    mut camera_transform: Single<&mut Transform, (With<PrimaryCamera>, Without<Player>)>,
) {
    let player_transform = *player_query;

    let camera_rotation = Quat::from_euler(EulerRot::YXZ, settings.yaw, settings.pitch, 0.0);
    camera_transform.rotation = camera_rotation;

    let camera_offset = Vec3::new(0.0, 1.5, settings.distance);
    let mut desired_camera_pos = player_transform.translation + camera_rotation * camera_offset;

    if desired_camera_pos.y < 0.5 {
        desired_camera_pos.y = 0.5;
    }
    camera_transform.translation = desired_camera_pos;
}

