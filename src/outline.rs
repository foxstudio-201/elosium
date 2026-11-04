use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    debug::HudVisible,
    interaction::{aim_ray, raycast},
    player::Player,
    menu::MenuState,
    world::VoxelWorld,
};

#[derive(Component)]
pub struct BlockOutline;

pub fn spawn_block_outline(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh());
    let root = commands
        .spawn((
            BlockOutline,
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
        ))
        .id();
    let edges: [([f32; 3], [f32; 3]); 12] = [
        ([0.0, 0.5, 0.5], [1.02, 0.025, 0.025]),
        ([0.0, 0.5, -0.5], [1.02, 0.025, 0.025]),
        ([0.0, -0.5, 0.5], [1.02, 0.025, 0.025]),
        ([0.0, -0.5, -0.5], [1.02, 0.025, 0.025]),
        ([0.5, 0.0, 0.5], [0.025, 1.02, 0.025]),
        ([-0.5, 0.0, 0.5], [0.025, 1.02, 0.025]),
        ([0.5, 0.0, -0.5], [0.025, 1.02, 0.025]),
        ([-0.5, 0.0, -0.5], [0.025, 1.02, 0.025]),
        ([0.5, 0.5, 0.0], [0.025, 0.025, 1.02]),
        ([-0.5, 0.5, 0.0], [0.025, 0.025, 1.02]),
        ([0.5, -0.5, 0.0], [0.025, 0.025, 1.02]),
        ([-0.5, -0.5, 0.0], [0.025, 0.025, 1.02]),
    ];
    for (translation, scale) in edges {
        let edge = commands
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::from(translation))
                    .with_scale(Vec3::from(scale)),
            ))
            .id();
        commands.entity(root).add_child(edge);
    }
}

pub fn update_block_outline(
    menu: Res<MenuState>,
    hud: Res<HudVisible>,
    players: Query<&Player, With<Camera3d>>,
    world: Res<VoxelWorld>,
    mut outline: Query<(&mut Transform, &mut Visibility), With<BlockOutline>>,
) {
    let Ok((mut transform, mut visibility)) = outline.single_mut() else {
        return;
    };
    if menu.open || !hud.0 {
        *visibility = Visibility::Hidden;
        return;
    }
    let Ok(player) = players.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let (origin, direction, max_distance) = aim_ray(player);
    let half = Vec3::splat(crate::player::PLAYER_WIDTH * 0.5 + 0.15);
    let body_min = player.feet - Vec3::new(half.x, 0.05, half.z);
    let body_max = player.feet + Vec3::new(half.x, crate::player::PLAYER_HEIGHT + 0.05, half.z);
    match raycast(&world, origin, direction, max_distance) {
        Some((x, y, z)) => {
            if world.get_block(x, y, z).is_plant() {
                *visibility = Visibility::Hidden;
                return;
            }
            let block_min = Vec3::new(x as f32, y as f32, z as f32);
            let block_max = block_min + Vec3::ONE;
            let overlaps_body = block_min.x < body_max.x
                && block_max.x > body_min.x
                && block_min.y < body_max.y
                && block_max.y > body_min.y
                && block_min.z < body_max.z
                && block_max.z > body_min.z;
            if overlaps_body {
                *visibility = Visibility::Hidden;
                return;
            }
            transform.translation = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
            *visibility = Visibility::Inherited;
        }
        None => *visibility = Visibility::Hidden,
    }
}
