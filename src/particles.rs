use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
};

use crate::{
    blocks::Block,
    frontend::GameRules,
    hud::Inventory,
    materials::{BlockMaterial, GameMaterials},
    menu::MenuState,
    pack_atlas::{
        BLOCK_ATLAS_SIZE, BLOCK_UV_RECTS, FLORA_ATLAS_SIZE, FLORA_UV_RECTS, LEAF_ATLAS_SIZE,
        LEAF_UV_RECTS,
    },
    player::Player,
    world::VoxelWorld,
};

const GRAVITY: f32 = 22.0;
const DRAG: f32 = 2.6;
const RESTITUTION: f32 = 0.28;
const FRICTION: f32 = 0.72;
const DEBRIS_LAYER: usize = 3;
const DROP_LIFETIME: f32 = 300.0;

#[derive(Message)]
pub struct BlockBroken {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: Block,
}

#[derive(Component)]
pub struct Debris {
    velocity: Vec3,
    life: f32,
    max_life: f32,
    half: f32,
    grounded: bool,
}

#[derive(Component)]
pub struct BlockDrop {
    pub block: Block,
    pub count: u32,
    velocity: Vec3,
    half: f32,
    age: f32,
    life: f32,
    grounded: bool,
}

type MeshKey = (Block, u8, u8, u8);

#[derive(Resource, Default)]
pub struct DebrisMeshes(HashMap<MeshKey, Handle<Mesh>>);

#[derive(Resource)]
pub struct DustTimer(f32);

impl Default for DustTimer {
    fn default() -> Self {
        Self(0.0)
    }
}

#[derive(Resource, Default)]
struct BreakTest(f32);

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BlockBroken>()
            .init_resource::<DebrisMeshes>()
            .init_resource::<DustTimer>()
            .init_resource::<BreakTest>()
            .add_systems(
                Update,
                (
                    spawn_break_debris,
                    spawn_sprint_dust,
                    update_debris,
                    (update_drops, merge_drops).chain(),
                    break_test,
                ),
            );
    }
}

fn next_rand(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*state >> 8) & 0xFFFF) as f32 / 65_535.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AtlasKind {
    Block,
    Flora,
    Leaves,
}

pub fn block_atlas_kind(block: Block) -> AtlasKind {
    if block.is_plant() {
        AtlasKind::Flora
    } else if block.is_leaves() {
        AtlasKind::Leaves
    } else {
        AtlasKind::Block
    }
}

fn atlas_uv(kind: AtlasKind, tile: u8) -> [f32; 4] {
    let (rects, size): (&[[f32; 4]], [u32; 2]) = match kind {
        AtlasKind::Block => (&BLOCK_UV_RECTS, BLOCK_ATLAS_SIZE),
        AtlasKind::Flora => (&FLORA_UV_RECTS, FLORA_ATLAS_SIZE),
        AtlasKind::Leaves => (&LEAF_UV_RECTS, LEAF_ATLAS_SIZE),
    };
    let rect = rects[(tile as usize).min(rects.len() - 1)];
    let inset_u = 0.25 / size[0] as f32;
    let inset_v = 0.25 / size[1] as f32;
    [
        rect[0] + inset_u,
        rect[1] + inset_v,
        rect[2] - inset_u,
        rect[3] - inset_v,
    ]
}

pub fn block_particle_material(materials: &GameMaterials, block: Block) -> Handle<BlockMaterial> {
    match block_atlas_kind(block) {
        AtlasKind::Block => materials.particle.clone(),
        AtlasKind::Flora => materials.particle_flora.clone(),
        AtlasKind::Leaves => materials.particle_leaves.clone(),
    }
}

pub fn block_item_mesh(meshes: &mut Assets<Mesh>, block: Block) -> Handle<Mesh> {
    meshes.add(cube_mesh(block, Vec3::ONE))
}

fn tint_key(tint: Vec3) -> (u8, u8, u8) {
    (
        (tint.x.clamp(0.0, 1.0) * 255.0).round() as u8,
        (tint.y.clamp(0.0, 1.0) * 255.0).round() as u8,
        (tint.z.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn debris_mesh(
    meshes: &mut Assets<Mesh>,
    cache: &mut DebrisMeshes,
    block: Block,
    tint: Vec3,
) -> Handle<Mesh> {
    let color = tint_key(tint);
    let key = (block, color.0, color.1, color.2);
    if let Some(handle) = cache.0.get(&key) {
        return handle.clone();
    }
    let handle = meshes.add(cube_mesh(block, tint));
    cache.0.insert(key, handle.clone());
    handle
}

fn cube_mesh(block: Block, tint: Vec3) -> Mesh {
    let kind = block_atlas_kind(block);
    let top = atlas_uv(kind, block.texture(1));
    let side = atlas_uv(kind, block.texture(0));
    let bottom = atlas_uv(kind, block.texture(-1));
    let white = Vec3::ONE;
    let grass = block.is_grass_block();
    let faces: [([[f32; 3]; 4], [f32; 3], [f32; 4], Vec3); 6] = [
        (
            [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
            [0.0, 1.0, 0.0],
            top,
            if grass { tint } else { white },
        ),
        (
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
            [0.0, -1.0, 0.0],
            bottom,
            white,
        ),
        (
            [
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
            ],
            [0.0, 0.0, -1.0],
            side,
            white,
        ),
        (
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [0.0, 0.0, 1.0],
            side,
            white,
        ),
        (
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
            [-1.0, 0.0, 0.0],
            side,
            white,
        ),
        (
            [
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
                [0.5, -0.5, 0.5],
            ],
            [1.0, 0.0, 0.0],
            side,
            white,
        ),
    ];
    let corners = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    let mut positions = Vec::with_capacity(40);
    let mut normals = Vec::with_capacity(40);
    let mut uvs = Vec::with_capacity(40);
    let mut colors = Vec::with_capacity(40);
    let mut indices = Vec::with_capacity(60);
    for (face, normal, rect, color) in faces {
        push_quad(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut colors,
            &mut indices,
            face,
            normal,
            rect,
            color,
            corners,
        );
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[allow(clippy::too_many_arguments)]
fn push_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    face: [[f32; 3]; 4],
    normal: [f32; 3],
    rect: [f32; 4],
    tint: Vec3,
    corners: [[f32; 2]; 4],
) {
    let start = positions.len() as u32;
    for (corner, uv) in face.iter().zip(corners.iter()) {
        positions.push(*corner);
        normals.push(normal);
        uvs.push([
            rect[0] + uv[0] * (rect[2] - rect[0]),
            rect[1] + uv[1] * (rect[3] - rect[1]),
        ]);
        colors.push([tint.x, tint.y, tint.z, 1.0]);
    }
    indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
}

fn spawn_break_debris(
    mut messages: MessageReader<BlockBroken>,
    mut commands: Commands,
    world: Res<VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<DebrisMeshes>,
    materials: Res<GameMaterials>,
    rules: Res<GameRules>,
    mut seed: Local<u32>,
) {
    for event in messages.read() {
        let block = event.block;
        if block.is_air() || block.is_water() {
            continue;
        }
        let tint = if block.is_grass_block() {
            world.grass_tint_at(event.x, event.z)
        } else {
            Vec3::ONE
        };
        let handle = debris_mesh(&mut meshes, &mut cache, block, tint);
        let material = block_particle_material(&materials, block);
        let center = Vec3::new(
            event.x as f32 + 0.5,
            event.y as f32 + 0.5,
            event.z as f32 + 0.5,
        );
        *seed ^= (event.x as u32)
            .wrapping_mul(73_856_093)
            ^ (event.y as u32).wrapping_mul(19_349_663)
            ^ (event.z as u32).wrapping_mul(83_492_791);
        for _ in 0..10 {
            let offset = Vec3::new(
                next_rand(&mut seed) - 0.5,
                next_rand(&mut seed) - 0.5,
                next_rand(&mut seed) - 0.5,
            ) * 0.62;
            let direction = Vec3::new(
                next_rand(&mut seed) - 0.5,
                next_rand(&mut seed) * 0.7 + 0.15,
                next_rand(&mut seed) - 0.5,
            )
            .normalize_or_zero();
            let speed = 1.4 + next_rand(&mut seed) * 2.4;
            let size = 0.10 + next_rand(&mut seed) * 0.07;
            commands.spawn((
                Mesh3d(handle.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(center + offset).with_scale(Vec3::splat(size)),
                Debris {
                    velocity: direction * speed + Vec3::Y * 1.2,
                    life: 0.85 + next_rand(&mut seed) * 0.35,
                    max_life: 1.2,
                    half: size * 0.5,
                    grounded: false,
                },
                NotShadowCaster,
                RenderLayers::layer(DEBRIS_LAYER),
            ));
        }
        if rules.gamemode != 0 {
            continue;
        }
        let drop_size = 0.28;
        let velocity = Vec3::new(
            next_rand(&mut seed) - 0.5,
            2.0 + next_rand(&mut seed) * 0.9,
            next_rand(&mut seed) - 0.5,
        ) * 1.6;
        commands.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(center).with_scale(Vec3::splat(drop_size)),
            BlockDrop {
                block,
                count: 1,
                velocity,
                half: drop_size * 0.5,
                age: 0.0,
                life: DROP_LIFETIME,
                grounded: false,
            },
            NotShadowCaster,
            RenderLayers::layer(DEBRIS_LAYER),
        ));
    }
}

fn spawn_sprint_dust(
    time: Res<Time>,
    menu: Res<MenuState>,
    world: Res<VoxelWorld>,
    players: Query<&Player, With<Camera3d>>,
    mut timer: ResMut<DustTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<DebrisMeshes>,
    materials: Res<GameMaterials>,
    mut seed: Local<u32>,
) {
    if menu.open {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let horizontal = Vec2::new(player.velocity.x, player.velocity.z).length();
    if !player.sprinting || !player.on_ground || horizontal < 4.5 {
        return;
    }
    timer.0 -= time.delta_secs();
    if timer.0 > 0.0 {
        return;
    }
    timer.0 = 0.085;
    let feet = player.render_feet;
    let ground = world.get_block(
        feet.x.floor() as i32,
        (feet.y - 0.2).floor() as i32,
        feet.z.floor() as i32,
    );
    if !ground.is_solid() || ground.is_water() {
        return;
    }
    let tint = if ground.is_grass_block() {
        world.grass_tint_at(feet.x.floor() as i32, feet.z.floor() as i32)
    } else {
        Vec3::ONE
    };
    let handle = debris_mesh(&mut meshes, &mut cache, ground, tint);
    let direction = Vec3::new(-player.velocity.x, 0.0, -player.velocity.z).normalize_or_zero();
    *seed ^= (feet.x * 31.0 + feet.z * 17.0) as u32;
    for _ in 0..2 {
        let pos = feet
            + Vec3::new(
                next_rand(&mut seed) - 0.5,
                0.08,
                next_rand(&mut seed) - 0.5,
            ) * 0.35
            - direction * 0.15;
        let size = 0.055 + next_rand(&mut seed) * 0.04;
        let velocity = -direction * (0.7 + next_rand(&mut seed) * 0.9)
            + Vec3::Y * (0.7 + next_rand(&mut seed) * 0.8);
        commands.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(materials.particle.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(size)),
            Debris {
                velocity,
                life: 0.38 + next_rand(&mut seed) * 0.22,
                max_life: 0.6,
                half: size * 0.5,
                grounded: false,
            },
            NotShadowCaster,
            RenderLayers::layer(DEBRIS_LAYER),
        ));
    }
}

fn step_physics(
    world: &VoxelWorld,
    mut position: Vec3,
    mut velocity: Vec3,
    half: f32,
    grounded: bool,
    dt: f32,
) -> (Vec3, Vec3, bool) {
    if !grounded {
        let drag = (1.0 - DRAG * dt).clamp(0.0, 1.0);
        velocity *= drag;
    }
    velocity.y -= GRAVITY * dt;
    if velocity.x != 0.0 {
        position.x += velocity.x * dt;
        if overlaps_solid(world, position, half) {
            position.x -= velocity.x * dt;
            velocity.x = -velocity.x * RESTITUTION;
        }
    }
    if velocity.z != 0.0 {
        position.z += velocity.z * dt;
        if overlaps_solid(world, position, half) {
            position.z -= velocity.z * dt;
            velocity.z = -velocity.z * RESTITUTION;
        }
    }
    let mut grounded = false;
    let _ = grounded;
    if velocity.y != 0.0 {
        position.y += velocity.y * dt;
        if overlaps_solid(world, position, half) {
            position.y -= velocity.y * dt;
            if velocity.y < 0.0 {
                velocity.x *= FRICTION;
                velocity.z *= FRICTION;
                if velocity.y.abs() < 0.9 {
                    velocity.y = 0.0;
                    grounded = true;
                } else {
                    velocity.y = -velocity.y * RESTITUTION;
                }
            } else {
                velocity.y = 0.0;
            }
        }
    }
    if grounded {
        let friction = (1.0 - 9.0 * dt).clamp(0.0, 1.0);
        velocity.x *= friction;
        velocity.z *= friction;
    }
    (position, velocity, grounded)
}

fn update_debris(
    time: Res<Time>,
    world: Res<VoxelWorld>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Transform, &mut Debris)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (entity, mut transform, mut debris) in &mut particles {
        debris.life -= dt;
        if debris.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let (position, velocity, grounded) = step_physics(
            &world,
            transform.translation,
            debris.velocity,
            debris.half,
            debris.grounded,
            dt,
        );
        debris.velocity = velocity;
        debris.grounded = grounded;
        transform.translation = position;
        let age = (debris.life / debris.max_life).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(debris.half * 2.0 * (0.45 + 0.55 * age));
    }
}

fn update_drops(
    time: Res<Time>,
    world: Res<VoxelWorld>,
    players: Query<&Player, With<Camera3d>>,
    mut inventory: ResMut<Inventory>,
    mut commands: Commands,
    mut drops: Query<(Entity, &mut Transform, &mut BlockDrop)>,
) {
    let dt = time.delta_secs().min(0.05);
    let player = players.single().ok();
    for (entity, mut transform, mut drop) in &mut drops {
        drop.life -= dt;
        drop.age += dt;
        if drop.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let (position, velocity, grounded) = step_physics(
            &world,
            transform.translation,
            drop.velocity,
            drop.half,
            drop.grounded,
            dt,
        );
        drop.velocity = velocity;
        drop.grounded = grounded;
        transform.translation = position;
        if let Some(player) = player {
            if drop.age > 0.5 && touches_player(player, position, drop.half) {
                let remaining = inventory.add_blocks(drop.block, drop.count);
                if remaining == 0 {
                    commands.entity(entity).despawn();
                } else {
                    drop.count = remaining;
                }
            }
        }
    }
}

fn touches_player(player: &Player, position: Vec3, half: f32) -> bool {
    let min = player.feet - Vec3::new(0.3, 0.0, 0.3);
    let max = player.feet + Vec3::new(0.3, 1.8, 0.3);
    let min = min - Vec3::new(1.0, 0.5, 1.0);
    let max = max + Vec3::new(1.0, 0.5, 1.0);
    let item_min = position - Vec3::splat(half);
    let item_max = position + Vec3::splat(half);
    item_min.x < max.x
        && item_max.x > min.x
        && item_min.y < max.y
        && item_max.y > min.y
        && item_min.z < max.z
        && item_max.z > min.z
}

fn merge_drops(
    mut commands: Commands,
    mut drops: Query<(Entity, &Transform, &mut BlockDrop)>,
) {
    let list: Vec<(Entity, Vec3, Block, u32)> = drops
        .iter()
        .map(|(entity, transform, drop)| (entity, transform.translation, drop.block, drop.count))
        .collect();
    let mut removed: Vec<Entity> = Vec::new();
    let mut counts: Vec<u32> = list.iter().map(|item| item.3).collect();
    for i in 0..list.len() {
        if removed.contains(&list[i].0) {
            continue;
        }
        for j in (i + 1)..list.len() {
            if removed.contains(&list[j].0) || list[i].2 != list[j].2 {
                continue;
            }
            if list[i].1.distance(list[j].1) > 0.6 {
                continue;
            }
            let total = counts[i] + counts[j];
            if total > 64 {
                continue;
            }
            counts[i] = total;
            removed.push(list[j].0);
        }
    }
    if removed.is_empty() {
        return;
    }
    for (entity, _, mut drop) in &mut drops {
        if removed.contains(&entity) {
            commands.entity(entity).despawn();
        }
    }
    for (index, (entity, _, _, _)) in list.iter().enumerate() {
        if removed.contains(entity) {
            continue;
        }
        if let Ok((_, _, mut drop)) = drops.get_mut(*entity) {
            drop.count = counts[index];
        }
    }
}

fn overlaps_solid(world: &VoxelWorld, center: Vec3, half: f32) -> bool {
    let min = center - Vec3::splat(half);
    let max = center + Vec3::splat(half);
    for x in min.x.floor() as i32..=max.x.floor() as i32 {
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                if world.get_block(x, y, z).is_solid() {
                    return true;
                }
            }
        }
    }
    false
}

fn break_test(
    time: Res<Time>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<crate::streaming::ChunkRenderer>,
    players: Query<&Player, With<Camera3d>>,
    mut timer: ResMut<BreakTest>,
    mut events: MessageWriter<BlockBroken>,
) {
    if std::env::var("ELOSIUM_BREAK_TEST").is_err() {
        return;
    }
    let mode = std::env::var("ELOSIUM_BREAK_TEST").ok();
    timer.0 += time.delta_secs();
    let interval = if mode.as_deref() == Some("3") { 0.12 } else { 0.7 };
    if timer.0 < interval {
        return;
    }
    timer.0 = 0.0;
    let Ok(player) = players.single() else {
        return;
    };
    if mode.as_deref() == Some("3") {
        let forward = Vec3::new(
            -player.yaw.sin() * player.pitch.cos(),
            player.pitch.sin(),
            -player.yaw.cos() * player.pitch.cos(),
        );
        let point = player.body + forward * 1.4;
        events.write(BlockBroken {
            x: point.x.floor() as i32,
            y: point.y.floor() as i32,
            z: point.z.floor() as i32,
            block: Block::GrassBlock,
        });
        return;
    }
    if mode.as_deref() == Some("2") {
        let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
        let point = player.body + forward * 1.5 - Vec3::Y * 0.5;
        let ground = world.get_block(
            player.feet.x.floor() as i32,
            (player.feet.y - 0.3).floor() as i32,
            player.feet.z.floor() as i32,
        );
        let block = if ground.is_solid() {
            ground
        } else {
            Block::Stone
        };
        events.write(BlockBroken {
            x: point.x.floor() as i32,
            y: point.y.floor() as i32,
            z: point.z.floor() as i32,
            block,
        });
        return;
    }
    let (origin, direction, max_distance) = crate::interaction::aim_ray(player);
    if let Some((x, y, z)) = crate::interaction::raycast(&world, origin, direction, max_distance) {
        let block = world.get_block(x, y, z);
        world.set_block_recorded(x, y, z, Block::Air);
        renderer.mark_block_dirty(x, y, z);
        events.write(BlockBroken { x, y, z, block });
    }
}
