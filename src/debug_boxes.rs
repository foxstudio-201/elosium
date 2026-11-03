use bevy::{camera::visibility::RenderLayers, light::NotShadowCaster, prelude::*};

use crate::{
    interaction::{aim_ray, raycast_point},
    menu::MenuState,
    player::{Player, PLAYER_HEIGHT, PLAYER_WIDTH},
    world::VoxelWorld,
};

#[derive(Resource)]
pub struct DebugBoxes {
    pub enabled: bool,
}

impl Default for DebugBoxes {
    fn default() -> Self {
        Self {
            enabled: std::env::var("ELOSIUM_HITBOX").is_ok(),
        }
    }
}

#[derive(Component)]
pub struct DebugHitbox;

#[derive(Component)]
pub struct DebugRay;

const EDGE: f32 = 0.018;
const RAY_THICKNESS: f32 = 0.012;

pub fn spawn_debug_boxes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh());
    let edge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        unlit: true,
        ..default()
    });
    let ray_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.68, 0.18),
        unlit: true,
        ..default()
    });

    let half = PLAYER_WIDTH * 0.5;
    let height = PLAYER_HEIGHT;
    let mut segments: Vec<(Vec3, Vec3)> = Vec::new();
    for (sx, sz) in [(-1.0f32, -1.0f32), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        segments.push((
            Vec3::new(sx * half, height * 0.5, sz * half),
            Vec3::new(EDGE, height, EDGE),
        ));
    }
    for y in [0.0f32, height] {
        for sz in [-1.0f32, 1.0] {
            segments.push((
                Vec3::new(0.0, y, sz * half),
                Vec3::new(PLAYER_WIDTH, EDGE, EDGE),
            ));
        }
        for sx in [-1.0f32, 1.0] {
            segments.push((
                Vec3::new(sx * half, y, 0.0),
                Vec3::new(EDGE, EDGE, PLAYER_WIDTH),
            ));
        }
    }

    let root = commands
        .spawn((
            DebugHitbox,
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
        ))
        .id();
    for (translation, scale) in segments {
        let segment = commands
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(edge_material.clone()),
                Transform::from_translation(translation).with_scale(scale),
                NotShadowCaster,
                RenderLayers::layer(1),
            ))
            .id();
        commands.entity(root).add_child(segment);
    }

    commands.spawn((
        DebugRay,
        Mesh3d(cube),
        MeshMaterial3d(ray_material),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        RenderLayers::layer(1),
    ));
}

pub fn toggle_debug_boxes(keys: Res<ButtonInput<KeyCode>>, mut boxes: ResMut<DebugBoxes>) {
    if keys.pressed(KeyCode::F3) && keys.just_pressed(KeyCode::KeyB) {
        boxes.enabled = !boxes.enabled;
    }
}

pub fn update_debug_boxes(
    boxes: Res<DebugBoxes>,
    menu: Res<MenuState>,
    players: Query<&Player, With<Camera3d>>,
    world: Res<VoxelWorld>,
    mut hitbox: Query<
        (&mut Transform, &mut Visibility),
        (With<DebugHitbox>, Without<DebugRay>),
    >,
    mut ray: Query<(&mut Transform, &mut Visibility), (With<DebugRay>, Without<DebugHitbox>)>,
) {
    let show = boxes.enabled && !menu.open;
    let Ok(player) = players.single() else {
        return;
    };

    if let Ok((mut transform, mut visibility)) = hitbox.single_mut() {
        if show {
            transform.translation = player.render_feet;
            transform.rotation = Quat::IDENTITY;
            transform.scale = Vec3::new(1.0, player.pose.height() / PLAYER_HEIGHT, 1.0);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    let Ok((mut transform, mut visibility)) = ray.single_mut() else {
        return;
    };
    if !show {
        *visibility = Visibility::Hidden;
        return;
    }
    let (origin, direction, max_distance) = aim_ray(player);
    let end = raycast_point(&world, origin, direction, max_distance)
        .unwrap_or(origin + direction * max_distance);
    let segment = end - origin;
    let length = segment.length();
    if length <= 0.01 {
        *visibility = Visibility::Hidden;
        return;
    }
    transform.translation = origin + segment * 0.5;
    transform.rotation = Quat::from_rotation_arc(Vec3::Z, segment / length);
    transform.scale = Vec3::new(RAY_THICKNESS, RAY_THICKNESS, length);
    *visibility = Visibility::Inherited;
}
