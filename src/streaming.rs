use std::collections::{HashMap, HashSet};

use bevy::{camera::visibility::RenderLayers, light::NotShadowCaster, prelude::*};

use crate::{
    materials::GameMaterials,
    menu::MenuState,
    mesher,
    player::{Player, ViewMode},
    reflection::ReflectionCamera,
    world::{section_index, section_origin_y, ChunkPos, VoxelWorld},
};

pub const DEFAULT_RENDER_DISTANCE: i32 = 7;

#[derive(Resource)]
pub struct RenderSettings {
    pub distance: i32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            distance: DEFAULT_RENDER_DISTANCE,
        }
    }
}
const GENERATE_BUDGET: usize = 4;
const MESH_BUDGET: usize = 10;
const REMESH_BUDGET: usize = 8;
const GENERATE_BOOST: usize = 10;
const MESH_BOOST: usize = 40;
const REMESH_BOOST: usize = 20;

#[derive(Resource, Default)]
pub struct LoadWarmup(pub f32);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SectionKey {
    chunk: ChunkPos,
    section: i32,
}

#[derive(Default)]
struct SectionEntities {
    opaque: Option<Entity>,
    water: Option<Entity>,
    water_shadow: Option<Entity>,
    overlay: Option<Entity>,
    leaves: Option<Entity>,
    flora: Option<Entity>,
}

impl SectionEntities {
    fn despawn(&mut self, commands: &mut Commands) {
        for entity in [
            self.opaque.take(),
            self.water.take(),
            self.water_shadow.take(),
            self.overlay.take(),
            self.leaves.take(),
            self.flora.take(),
        ]
        .into_iter()
        .flatten()
        {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Resource, Default)]
pub struct ChunkRenderer {
    views: HashMap<SectionKey, SectionEntities>,
    dirty: HashSet<SectionKey>,
}

impl ChunkRenderer {
    pub fn reset(&mut self, commands: &mut Commands) {
        for (_, mut view) in self.views.drain() {
            view.despawn(commands);
        }
        self.dirty.clear();
    }

    pub fn view_count(&self) -> usize {
        self.views
            .values()
            .filter(|view| {
                view.opaque.is_some()
                    || view.water.is_some()
                    || view.overlay.is_some()
                    || view.leaves.is_some()
                    || view.flora.is_some()
            })
            .count()
    }

    pub fn mark_block_dirty(&mut self, x: i32, y: i32, z: i32) {        for dy in -1..=1 {
            let block_y = y + dy;
            let Some(section) = section_index(block_y) else {
                continue;
            };
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let chunk = ChunkPos::from_block(x + dx, z + dz);
                    self.dirty.insert(SectionKey { chunk, section });
                }
            }
        }
    }

    pub fn mark_occluder_dirty(&mut self, x: i32, y: i32, z: i32) {
        for (ox, oy, oz) in [
            (0, 0, 0),
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            let Some(section) = section_index(y + oy) else {
                continue;
            };
            let chunk = ChunkPos::from_block(x + ox, z + oz);
            self.dirty.insert(SectionKey { chunk, section });
        }
    }
}

pub fn update_occluders(
    menu: Res<MenuState>,
    view: Res<ViewMode>,
    player: Query<(&Player, &GlobalTransform), (With<Camera3d>, Without<ReflectionCamera>)>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
) {
    let mut next = HashSet::new();
    if !menu.open && view.mode == 2 {
        if let Ok((player, camera)) = player.single() {
            let eye = camera.translation();
            let target_center = player.body - Vec3::Y * 0.7;
            let forward = (target_center - eye).normalize_or_zero();
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let feet = player.feet;
            for height in [0.35, 0.95, 1.55] {
                for offset in [-0.3, 0.0, 0.3] {
                    let target = feet + Vec3::Y * height + right * offset;
                    let delta = target - eye;
                    let length = delta.length();
                    if length < 0.8 {
                        continue;
                    }
                    let direction = delta / length;
                    let mut distance = 0.0;
                    while distance < length - 0.5 {
                        let point = eye + direction * distance;
                        let (x, y, z) = (
                            point.x.floor() as i32,
                            point.y.floor() as i32,
                            point.z.floor() as i32,
                        );
                        if world.is_solid(x, y, z) {
                            next.insert((x, y, z));
                        }
                        distance += 0.15;
                    }
                }
            }
            let hits: Vec<(i32, i32, i32)> = next.iter().copied().collect();
            for (x, y, z) in hits {
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if world.is_solid(x + dx, y, z + dz) {
                        next.insert((x + dx, y, z + dz));
                    }
                }
                if world.is_solid(x, y + 1, z) {
                    next.insert((x, y + 1, z));
                }
            }
        }
    }
    if world.occluders() == &next {
        return;
    }
    let previous = world.occluders().clone();
    for (x, y, z) in previous.symmetric_difference(&next).copied() {
        renderer.mark_occluder_dirty(x, y, z);
    }
    world.set_occluders(next);
}

fn build_section_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &GameMaterials,
    world: &VoxelWorld,
    key: SectionKey,
) -> SectionEntities {
    let Some(chunk) = world.chunk(key.chunk) else {
        return SectionEntities::default();
    };
    if chunk.section_block_count(key.section) == 0 {
        return SectionEntities::default();
    }

    let data = mesher::build_section(world, key.chunk, key.section);
    let origin = Vec3::new(
        key.chunk.origin_x() as f32,
        section_origin_y(key.section) as f32,
        key.chunk.origin_z() as f32,
    );
    let mut view = SectionEntities::default();

    if let Some(mesh) = data.opaque {
        view.opaque = Some(
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh.build())),
                    MeshMaterial3d(materials.block.clone()),
                    Transform::from_translation(origin),
                ))
                .id(),
        );
    }
    if let Some(mesh) = data.water {
        view.water = Some(
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh.build())),
                    MeshMaterial3d(materials.water.clone()),
                    Transform::from_translation(origin),
                    RenderLayers::layer(1),
                    NotShadowCaster,
                ))
                .id(),
        );
    }
    if let Some(mesh) = data.water_shadow {
        view.water_shadow = Some(
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh.build())),
                    MeshMaterial3d(materials.water_shadow.clone()),
                    Transform::from_translation(origin),
                    RenderLayers::layer(2),
                ))
                .id(),
        );
    }
    if let Some(mesh) = data.overlay {
        view.overlay = Some(
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh.build())),
                    MeshMaterial3d(materials.wind.clone()),
                    Transform::from_translation(origin),
                ))
                .id(),
        );
    }
    if let Some(mesh) = data.leaves {
        view.leaves = Some(
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh.build())),
                    MeshMaterial3d(materials.leaves.clone()),
                    Transform::from_translation(origin),
                ))
                .id(),
        );
    }
    if let Some(mesh) = data.flora {
        view.flora = Some(
            commands
                .spawn((
                    Mesh3d(meshes.add(mesh.build())),
                    MeshMaterial3d(materials.flora.clone()),
                    Transform::from_translation(origin),
                ))
                .id(),
        );
    }

    view
}

pub fn stream_chunks(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<RenderSettings>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<GameMaterials>,
    mut warmup: ResMut<LoadWarmup>,
    camera: Query<&Transform, (With<Camera3d>, Without<ReflectionCamera>)>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    let boosted = warmup.0 > 0.0;
    if boosted {
        warmup.0 = (warmup.0 - time.delta_secs()).max(0.0);
    }
    let generate_budget = if boosted { GENERATE_BOOST } else { GENERATE_BUDGET };
    let mesh_budget = if boosted { MESH_BOOST } else { MESH_BUDGET };
    let remesh_budget = if boosted { REMESH_BOOST } else { REMESH_BUDGET };
    let center = ChunkPos::from_block(
        camera.translation.x.floor() as i32,
        camera.translation.z.floor() as i32,
    );

    let unload_distance = settings.distance + 2;
    let to_unload: Vec<ChunkPos> = world
        .loaded_chunks()
        .filter(|pos| {
            (pos.x - center.x).abs() > unload_distance
                || (pos.z - center.z).abs() > unload_distance
        })
        .collect();
    for pos in to_unload {
        if let Some(chunk) = world.chunk(pos) {
            for section in chunk.min_section..=chunk.max_section {
                let key = SectionKey { chunk: pos, section };
                if let Some(mut view) = renderer.views.remove(&key) {
                    view.despawn(&mut commands);
                }
                renderer.dirty.remove(&key);
            }
        }
        world.unload_chunk(pos);
    }

    let mut missing = Vec::new();
    for dx in -settings.distance..=settings.distance {
        for dz in -settings.distance..=settings.distance {
            let pos = ChunkPos::new(center.x + dx, center.z + dz);
            if !world.is_loaded(pos) {
                missing.push((dx * dx + dz * dz, pos));
            }
        }
    }
    missing.sort_by_key(|(distance, _)| *distance);
    for (_, pos) in missing.into_iter().take(generate_budget) {
        world.load_chunk(pos);
    }

    let dirty: Vec<SectionKey> = renderer.dirty.iter().copied().take(remesh_budget).collect();
    for key in dirty {
        renderer.dirty.remove(&key);
        if !world.is_loaded(key.chunk) {
            continue;
        }
        if let Some(mut view) = renderer.views.remove(&key) {
            view.despawn(&mut commands);
        }
        let view = build_section_entities(&mut commands, &mut meshes, &materials, &world, key);
        renderer.views.insert(key, view);
    }

    let mesh_distance = settings.distance - 1;
    let mut candidates = Vec::new();
    for dx in -mesh_distance..=mesh_distance {
        for dz in -mesh_distance..=mesh_distance {
            let pos = ChunkPos::new(center.x + dx, center.z + dz);
            if !world.is_loaded(pos) {
                continue;
            }
            let neighbors_loaded = [
                ChunkPos::new(pos.x + 1, pos.z),
                ChunkPos::new(pos.x - 1, pos.z),
                ChunkPos::new(pos.x, pos.z + 1),
                ChunkPos::new(pos.x, pos.z - 1),
            ]
            .iter()
            .all(|neighbor| world.is_loaded(*neighbor));
            if neighbors_loaded {
                candidates.push((dx * dx + dz * dz, pos));
            }
        }
    }
    candidates.sort_by_key(|(distance, _)| *distance);

    let mut budget = mesh_budget;
    'chunks: for (_, pos) in candidates {
        let Some(chunk) = world.chunk(pos) else {
            continue;
        };
        let mut sections: Vec<i32> = chunk.section_indices().collect();
        sections.sort_unstable();
        for section in sections {
            let key = SectionKey {
                chunk: pos,
                section,
            };
            if renderer.views.contains_key(&key) {
                continue;
            }
            if budget == 0 {
                break 'chunks;
            }
            if chunk.section_block_count(section) == 0 {
                continue;
            }
            let view = build_section_entities(&mut commands, &mut meshes, &materials, &world, key);
            renderer.views.insert(key, view);
            budget -= 1;
        }
    }
}
