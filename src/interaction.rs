use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    debug::HudVisible,
    frontend::GameRules,
    menu::MenuState,
    player::{Player, ViewMode, PLAYER_WIDTH},
    reflection::ReflectionCamera,
    streaming::ChunkRenderer,
    world::{Block, VoxelWorld},
};

#[derive(Component)]
pub struct Crosshair;

#[derive(Resource, Default)]
pub struct BreakTimer {
    pub cooldown: f32,
}

pub(crate) fn aim_ray(player: &Player) -> (Vec3, Vec3, f32) {
    let direction = Quat::from_rotation_y(player.yaw)
        * Quat::from_rotation_x(player.pitch)
        * Vec3::NEG_Z;
    (player.body, direction, 5.5)
}

pub(crate) fn raycast_point(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<Vec3> {
    let mut distance = 0.0;
    while distance <= max_distance {
        let point = origin + direction * distance;
        if world.is_solid(
            point.x.floor() as i32,
            point.y.floor() as i32,
            point.z.floor() as i32,
        ) {
            return Some(origin + direction * (distance - 0.02).max(0.0));
        }
        distance += 0.05;
    }
    None
}

pub fn spawn_crosshair(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Crosshair,
        Mesh3d(meshes.add(crosshair_mesh())),
        MeshMaterial3d(material),
        Transform::default(),
        NotShadowCaster,
        RenderLayers::layer(1),
    ));
}

pub fn follow_camera(
    view: Res<ViewMode>,
    hud: Res<HudVisible>,
    camera: Query<&Transform, (With<Camera3d>, Without<Crosshair>, Without<ReflectionCamera>)>,
    mut crosshair: Query<(&mut Transform, &mut Visibility), With<Crosshair>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    if let Ok((mut transform, mut visibility)) = crosshair.single_mut() {
        *visibility = if view.mode == 2 || !hud.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        transform.translation = camera.translation + camera.rotation * Vec3::NEG_Z * 0.6;
        transform.rotation = camera.rotation;
    }
}

pub fn break_block(
    time: Res<Time>,
    menu: Res<MenuState>,
    rules: Res<GameRules>,
    view: Res<ViewMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions>,
    mut players: Query<&mut Player, With<Camera3d>>,
    mut timer: ResMut<BreakTimer>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
    mut broken: MessageWriter<crate::particles::BlockBroken>,
) {
    if rules.gamemode >= 2 {
        return;
    }
    let active = !menu.open && (cursor.grab_mode != CursorGrabMode::None || view.mode == 2);
    if !active || !mouse.pressed(MouseButton::Left) {
        timer.cooldown = 0.0;
        return;
    }
    timer.cooldown -= time.delta_secs();
    if timer.cooldown > 0.0 {
        return;
    }
    timer.cooldown = 0.25;
    let Ok((origin, direction, max_distance)) = players.single().map(aim_ray) else {
        return;
    };
    if let Some((x, y, z)) = raycast(&world, origin, direction, max_distance) {
        let hit_block = world.get_block(x, y, z);
        world.set_block_recorded(x, y, z, Block::Air);
        broken.write(crate::particles::BlockBroken {
            x,
            y,
            z,
            block: hit_block,
        });
        if world.get_block(x, y + 1, z).is_plant() {
            let plant = world.get_block(x, y + 1, z);
            world.set_block_recorded(x, y + 1, z, Block::Air);
            renderer.mark_block_dirty(x, y + 1, z);
            broken.write(crate::particles::BlockBroken {
                x,
                y: y + 1,
                z,
                block: plant,
            });
        }
        if let Ok(mut player) = players.single_mut() {
            player.swinging = true;
            player.swing = 0.0;
        }
        renderer.mark_block_dirty(x, y, z);
    }
}

pub fn place_block(
    menu: Res<MenuState>,
    rules: Res<GameRules>,
    mouse: Res<ButtonInput<MouseButton>>,
    view: Res<ViewMode>,
    cursor: Single<&CursorOptions>,
    mut players: Query<&mut Player, With<Camera3d>>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
) {
    if rules.gamemode >= 2 {
        return;
    }
    let active = !menu.open && (cursor.grab_mode != CursorGrabMode::None || view.mode == 2);
    if !active || !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok((origin, direction, max_distance)) = players.single().map(aim_ray) else {
        return;
    };
    let Some((hit_x, hit_y, hit_z)) = raycast(&world, origin, direction, max_distance) else {
        return;
    };
    let block = world.get_block(hit_x, hit_y, hit_z);
    if block == Block::Air {
        return;
    }
    let Some((x, y, z)) = place_cell(&world, origin, direction, max_distance) else {
        return;
    };
    let target = world.get_block(x, y, z);
    if !(target.is_air() || target.is_water() || target.is_plant()) {
        return;
    }
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    let feet = player.feet;
    if cell_intersects_player(x, y, z, feet, player.pose.height()) {
        return;
    }
    player.swinging = true;
    player.swing = 0.0;
    world.set_block_recorded(x, y, z, block);
    renderer.mark_block_dirty(x, y, z);
}

fn place_cell(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(i32, i32, i32)> {
    let mut distance = 0.0;
    let mut previous = None;
    while distance <= max_distance {
        let point = origin + direction * distance;
        let block = (
            point.x.floor() as i32,
            point.y.floor() as i32,
            point.z.floor() as i32,
        );
        if world.is_solid(block.0, block.1, block.2) {
            return previous;
        }
        previous = Some(block);
        distance += 0.05;
    }
    None
}

fn cell_intersects_player(x: i32, y: i32, z: i32, feet: Vec3, height: f32) -> bool {
    let min = feet - Vec3::new(PLAYER_WIDTH * 0.5, 0.0, PLAYER_WIDTH * 0.5);
    let max = feet + Vec3::new(PLAYER_WIDTH * 0.5, height, PLAYER_WIDTH * 0.5);
    let cell_min = Vec3::new(x as f32, y as f32, z as f32);
    let cell_max = cell_min + Vec3::ONE;
    min.x < cell_max.x
        && max.x > cell_min.x
        && min.y < cell_max.y
        && max.y > cell_min.y
        && min.z < cell_max.z
        && max.z > cell_min.z
}

pub fn raycast(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(i32, i32, i32)> {
    let mut distance = 0.0;
    while distance <= max_distance {
        let point = origin + direction * distance;
        let block = (
            point.x.floor() as i32,
            point.y.floor() as i32,
            point.z.floor() as i32,
        );
        if world.get_block(block.0, block.1, block.2).is_targetable() {
            return Some(block);
        }
        distance += 0.05;
    }
    None
}

fn crosshair_mesh() -> Mesh {
    let bars = [
        (-0.0012, -0.010, 0.0012, 0.010),
        (-0.010, -0.0012, 0.010, 0.0012),
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for (x0, y0, x1, y1) in bars {
        let base = positions.len() as u32;
        positions.extend_from_slice(&[
            [x0, y0, 0.0],
            [x1, y0, 0.0],
            [x1, y1, 0.0],
            [x0, y1, 0.0],
        ]);
        normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);
        uvs.extend_from_slice(&[[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
