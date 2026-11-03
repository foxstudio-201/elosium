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
    match raycast(&world, origin, direction, max_distance) {
        Some((x, y, z)) => {
            transform.translation = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
            *visibility = Visibility::Inherited;
        }
        None => *visibility = Visibility::Hidden,
    }
}
