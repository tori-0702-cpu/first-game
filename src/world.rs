use bevy::prelude::*;
use avian3d::prelude::*;
use crate::camera::PrimaryCamera;
use crate::drone::Drone;
use crate::player::{Player, PlayerPhysics, MODEL_PATH, Model};
use crate::wire::{WireState, TargetObstacle};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>
) {

    commands.insert_resource(Model(asset_server.load(MODEL_PATH)));
    // ライトの設定
    commands.spawn((
            DirectionalLight::default(),
            Transform::from_xyz(50.0, 120.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    // 床の生成
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));


    // カメラの生成
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        PrimaryCamera,
    ));

    // ドローンの生成
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.8, 1.0),
            ..default()
        })),
        Transform::from_xyz(0.8, 2.0, -0.8),
        Drone,
    ));

    // 障害物（TargetObstacle）の生成
    let obstacle_color = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.8),
        ..default()
    });

    let obstacles = [
        (Vec3::new(0.0, 5.0, -15.0), Vec3::new(6.0, 10.0, 6.0)),
        (Vec3::new(12.0, 8.0, -25.0), Vec3::new(8.0, 16.0, 8.0)),
        (Vec3::new(-15.0, 6.0, -20.0), Vec3::new(10.0, 12.0, 6.0)),
        (Vec3::new(0.0, 12.0, -40.0), Vec3::new(10.0, 24.0, 10.0)),
    ];

    for (pos, size) in obstacles {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(obstacle_color.clone()),
            Transform::from_translation(pos),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            TargetObstacle { size },
        ));
    }
}
