use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

// 1. コンポーネントとリソースの定義
#[derive(Component)]
struct Player;

#[derive(Component)]
struct PrimaryCamera;

#[derive(Resource)]
struct CameraSettings {
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

// 2. メイン関数
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CameraSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_camera, update_camera_and_player).chain())
        .run();
}

// 3. セットアップ（生成処理）
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ライト
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 床
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // プレイヤー
    commands.spawn((
        Player,
        Mesh3d(meshes.add(Cuboid::new(1.0, 2.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    // カメラ
    commands.spawn((
        PrimaryCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// 4. マウス操作によるカメラの回転
fn rotate_camera(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut settings: ResMut<CameraSettings>,
) {
    if mouse_motion.delta != Vec2::ZERO {
        settings.yaw -= mouse_motion.delta.x * settings.sensitivity;
        settings.pitch -= mouse_motion.delta.y * settings.sensitivity;
        settings.pitch = settings.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }
}

// 5. プレイヤーの移動とカメラ追従
fn update_camera_and_player(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<PrimaryCamera>)>,
    mut camera_query: Query<&mut Transform, (With<PrimaryCamera>, Without<Player>)>,
) {
    // single_mut() で Mut<Transform> を取得
    let mut player_transform = player_query.single_mut().expect("Failed to get player transform");
    let mut camera_transform = camera_query.single_mut().expect("Failed to get player transform");

    // 入力取得
    let mut input_dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { input_dir.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { input_dir.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { input_dir.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { input_dir.x += 1.0; }

    // カメラの向きに基づく移動方向計算
    let camera_yaw_rotation = Quat::from_rotation_y(settings.yaw);
    let forward = camera_yaw_rotation * Vec3::NEG_Z;
    let right = camera_yaw_rotation * Vec3::X;

    // 移動処理
    if input_dir != Vec3::ZERO {
        let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize();
        let speed = 5.0;
        player_transform.translation += move_dir * speed * time.delta_secs();
        player_transform.look_to(move_dir, Vec3::Y);
    }

    // カメラ追従処理
    let camera_rotation = Quat::from_euler(EulerRot::YXZ, settings.yaw, settings.pitch, 0.0);
    let offset = Vec3::new(0.0, 3.0, 7.0);

    camera_transform.rotation = camera_rotation;
    camera_transform.translation = player_transform.translation + camera_rotation * offset;
}
