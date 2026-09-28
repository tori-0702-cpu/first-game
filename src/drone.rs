use bevy::prelude::*;
use crate::player::{Player, PlayerPhysics};
use crate::wire::WireState;

#[derive(Component)]
pub struct Drone;

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_drone_position);
    }
}

pub fn update_drone_position(
    time: Res<Time>,
    player_query: Single<(&Transform, &PlayerPhysics, &WireState), With<Player>>,
    mut drone_query: Single<&mut Transform, (With<Drone>, Without<Player>)>,
) {
    let (player_transform, physics, wire_state) = *player_query;
    let ref mut drone_transform = *drone_query;

    let target_pos = if let Some(target) = wire_state.target_point {
        let player_to_target = (target - player_transform.translation).normalize_or_zero();
        player_transform.translation + player_to_target * 1.5 + Vec3::Y * 0.5
    } else {
        let speed = physics.velocity.length();
        let hover_offset = Vec3::new(
            (time.elapsed_secs() * 2.0).sin() * 0.3,
            1.8 + (time.elapsed_secs() * 3.0).sin() * 0.1,
            (time.elapsed_secs() * 1.5).cos() * 0.3,
        );

        let follow_behind = if speed > 1.0 {
            -physics.velocity.normalize() * 1.2
        } else {
            Vec3::new(0.8, 0.0, -0.8)
        };

        player_transform.translation + follow_behind + hover_offset
    };

    drone_transform.translation = drone_transform
        .translation
        .lerp(target_pos, 10.0 * time.delta_secs());

    if let Some(target) = wire_state.target_point {
        drone_transform.look_at(target, Vec3::Y);
    } else if let Some(aim) = wire_state.aim_hit_point {
        drone_transform.look_at(aim, Vec3::Y);
    } else {
        let forward = drone_transform.translation + *player_transform.forward();
        drone_transform.look_at(forward, Vec3::Y);
    }
}
