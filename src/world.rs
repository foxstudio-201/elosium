use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    blocks::{self, Biome, PlantKind},
    storage::{ChunkData, WorldStorage},
    worldgen,
};

pub use crate::blocks::Block;

pub const CHUNK: i32 = 16;
pub const SECTION: i32 = 16;
pub const MIN_Y: i32 = -512;
pub const MAX_Y: i32 = 512;
pub const SEA_LEVEL: i32 = 63;
pub const WATER_SURFACE_HEIGHT: f32 = SEA_LEVEL as f32 + 0.875;
pub const SECTION_COUNT: i32 = (MAX_Y - MIN_Y) / SECTION;
pub const SECTION_VOLUME: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn from_block(x: i32, z: i32) -> Self {
        Self {
            x: x.div_euclid(CHUNK),
            z: z.div_euclid(CHUNK),
        }
    }

    pub fn origin_x(self) -> i32 {
        self.x * CHUNK
    }

    pub fn origin_z(self) -> i32 {
        self.z * CHUNK
    }
}

pub fn section_index(y: i32) -> Option<i32> {
    if !(MIN_Y..MAX_Y).contains(&y) {
        return None;
    }
    Some((y - MIN_Y).div_euclid(SECTION))
}

pub fn section_origin_y(index: i32) -> i32 {
    MIN_Y + index * SECTION
}

#[derive(Clone, Copy)]
pub struct ColumnInfo {
    pub height: i32,
    pub water_top: i32,
    pub biome: Biome,
    pub climate: [f32; 2],
    pub grass: Vec3,
    pub foliage: Vec3,
    pub stream: f32,
}

struct Section {
    blocks: Vec<Block>,
    count: usize,
}

impl Section {
    fn new() -> Self {
        Self {
            blocks: vec![Block::Air; SECTION_VOLUME],
            count: 0,
        }
    }

    fn filled(block: Block) -> Self {
        Self {
            blocks: vec![block; SECTION_VOLUME],
            count: if block == Block::Air { 0 } else { SECTION_VOLUME },
        }
    }

    fn set_deepslate(&mut self, origin_y: i32) {
        for ly in 0..SECTION {
            if origin_y + ly < worldgen::DEEPSLATE_TOP {
                let base = (ly * CHUNK * CHUNK) as usize;
                for index in base..base + (CHUNK * CHUNK) as usize {
                    self.blocks[index] = Block::Deepslate;
                }
            }
        }
    }

    fn index(lx: i32, ly: i32, lz: i32) -> usize {
        (lx + lz * SECTION + ly * SECTION * SECTION) as usize
    }

    fn get(&self, lx: i32, ly: i32, lz: i32) -> Block {
        self.blocks[Self::index(lx, ly, lz)]
    }

    fn set(&mut self, lx: i32, ly: i32, lz: i32, block: Block) {
        let index = Self::index(lx, ly, lz);
        let previous = self.blocks[index];
        if previous == Block::Air && block != Block::Air {
            self.count += 1;
        } else if previous != Block::Air && block == Block::Air {
            self.count -= 1;
        }
        self.blocks[index] = block;
    }
}

pub struct Chunk {
    pub columns: Vec<ColumnInfo>,
    sections: HashMap<i32, Section>,
    pub min_section: i32,
    pub max_section: i32,
}

impl Chunk {
    fn new() -> Self {
        Self {
            columns: Vec::new(),
            sections: HashMap::new(),
            min_section: SECTION_COUNT,
            max_section: -1,
        }
    }

    pub fn column(&self, lx: i32, lz: i32) -> ColumnInfo {
        self.columns[(lx + lz * CHUNK) as usize]
    }

    pub fn section_indices(&self) -> impl Iterator<Item = i32> + '_ {
        self.sections.keys().copied()
    }

    pub fn section_block_count(&self, index: i32) -> usize {
        self.sections.get(&index).map_or(0, |section| section.count)
    }

    pub fn section_solid_count(&self, index: i32) -> usize {
        let Some(section) = self.sections.get(&index) else {
            return 0;
        };
        let mut count = 0;
        for ly in 0..SECTION {
            for lz in 0..SECTION {
                for lx in 0..SECTION {
                    if section.get(lx, ly, lz).is_solid() {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    pub fn get_block_local(&self, lx: i32, y: i32, lz: i32) -> Block {
        let index = match section_index(y) {
            Some(index) => index,
            None => return Block::Air,
        };
        if let Some(section) = self.sections.get(&index) {
            return section.get(lx, y - section_origin_y(index), lz);
        }
        let column = self.column(lx, lz);
        if y <= column.height {
            Block::Stone
        } else if y <= column.water_top {
            Block::Water
        } else {
            Block::Air
        }
    }

    pub(crate) fn set_block_local(&mut self, lx: i32, y: i32, lz: i32, block: Block) {
        let index = match section_index(y) {
            Some(index) => index,
            None => return,
        };
        if block == Block::Air {
            if let Some(section) = self.sections.get_mut(&index) {
                section.set(lx, y - section_origin_y(index), lz, block);
            }
            return;
        }
        let section = self.sections.entry(index).or_insert_with(Section::new);
        section.set(lx, y - section_origin_y(index), lz, block);
        self.min_section = self.min_section.min(index);
        self.max_section = self.max_section.max(index);
    }

    fn insert_section(&mut self, index: i32, section: Section) {
        self.sections.insert(index, section);
        self.min_section = self.min_section.min(index);
        self.max_section = self.max_section.max(index);
    }

    fn from_data(data: ChunkData) -> Self {
        let mut chunk = Chunk::new();
        chunk.columns = data.columns;
        chunk.min_section = data.min_section;
        chunk.max_section = data.max_section;
        for (index, blocks) in data.sections {
            let count = blocks.iter().filter(|block| !block.is_air()).count();
            chunk.sections.insert(index, Section { blocks, count });
        }
        chunk
    }

    pub(crate) fn block_slices(&self) -> Vec<(i32, &[Block])> {
        self.sections
            .iter()
            .map(|(&index, section)| (index, section.blocks.as_slice()))
            .collect()
    }

    fn generate(&mut self, pos: ChunkPos, seed: u32) {
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let world_x = pos.origin_x() + lx;
                let world_z = pos.origin_z() + lz;
                let column = worldgen::column(world_x, world_z, seed);
                let mut grass = Vec3::ZERO;
                let mut foliage = Vec3::ZERO;
                let mut total = 0.0;
                for (sample, weight) in column.biome_samples {
                    let info = blocks::info(sample);
                    grass += blocks::srgb(info.grass) * weight;
                    foliage += blocks::srgb(info.foliage) * weight;
                    total += weight;
                }
                self.columns.push(ColumnInfo {
                    height: column.height,
                    water_top: column.water_top,
                    biome: column.biome,
                    climate: column.climate,
                    grass: grass / total,
                    foliage: foliage / total,
                    stream: column.stream,
                });
            }
        }

        self.generate_strata();
        self.apply_bedrock(pos, seed);
        self.apply_surfaces(pos, seed);
        if worldgen::world_type() != 1 {
            worldgen::scatter_ores(self, pos, seed);
            worldgen::carve_caves(self, pos, seed);
            self.plant_trees(pos, seed);
            self.plant_ground_cover(pos, seed);
            self.plant_underwater_cover(pos, seed);
        }
    }

    fn generate_strata(&mut self) {
        let min_height = self.columns.iter().map(|column| column.height).min().unwrap_or(MIN_Y);
        let max_height = self.columns.iter().map(|column| column.height).max().unwrap_or(MIN_Y);
        let Some(first) = section_index(MIN_Y) else {
            return;
        };
        let Some(last) = section_index(max_height) else {
            return;
        };
        for index in first..=last {
            let origin = section_origin_y(index);
            if origin + SECTION - 1 <= min_height {
                let mut section = Section::filled(Block::Stone);
                section.set_deepslate(origin);
                self.insert_section(index, section);
            } else {
                for ly in 0..SECTION {
                    let y = origin + ly;
                    for lz in 0..CHUNK {
                        for lx in 0..CHUNK {
                            if y <= self.column(lx, lz).height {
                                let block = if y < worldgen::DEEPSLATE_TOP {
                                    Block::Deepslate
                                } else {
                                    Block::Stone
                                };
                                self.set_block_local(lx, y, lz, block);
                            }
                        }
                    }
                }
            }
        }
    }

    fn apply_bedrock(&mut self, pos: ChunkPos, seed: u32) {
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let world_x = pos.origin_x() + lx;
                let world_z = pos.origin_z() + lz;
                let rough = (hash2(world_x, world_z, seed ^ 0x21f0_5d9b) * 3.0) as i32;
                for y in MIN_Y..(MIN_Y + 2 + rough).min(MIN_Y + 8) {
                    self.set_block_local(lx, y, lz, Block::Bedrock);
                }
            }
        }
    }

    fn apply_surfaces(&mut self, pos: ChunkPos, seed: u32) {
        if worldgen::world_type() == 1 {
            for lz in 0..CHUNK {
                for lx in 0..CHUNK {
                    self.set_block_local(lx, SEA_LEVEL, lz, Block::GrassBlock);
                    for y in (SEA_LEVEL - 3)..SEA_LEVEL {
                        self.set_block_local(lx, y, lz, Block::Dirt);
                    }
                }
            }
            return;
        }
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let column = self.column(lx, lz);
                let info = blocks::info(column.biome);
                let height = column.height;
                let underwater = height < column.water_top;
                let world_x = pos.origin_x() + lx;
                let world_z = pos.origin_z() + lz;
                let patch = fbm(world_x as f32 * 0.032, world_z as f32 * 0.032, seed ^ 0x2c7b_51d9);
                let dither = hash2(world_x, world_z, seed ^ 0x63a9_0f2d);
                let near_water = self.near_water(lx, lz, height);
                let snow_line = column.climate[0]
                    - (height.max(64) - 64) as f32 * (0.05 / 30.0);
                let snowy = info.surface == Block::SnowBlock || snow_line < 0.15;
                let mut surface = if column.stream > 0.0 {
                    if column.biome == Biome::Rainforest {
                        Block::Mud
                    } else if patch < 0.35 {
                        Block::Sand
                    } else {
                        Block::Gravel
                    }
                } else if underwater {
                    if column.biome == Biome::Ocean {
                        if patch < 0.34 {
                            Block::Sand
                        } else if patch < 0.82 {
                            Block::Gravel
                        } else {
                            Block::Clay
                        }
                    } else if patch < 0.42 {
                        Block::Sand
                    } else if patch < 0.72 {
                        Block::Gravel
                    } else {
                        info.underwater
                    }
                } else if column.biome == Biome::Beach {
                    if patch < 0.72 {
                        Block::Sand
                    } else {
                        Block::Gravel
                    }
                } else if near_water && height <= SEA_LEVEL + 4 {
                    if patch < 0.68 {
                        Block::Sand
                    } else {
                        Block::Gravel
                    }
                } else {
                    let palette = blocks::surfaces(column.biome);
                    let total: f32 = palette.iter().map(|(_, weight)| weight).sum();
                    let roll = (dither * 0.55 + patch * 0.65).fract() * total;
                    let mut cursor = roll;
                    let mut picked = palette[0].0;
                    for &(block, weight) in palette {
                        if cursor < weight {
                            picked = block;
                            break;
                        }
                        cursor -= weight;
                    }
                    picked
                };
                if !underwater {
                    if snowy {
                        surface = Block::SnowBlock;
                    } else if surface == Block::SnowBlock {
                        surface = if info.surface == Block::SnowBlock {
                            Block::GrassBlock
                        } else {
                            info.surface
                        };
                    }
                }
                let filler = if underwater || surface == Block::Sand || surface == Block::Gravel {
                    if surface == Block::Sand {
                        Block::Sand
                    } else if surface == Block::Gravel {
                        Block::Gravel
                    } else {
                        info.underwater
                    }
                } else {
                    info.filler
                };
                self.set_block_local(lx, height, lz, surface);
                for y in (height - 3)..height {
                    self.set_block_local(lx, y, lz, filler);
                }
                for y in (height + 1).max(MIN_Y)..=column.water_top {
                    let block = if y == column.water_top && snowy {
                        if column.biome == Biome::IcyHeights {
                            Block::PackedIce
                        } else {
                            Block::Ice
                        }
                    } else {
                        Block::Water
                    };
                    self.set_block_local(lx, y, lz, block);
                }
            }
        }
    }

    fn near_water(&self, lx: i32, lz: i32, height: i32) -> bool {
        for dz in -2..=2 {
            for dx in -2..=2 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let x = lx + dx;
                let z = lz + dz;
                if !(0..CHUNK).contains(&x) || !(0..CHUNK).contains(&z) {
                    continue;
                }
                let neighbor = self.column(x, z);
                if neighbor.height < neighbor.water_top && neighbor.water_top >= height - 2 {
                    return true;
                }
            }
        }
        false
    }

    fn plant_underwater_cover(&mut self, pos: ChunkPos, seed: u32) {
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let column = self.column(lx, lz);
                let depth = column.water_top - column.height;
                if depth < 1 {
                    continue;
                }
                let surface = self.get_block_local(lx, column.height, lz);
                if !matches!(
                    surface,
                    Block::Sand
                        | Block::Gravel
                        | Block::Dirt
                        | Block::Clay
                        | Block::Mud
                        | Block::Stone
                        | Block::MossBlock
                ) {
                    continue;
                }
                let world_x = pos.origin_x() + lx;
                let world_z = pos.origin_z() + lz;
                let ocean = column.biome == Biome::Ocean;
                let river = column.stream > 0.0;
                let chance = if ocean {
                    0.22
                } else if river {
                    0.16
                } else {
                    0.07
                };
                if hash2(world_x, world_z, seed ^ 0x3ac1_8d27) < chance {
                    self.set_block_local(lx, column.height + 1, lz, Block::Seagrass);
                    if depth >= 2 && hash2(world_x, world_z, seed ^ 0x5b1f_9a31) < 0.35 {
                        self.set_block_local(lx, column.height + 2, lz, Block::Seagrass);
                    }
                }
                if ocean
                    && depth >= 5
                    && hash2(world_x, world_z, seed ^ 0x6d2b_44c1) < 0.035
                {
                    let span = (depth - 1).min(10) as f32;
                    let height = 2 + (hash2(world_x, world_z, seed ^ 0x1c3f_77a9) * span) as i32;
                    let top = (column.height + 1 + height).min(column.water_top);
                    for y in (column.height + 1)..top {
                        self.set_block_local(lx, y, lz, Block::KelpPlant);
                    }
                    self.set_block_local(lx, top, lz, Block::Kelp);
                }
            }
        }
    }

    fn plant_ground_cover(&mut self, pos: ChunkPos, seed: u32) {
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let column = self.column(lx, lz);
                if column.height < column.water_top {
                    continue;
                }
                let surface = self.get_block_local(lx, column.height, lz);
                if !(surface.is_soil() || surface == Block::Sand || surface == Block::MossBlock) {
                    continue;
                }
                if !self.get_block_local(lx, column.height + 1, lz).is_air() {
                    continue;
                }
                let info = blocks::info(column.biome);
                let world_x = pos.origin_x() + lx;
                let world_z = pos.origin_z() + lz;
                if matches!(
                    column.biome,
                    Biome::WillowForest | Biome::Rainforest | Biome::BambooForest
                ) && column.height <= SEA_LEVEL + 1
                    && hash2(world_x, world_z, seed ^ 0x1b7e_42c9) < 0.22
                {
                    self.set_block_local(lx, column.height + 1, lz, Block::Cattail);
                    continue;
                }
                if matches!(
                    column.biome,
                    Biome::Rainforest | Biome::BambooForest | Biome::Outback
                ) && column.height <= SEA_LEVEL + 1
                    && hash2(world_x, world_z, seed ^ 0x6c1d_2f47) < 0.18
                {
                    self.set_block_local(lx, column.height + 1, lz, Block::SugarCane);
                    if hash2(world_x, world_z, seed ^ 0x3f7a_19b1) < 0.5 {
                        self.set_block_local(lx, column.height + 2, lz, Block::SugarCane);
                    }
                    continue;
                }
                if surface == Block::Sand
                    && column.height <= SEA_LEVEL + 2
                    && hash2(world_x, world_z, seed ^ 0x4d1b_8ee3) < 0.08
                {
                    self.set_block_local(lx, column.height + 1, lz, Block::BeachGrass);
                    continue;
                }
                if info.plant_density <= 0.0 {
                    continue;
                }
                let patch = fbm(world_x as f32 * 0.018, world_z as f32 * 0.018, seed ^ 0x5b1f_0c93);
                let belt = fbm(world_x as f32 * 0.0035, world_z as f32 * 0.0035, seed ^ 0x2d71_8a3f);
                let density =
                    (info.plant_density * 2.8 * (0.15 + patch * 2.0) * (0.45 + belt * 1.3))
                        .min(0.95);
                let roll = hash2(world_x, world_z, seed ^ 0x51f1_0a11);
                if roll >= density {
                    continue;
                }
                let pick = hash2(world_x, world_z, seed ^ 0x2b1f_9e13);
                let plant = match info.plants {
                    PlantKind::None => continue,
                    PlantKind::Grass => {
                        if pick < 0.04 {
                            if column.climate[1] > 0.55 {
                                Block::BrownMushroom
                            } else {
                                Block::RedMushroom
                            }
                        } else if pick < 0.32 {
                            Block::Fern
                        } else if pick < 0.5 {
                            Block::HighGrass
                        } else {
                            Block::ShortGrass
                        }
                    }
                    PlantKind::Fern => {
                        if pick < 0.06 {
                            Block::BrownMushroom
                        } else if pick < 0.72 {
                            Block::Fern
                        } else if pick < 0.86 {
                            Block::HighGrass
                        } else {
                            Block::ShortGrass
                        }
                    }
                    PlantKind::Flowers => {
                        if pick < 0.35 {
                            flower_block(column.climate[0], pick)
                        } else {
                            Block::ShortGrass
                        }
                    }
                    PlantKind::FlowerField => {
                        if pick < 0.45 {
                            flower_block(column.climate[0], pick)
                        } else if pick < 0.62 && column.biome == Biome::FlowerFields {
                            Block::Daisy
                        } else if pick < 0.72 && column.biome == Biome::CloverPlains {
                            Block::Clover
                        } else {
                            Block::ShortGrass
                        }
                    }
                    PlantKind::Prairie => {
                        if pick < 0.12 {
                            flower_block(column.climate[0], pick)
                        } else if pick < 0.26 {
                            Block::Clover
                        } else if pick < 0.4 && column.climate[0] > 0.7 {
                            Block::DesertGrass
                        } else if pick < 0.5 && column.climate[1] < 0.45 {
                            Block::DuneGrass
                        } else if pick < 0.66 {
                            Block::PrairieGrass
                        } else {
                            Block::ShortGrass
                        }
                    }
                    PlantKind::DeadBush => {
                        if pick < 0.3 {
                            Block::DeadGrass
                        } else if pick < 0.42 {
                            Block::Sprout
                        } else {
                            Block::DeadBush
                        }
                    }
                    PlantKind::Mushroom => {
                        if pick < 0.5 {
                            Block::RedMushroom
                        } else {
                            Block::BrownMushroom
                        }
                    }
                };
                self.set_block_local(lx, column.height + 1, lz, plant);
            }
        }
    }

    fn plant_trees(&mut self, pos: ChunkPos, seed: u32) {
        let cell_size = 5i32;
        let cells = CHUNK / cell_size;
        let mut placed: Vec<(i32, i32)> = Vec::new();
        for cell_z in 0..cells {
            for cell_x in 0..cells {
                let base_x = cell_x * cell_size;
                let base_z = cell_z * cell_size;
                let center_x = base_x + cell_size / 2;
                let center_z = base_z + cell_size / 2;
                let center = self.column(center_x, center_z);
                if center.height < center.water_top {
                    continue;
                }
                let info = blocks::info(center.biome);
                let total: f32 = info.trees.iter().map(|(_, chance)| *chance).sum();
                if total <= 0.0 {
                    continue;
                }
                let world_center_x = pos.origin_x() + center_x;
                let world_center_z = pos.origin_z() + center_z;
                let roll = hash2(world_center_x, world_center_z, seed ^ 0x6d2b_79f5);
                let density = (total * (cell_size * cell_size) as f32).min(0.9);
                if roll >= density {
                    continue;
                }
                let jitter_x = (hash2(world_center_x, world_center_z, seed ^ 0x3f71_a2d9)
                    * cell_size as f32) as i32;
                let jitter_z = (hash2(world_center_x, world_center_z, seed ^ 0x5b8d_11c7)
                    * cell_size as f32) as i32;
                let lx = (base_x + jitter_x).clamp(2, CHUNK - 3);
                let lz = (base_z + jitter_z).clamp(2, CHUNK - 3);
                if placed
                    .iter()
                    .any(|&(px, pz)| (px - lx).abs() + (pz - lz).abs() < 3)
                {
                    continue;
                }
                let column = self.column(lx, lz);
                if column.height < column.water_top {
                    continue;
                }
                let ground = self.get_block_local(lx, column.height, lz);
                if !(ground.is_soil() || ground == Block::Sand || ground == Block::RedSand) {
                    continue;
                }
                if !self.get_block_local(lx, column.height + 1, lz).is_air() {
                    continue;
                }
                let biome_info = blocks::info(column.biome);
                let total: f32 = biome_info.trees.iter().map(|(_, chance)| *chance).sum();
                if total <= 0.0 {
                    continue;
                }
                let world_x = pos.origin_x() + lx;
                let world_z = pos.origin_z() + lz;
                let mut pick = hash2(world_x, world_z, seed ^ 0x6d2b_79f5) * total;
                let mut kind = biome_info.trees[0].0;
                for (candidate, chance) in biome_info.trees {
                    if pick < *chance {
                        kind = *candidate;
                        break;
                    }
                    pick -= *chance;
                }
                let tree_seed = seed ^ hash2(world_x, world_z, seed ^ 0x7e11_55c3).to_bits();
                worldgen::plant_tree(self, lx, lz, column.height, kind, tree_seed);
                placed.push((lx, lz));
            }
        }
    }
}

#[derive(Resource)]
pub struct VoxelWorld {
    chunks: HashMap<ChunkPos, Chunk>,
    flow: HashMap<(i32, i32, i32), u8>,
    occluders: HashSet<(i32, i32, i32)>,
    edits: HashMap<(i32, i32, i32), Block>,
    seed: u32,
    storage: Option<WorldStorage>,
}

impl VoxelWorld {
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            flow: HashMap::new(),
            occluders: HashSet::new(),
            edits: HashMap::new(),
            seed,
            storage: None,
        }
    }

    pub fn set_storage(&mut self, storage: Option<WorldStorage>) {
        self.storage = storage;
    }

    pub fn find_land_spawn(&mut self, center: ChunkPos) -> Vec3 {
        let mut best: Option<(i32, i32, i32)> = None;
        let mut best_score = i32::MIN;
        for dx in (-512..=512).step_by(32) {
            for dz in (-512..=512).step_by(32) {
                let x = center.origin_x() + 8 + dx;
                let z = center.origin_z() + 8 + dz;
                let column = worldgen::column(x, z, self.seed);
                if column.height <= SEA_LEVEL + 2
                    || matches!(column.biome, Biome::Ocean | Biome::Beach)
                {
                    continue;
                }
                let mut score = 0;
                for offset_x in [-96, 0, 96] {
                    for offset_z in [-96, 0, 96] {
                        let sample = worldgen::column(x + offset_x, z + offset_z, self.seed);
                        if sample.height > SEA_LEVEL + 2
                            && !matches!(sample.biome, Biome::Ocean | Biome::Beach)
                        {
                            score += 2;
                        } else if sample.height < SEA_LEVEL {
                            score -= 1;
                        }
                        if (SEA_LEVEL + 2..=SEA_LEVEL + 60).contains(&sample.height) {
                            score += 1;
                        }
                    }
                }
                if score > best_score {
                    best_score = score;
                    best = Some((x, z, column.height));
                }
            }
        }
        if let Some((x, z, height)) = best {
            let chunk = ChunkPos::from_block(x, z);
            self.load_chunk(chunk);
            for ox in (-8..=8).step_by(1) {
                for oz in (-8..=8).step_by(1) {
                    let sx = x + ox;
                    let sz = z + oz;
                    let pos = ChunkPos::from_block(sx, sz);
                    if !self.is_loaded(pos) {
                        self.load_chunk(pos);
                    }
                    let height = self.surface_height(sx, sz);
                    if height <= SEA_LEVEL || height >= MAX_Y - 32 {
                        continue;
                    }
                    if let Some(info) = self.column_at(sx, sz)
                        && info.height < info.water_top
                    {
                        continue;
                    }
                    if !self.get_block(sx, height, sz).is_soil() {
                        continue;
                    }
                    if (1..=4).any(|dy| !self.get_block(sx, height + dy, sz).is_air()) {
                        continue;
                    }
                    return Vec3::new(
                        sx as f32 + 0.5,
                        (height + 1) as f32 + 0.01,
                        sz as f32 + 0.5,
                    );
                }
            }
            return Vec3::new(x as f32 + 0.5, (height + 1) as f32 + 0.01, z as f32 + 0.5);
        }
        let x = center.origin_x() + 8;
        let z = center.origin_z() + 8;
        Vec3::new(
            x as f32 + 0.5,
            (self.surface_height(x, z) + 1) as f32 + 0.01,
            z as f32 + 0.5,
        )
    }

    pub fn edits(&self) -> &HashMap<(i32, i32, i32), Block> {
        &self.edits
    }

    pub fn load_edits(&mut self, edits: HashMap<(i32, i32, i32), Block>) {
        self.edits = edits;
    }

    pub fn set_block_recorded(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if block != Block::Water {
            self.edits.insert((x, y, z), block);
        }
        self.set_block(x, y, z, block);
    }

    pub fn occluders(&self) -> &HashSet<(i32, i32, i32)> {
        &self.occluders
    }

    pub fn set_occluders(&mut self, occluders: HashSet<(i32, i32, i32)>) {
        self.occluders = occluders;
    }

    pub fn is_occluded(&self, x: i32, y: i32, z: i32) -> bool {
        self.occluders.contains(&(x, y, z))
    }

    pub fn is_solid_rendered(&self, x: i32, y: i32, z: i32) -> bool {
        self.is_solid(x, y, z) && !self.is_occluded(x, y, z)
    }

    pub fn water_level(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        self.flow.get(&(x, y, z)).copied()
    }

    pub fn set_water(&mut self, x: i32, y: i32, z: i32, level: u8) {
        self.set_block(x, y, z, Block::Water);
        self.flow.insert((x, y, z), level.min(7));
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    pub fn load_chunk(&mut self, pos: ChunkPos) -> bool {
        if self.chunks.contains_key(&pos) {
            return false;
        }
        let cached = self
            .storage
            .as_ref()
            .and_then(|storage| storage.load_chunk(pos));
        let mut chunk = match cached {
            Some(data) => Chunk::from_data(data),
            None => {
                let mut chunk = Chunk::new();
                chunk.generate(pos, self.seed);
                if let Some(storage) = self.storage.as_ref() {
                    let sections = chunk.block_slices();
                    storage.save_chunk(
                        pos,
                        &crate::storage::ChunkView {
                            columns: &chunk.columns,
                            sections: &sections,
                            min_section: chunk.min_section,
                            max_section: chunk.max_section,
                        },
                    );
                }
                chunk
            }
        };
        for (&(x, y, z), &block) in self.edits.iter() {
            if ChunkPos::from_block(x, z) == pos {
                chunk.set_block_local(x - pos.origin_x(), y, z - pos.origin_z(), block);
            }
        }
        self.chunks.insert(pos, chunk);
        true
    }

    pub fn unload_chunk(&mut self, pos: ChunkPos) -> bool {
        self.flow
            .retain(|(x, _, z), _| ChunkPos::from_block(*x, *z) != pos);
        self.chunks.remove(&pos).is_some()
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    pub fn loaded_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(MIN_Y..MAX_Y).contains(&y) {
            return Block::Air;
        }
        let pos = ChunkPos::from_block(x, z);
        let Some(chunk) = self.chunks.get(&pos) else {
            return Block::Air;
        };
        let lx = x - pos.origin_x();
        let lz = z - pos.origin_z();
        chunk.get_block_local(lx, y, lz)
    }

    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z).is_solid()
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if !(MIN_Y..MAX_Y).contains(&y) {
            return;
        }
        let pos = ChunkPos::from_block(x, z);
        if let Some(chunk) = self.chunks.get_mut(&pos) {
            if block != Block::Water {
                self.flow.remove(&(x, y, z));
            }
            chunk.set_block_local(x - pos.origin_x(), y, z - pos.origin_z(), block);
        }
    }

    pub fn column_at(&self, x: i32, z: i32) -> Option<ColumnInfo> {
        let pos = ChunkPos::from_block(x, z);
        self.chunks
            .get(&pos)
            .map(|chunk| chunk.column(x - pos.origin_x(), z - pos.origin_z()))
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.column_at(x, z)
            .map_or_else(|| worldgen::height_at(x, z, self.seed), |column| column.height)
    }

    pub fn grass_tint_at(&self, x: i32, z: i32) -> Vec3 {
        self.column_at(x, z).map_or(Vec3::ONE, |column| column.grass)
    }

    pub fn foliage_tint_at(&self, x: i32, z: i32) -> Vec3 {
        self.column_at(x, z)
            .map_or(Vec3::ONE, |column| column.foliage)
    }
}

fn flower_block(temperature: f32, roll: f32) -> Block {
    let choices: &[Block] = if temperature < 0.28 {
        &[
            Block::WhiteTulip,
            Block::PinkTulip,
            Block::OxeyeDaisy,
            Block::LilyOfTheValley,
            Block::Crocus,
            Block::BlueSage,
            Block::Sprout,
        ]
    } else if temperature < 0.5 {
        &[
            Block::Dandelion,
            Block::Poppy,
            Block::Cornflower,
            Block::OxeyeDaisy,
            Block::Lavender,
            Block::IncanLily,
            Block::Amaranth,
        ]
    } else if temperature < 0.72 {
        &[
            Block::Allium,
            Block::AzureBluet,
            Block::Cornflower,
            Block::Poppy,
            Block::JapaneseOrchid,
            Block::PinkAllium,
            Block::Hydrangea,
            Block::Rose,
            Block::FirecrackerFlower,
        ]
    } else {
        &[
            Block::OrangeTulip,
            Block::RedTulip,
            Block::PinkTulip,
            Block::LilyOfTheValley,
            Block::CyanAmaranth,
            Block::BlackRose,
            Block::PinkDaffodil,
            Block::Glowflower,
            Block::IncanLily,
        ]
    };
    choices[((roll * choices.len() as f32) as usize).min(choices.len() - 1)]
}

pub fn hash2(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x1652_67d1)
        ^ (z as u32).wrapping_mul(0x27d4_eb2d)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h = (h ^ (h >> 13)).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    h as f32 / u32::MAX as f32
}

pub(crate) fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - xi as f32;
    let zf = z - zi as f32;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = zf * zf * (3.0 - 2.0 * zf);
    let a = hash2(xi, zi, seed);
    let b = hash2(xi + 1, zi, seed);
    let c = hash2(xi, zi + 1, seed);
    let d = hash2(xi + 1, zi + 1, seed);
    let ab = a + (b - a) * u;
    let cd = c + (d - c) * u;
    ab + (cd - ab) * v
}

pub(crate) fn fbm(x: f32, z: f32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut norm = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for octave in 0..4 {
        sum += amplitude
            * value_noise(
                x * frequency,
                z * frequency,
                seed.wrapping_add(octave * 131),
            );
        norm += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / norm
}
