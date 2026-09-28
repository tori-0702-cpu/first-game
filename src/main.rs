use bevy::prelude::*;

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
use wire::WirePlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraPlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(WirePlugin)
        .add_plugins(DronePlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(InputPlugin)
        .run();
}
