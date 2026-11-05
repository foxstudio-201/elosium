use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    biome_source,
    blocks::{self, Biome, Region, TreeKind},
    mc_worldgen, terrain,
    world::{fbm, hash2, value_noise, Chunk, ChunkPos, Block, CHUNK, MAX_Y, MIN_Y, SEA_LEVEL},
};

pub const DEEPSLATE_TOP: i32 = 0;

static WORLD_TYPE: AtomicU8 = AtomicU8::new(0);

pub fn set_world_type(value: u8) {
    WORLD_TYPE.store(value.min(3), Ordering::Relaxed);
}

pub fn world_type() -> u8 {
    WORLD_TYPE.load(Ordering::Relaxed)
}

pub struct Column {
    pub biome: Biome,
    pub biome_samples: [(Biome, f32); 5],
    pub height: i32,
    pub water_top: i32,
    pub climate: [f32; 2],
    pub stream: f32,
}

struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn unit(&mut self) -> f32 {
        self.next() as f32 / u32::MAX as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit()
    }

    fn int(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next() % (max - min + 1) as u32) as i32
    }
}

fn climate(x: i32, z: i32, seed: u32) -> ([f32; 2], Region, bool) {
    let scale = if world_type() == 2 { 0.25 } else { 1.0 };
    let x = x as f32 * scale;
    let z = z as f32 * scale;
    let temperature =
        ((fbm(x * 0.00042, z * 0.00042, seed ^ 0x2f9a_1c33) - 0.5) * 3.4 + 0.5)
            .clamp(0.0, 1.0);
    let humidity =
        ((fbm(x * 0.00031, z * 0.00031, seed ^ 0x7c41_5e0d) - 0.5) * 3.0 + 0.5)
            .clamp(0.0, 1.0);
    let region = if fbm(x * 0.00035, z * 0.00035, seed ^ 0x51d3_9b71) < 0.55 {
        Region::Primary
    } else {
        Region::Secondary
    };
    let variant = fbm(x * 0.0021, z * 0.0021, seed ^ 0x2c8b_6f13) > 0.56;
    ([temperature, humidity], region, variant)
}

pub fn biome_at(x: i32, z: i32, seed: u32) -> Biome {
    if let Some(sample) = mc_worldgen::climate_sample(x, z, seed, world_type()) {
        return biome_source::select(sample);
    }
    let (climate, region, variant) = climate(x, z, seed);
    blocks::select_biome(climate[0], climate[1], region, variant)
}

pub fn column(x: i32, z: i32, seed: u32) -> Column {
    if world_type() == 1 {
        return Column {
            biome: Biome::Grassland,
            biome_samples: [(Biome::Grassland, 1.0); 5],
            height: SEA_LEVEL,
            water_top: SEA_LEVEL,
            climate: [0.5, 0.5],
            stream: 0.0,
        };
    }
    let (custom_climate, region, variant) = climate(x, z, seed);
    let mc_climate = mc_worldgen::climate_sample(x, z, seed, world_type());
    let center_biome = mc_climate.map_or_else(
        || blocks::select_biome(custom_climate[0], custom_climate[1], region, variant),
        biome_source::select,
    );
    let climate = mc_climate.map_or(custom_climate, |sample| {
        [
            ((sample[0] + 1.0) * 0.5).clamp(0.0, 1.0),
            ((sample[1] + 1.0) * 0.5).clamp(0.0, 1.0),
        ]
    });
    let neighborhood = [(10, 0), (-10, 0), (0, 10), (0, -10)];
    let mut samples = [(Biome::Ocean, 1.0f32); 5];
    samples[0] = (center_biome, 2.0);
    let mut total = 2.0;
    for (index, (dx, dz)) in neighborhood.iter().enumerate() {
        let biome = biome_at(x + dx, z + dz, seed);
        samples[index + 1] = (biome, 1.0);
        total += 1.0;
    }
    let mut hilliness = 0.0;
    let mut offset = 0.0;
    let mut ridges = 0.0;
    for (biome, weight) in samples {
        let info = blocks::info(biome);
        hilliness += info.hilliness * weight / total;
        offset += info.offset * weight / total;
        ridges += info.ridges * weight / total;
    }

    let mut height = match mc_worldgen::surface_height(x, z, seed, world_type()) {
        Some(height) => height as f32,
        None => {
            terrain::surface_height(x, z, seed) as f32
                + offset * 1.5
                + (hilliness - 1.0) * 1.2
                + ridges * 0.8
        }
    };
    height = height.clamp(MIN_Y as f32 + 6.0, MAX_Y as f32 - 24.0);

    let mut biome = samples[0].0;
    if height < (SEA_LEVEL - 7) as f32 {
        biome = Biome::Ocean;
    } else if height < (SEA_LEVEL + 2) as f32 && biome != Biome::Ocean {
        biome = Biome::Beach;
    }

    let stream = stream_strength(x, z, seed);
    let mut water_top = if height < SEA_LEVEL as f32 {
        SEA_LEVEL
    } else {
        height as i32
    };
    let mut stream = stream;
    if stream > 0.0 && height > (SEA_LEVEL + 4) as f32 {
        let surface = height;
        let bed = surface - 2.5 - stream * stream * 3.5;
        height = bed;
        water_top = surface as i32 - 1;
    } else {
        stream = 0.0;
    }

    Column {
        biome,
        biome_samples: samples,
        height: height as i32,
        water_top,
        climate,
        stream,
    }
}

pub fn height_at(x: i32, z: i32, seed: u32) -> i32 {
    column(x, z, seed).height
}

fn stream_strength(x: i32, z: i32, seed: u32) -> f32 {
    let warp_x = (value_noise(x as f32 * 0.004, z as f32 * 0.004, seed ^ 0x3ac1_77d9) - 0.5) * 70.0;
    let warp_z = (value_noise(x as f32 * 0.004, z as f32 * 0.004, seed ^ 0x71b9_2f45) - 0.5) * 70.0;
    let wx = x as f32 + warp_x;
    let wz = z as f32 + warp_z;
    let lines = value_noise(wx * 0.0011, wz * 0.0011, seed ^ 0x2545_f491) - 0.5;
    let width = 0.010;
    let distance = lines.abs();
    if distance >= width {
        return 0.0;
    }
    1.0 - distance / width
}

const ORE_TABLE: [(Block, Block, i32, i32, i32); 9] = [
    (Block::CoalOre, Block::DeepslateCoalOre, -40, 190, 7),
    (Block::CopperOre, Block::DeepslateCopperOre, -40, 110, 6),
    (Block::IronOre, Block::DeepslateIronOre, -120, 130, 5),
    (Block::GoldOre, Block::DeepslateGoldOre, -220, 40, 3),
    (Block::RedstoneOre, Block::DeepslateRedstoneOre, -320, 15, 4),
    (Block::LapisOre, Block::DeepslateLapisOre, -320, 25, 3),
    (Block::DiamondOre, Block::DeepslateDiamondOre, -460, -20, 3),
    (Block::EmeraldOre, Block::DeepslateEmeraldOre, -60, 200, 1),
    (Block::Diorite, Block::Granite, -200, 120, 2),
];

pub fn scatter_ores(chunk: &mut Chunk, pos: ChunkPos, seed: u32) {
    for (index, &(stone_ore, deepslate_ore, min_y, max_y, blobs)) in ORE_TABLE.iter().enumerate() {
        for blob in 0..blobs {
            let mut rng = Rng::new(
                hash2(pos.x * 31 + blob, pos.z * 17 + index as i32, seed ^ 0x46b1_9d27).to_bits(),
            );
            let lx = rng.int(2, CHUNK - 3);
            let lz = rng.int(2, CHUNK - 3);
            let tx = rng.int(0, CHUNK - 1);
            let tz = rng.int(0, CHUNK - 1);
            let surface = chunk.column(tx, tz).height;
            let top = max_y.min(surface - 2);
            if top <= min_y + 2 {
                continue;
            }
            let ly = rng.int(min_y, top).clamp(MIN_Y + 4, MAX_Y - 4);
            let radius = rng.int(1, 2);
            let warm_min = if ly < DEEPSLATE_TOP {
                deepslate_ore
            } else {
                stone_ore
            };
            let cool_min = if ly < DEEPSLATE_TOP {
                Block::Deepslate
            } else {
                Block::Stone
            };
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    for dz in -radius..=radius {
                        if dx * dx + dy * dy + dz * dz > radius * radius + 1 {
                            continue;
                        }
                        let x = lx + dx;
                        let y = ly + dy;
                        let z = lz + dz;
                        if !(0..CHUNK).contains(&x) || !(0..CHUNK).contains(&z) {
                            continue;
                        }
                        if chunk.get_block_local(x, y, z) == cool_min {
                            chunk.set_block_local(x, y, z, warm_min);
                        }
                    }
                }
            }
        }
    }
    for (index, block) in [
        Block::Andesite,
        Block::Tuff,
        Block::Calcite,
        Block::Dripstone,
        Block::Gravel,
        Block::Clay,
    ]
    .into_iter()
    .enumerate()
    {
        for blob in 0..2 {
            let mut rng = Rng::new(
                hash2(pos.x * 13 + blob, pos.z * 29 + index as i32, seed ^ 0x6a09_e667).to_bits(),
            );
            let cx = rng.int(0, CHUNK - 1);
            let cz = rng.int(0, CHUNK - 1);
            let surface = chunk.column(cx, cz).height;
            let cy = rng.int(-380, (surface - 6).max(-370));
            let radius = rng.int(2, 4);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    for dz in -radius..=radius {
                        if dx * dx + dy * dy + dz * dz > radius * radius {
                            continue;
                        }
                        let x = cx + dx;
                        let z = cz + dz;
                        if !(0..CHUNK).contains(&x) || !(0..CHUNK).contains(&z) {
                            continue;
                        }
                        if chunk.get_block_local(x, cy + dy, z).is_stone_like() {
                            chunk.set_block_local(x, cy + dy, z, block);
                        }
                    }
                }
            }
        }
    }
}

pub fn carve_caves(chunk: &mut Chunk, pos: ChunkPos, seed: u32) {
    let worm_count = 2 + (hash2(pos.x, pos.z, seed ^ 0x0bad_5eed) * 2.0) as i32;
    for worm in 0..worm_count {
        let mut rng = Rng::new(hash2(pos.x, pos.z, seed ^ 0x1f3a_7c51).to_bits() ^ worm as u32);
        let mut x = rng.range(1.0, CHUNK as f32 - 2.0);
        let mut z = rng.range(1.0, CHUNK as f32 - 2.0);
        let surface = chunk.column(x as i32, z as i32).height;
        let start_y = rng.range(
            (surface - 90).max(MIN_Y) as f32,
            (surface - 12).max(MIN_Y + 10) as f32,
        );
        let mut y = start_y;
        let mut dx = rng.range(-1.0, 1.0);
        let mut dz = rng.range(-1.0, 1.0);
        let mut dy = rng.range(-0.35, 0.35);
        let length = rng.int(50, 110);
        for _ in 0..length {
            dx = (dx + rng.range(-0.45, 0.45)).clamp(-1.0, 1.0);
            dz = (dz + rng.range(-0.45, 0.45)).clamp(-1.0, 1.0);
            dy = (dy + rng.range(-0.25, 0.25)).clamp(-0.7, 0.7);
            x += dx;
            y += dy;
            z += dz;
            if !(1.0..CHUNK as f32 - 1.0).contains(&x) || !(1.0..CHUNK as f32 - 1.0).contains(&z) {
                break;
            }
            if y <= (MIN_Y + 6) as f32 || y >= (MAX_Y - 8) as f32 {
                break;
            }
            let column = chunk.column(x as i32, z as i32);
            if (y as i32) >= column.height - 1 {
                continue;
            }
            let radius = rng.range(1.1, 2.4);
            carve_sphere(chunk, x, y, z, radius);
        }
    }
}

fn carve_sphere(chunk: &mut Chunk, cx: f32, cy: f32, cz: f32, radius: f32) {
    let min_x = (cx - radius).floor() as i32;
    let max_x = (cx + radius).ceil() as i32;
    let min_z = (cz - radius).floor() as i32;
    let max_z = (cz + radius).ceil() as i32;
    let min_y = (cy - radius).floor() as i32;
    let max_y = (cy + radius).ceil() as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            for z in min_z..=max_z {
                if !(0..CHUNK).contains(&x) || !(0..CHUNK).contains(&z) {
                    continue;
                }
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let dz = z as f32 + 0.5 - cz;
                if dx * dx + dy * dy + dz * dz > radius * radius {
                    continue;
                }
                let block = chunk.get_block_local(x, y, z);
                if block.is_stone_like() || block == Block::Dirt || block == Block::Gravel {
                    if cy < -420.0 {
                        chunk.set_block_local(x, y, z, Block::Lava);
                    } else {
                        chunk.set_block_local(x, y, z, Block::Air);
                    }
                }
            }
        }
    }
}

pub fn plant_tree(chunk: &mut Chunk, lx: i32, lz: i32, surface: i32, kind: TreeKind, seed: u32) {
    let mut rng = Rng::new(hash2(lx * 7 + 3, lz * 11 + 5, seed ^ 0x77aa_19d3).to_bits());
    match kind {
        TreeKind::Oak => {
            let height = rng.int(4, 6);
            round_tree(chunk, lx, lz, surface, height, Block::OakLog, Block::OakLeaves, 2, &mut rng);
        }
        TreeKind::Birch => {
            let height = rng.int(5, 7);
            round_tree(chunk, lx, lz, surface, height, Block::BirchLog, Block::BirchLeaves, 1, &mut rng);
        }
        TreeKind::Spruce => {
            let height = rng.int(6, 9);
            cone_tree(chunk, lx, lz, surface, height, Block::SpruceLog, Block::SpruceLeaves, &mut rng);
        }
        TreeKind::Pine => {
            let height = rng.int(7, 12);
            cone_tree(chunk, lx, lz, surface, height, Block::PineLog, Block::PineLeaves, &mut rng);
        }
        TreeKind::Redwood => {
            let height = rng.int(16, 30);
            cone_tree(chunk, lx, lz, surface, height, Block::RedwoodLog, Block::RedwoodLeaves, &mut rng);
        }
        TreeKind::Jungle => {
            let height = rng.int(6, 10);
            round_tree(chunk, lx, lz, surface, height, Block::JungleLog, Block::JungleLeaves, 3, &mut rng);
        }
        TreeKind::Acacia => {
            let height = rng.int(3, 5);
            let mut top = surface + height;
            trunk(chunk, lx, lz, surface, height, Block::AcaciaLog);
            let lean_x = rng.int(-2, 2);
            let lean_z = rng.int(-2, 2);
            for step in 1..=2 {
                for x in 0..=lean_x.abs() {
                    let sx = lx + lean_x.signum() * x;
                    let sz = lz + lean_z.signum() * ((x * lean_z.abs()) / lean_x.abs().max(1));
                    set_log(chunk, sx, top + step, sz, Block::AcaciaLog);
                }
            }
            top += 2;
            flat_canopy(chunk, lx + lean_x, top, lz + lean_z, 3, Block::AcaciaLeaves);
        }
        TreeKind::DarkOak => {
            let height = rng.int(5, 6);
            for dx in 0..2 {
                for dz in 0..2 {
                    trunk(chunk, lx + dx, lz + dz, surface, height, Block::DarkOakLog);
                }
            }
            top_canopy(chunk, lx, surface + height, lz, 3, Block::DarkOakLeaves);
        }
        TreeKind::Cherry => {
            let height = rng.int(4, 6);
            round_tree(chunk, lx, lz, surface, height, Block::CherryLog, Block::CherryLeaves, 2, &mut rng);
        }
        TreeKind::Baobab => {
            let height = rng.int(3, 4);
            trunk_thick(chunk, lx, lz, surface, height, Block::BaobabLog);
            flat_canopy(chunk, lx, surface + height + 1, lz, 4, Block::BaobabLeaves);
        }
        TreeKind::Blackwood => {
            let height = rng.int(6, 8);
            round_tree(chunk, lx, lz, surface, height, Block::BlackwoodLog, Block::BlackwoodLeaves, 2, &mut rng);
        }
        TreeKind::Eucalyptus => {
            let height = rng.int(9, 14);
            trunk(chunk, lx, lz, surface, height, Block::EucalyptusLog);
            top_canopy(chunk, lx, surface + height, lz, 2, Block::EucalyptusLeaves);
            for branch in 0..3 {
                let angle = branch as f32 * 2.1 + rng.unit();
                let bx = lx + (angle.cos() * 2.2) as i32;
                let bz = lz + (angle.sin() * 2.2) as i32;
                set_log(chunk, bx, surface + height - 1 - branch as i32, bz, Block::EucalyptusLog);
                flat_canopy(chunk, bx, surface + height - branch as i32, bz, 2, Block::EucalyptusLeaves);
            }
        }
        TreeKind::Magnolia => {
            let height = rng.int(5, 7);
            round_tree(chunk, lx, lz, surface, height, Block::MagnoliaLog, Block::MagnoliaLeaves, 3, &mut rng);
        }
        TreeKind::Maple => {
            let height = rng.int(4, 6);
            round_tree(chunk, lx, lz, surface, height, Block::MapleLog, Block::MapleLeaves, 3, &mut rng);
        }
        TreeKind::Willow => {
            let height = rng.int(5, 7);
            trunk(chunk, lx, lz, surface, height, Block::WillowLog);
            top_canopy(chunk, lx, surface + height, lz, 3, Block::WillowLeaves);
            for branch in 0..6 {
                let angle = branch as f32 * 1.05;
                let bx = lx + (angle.cos() * 3.0) as i32;
                let bz = lz + (angle.sin() * 3.0) as i32;
                for drop in 0..3 {
                    set_leaf(chunk, bx, surface + height - 1 - drop, bz, Block::WillowLeaves);
                }
            }
        }
        TreeKind::Dead => {
            let height = rng.int(3, 5);
            trunk(chunk, lx, lz, surface, height, Block::DeadLog);
            for branch in 0..3 {
                let angle = branch as f32 * 2.0 + rng.unit();
                let bx = lx + (angle.cos() * 2.0) as i32;
                let bz = lz + (angle.sin() * 2.0) as i32;
                set_log(chunk, bx, surface + height - 1, bz, Block::DeadLog);
                if rng.unit() < 0.5 {
                    set_log(chunk, bx, surface + height, bz, Block::DeadLog);
                }
            }
        }
        TreeKind::Cactus => {
            let height = rng.int(2, 4);
            trunk(chunk, lx, lz, surface, height, Block::Cactus);
            if rng.unit() < 0.6 {
                let arm_x = if rng.unit() < 0.5 { 1 } else { -1 };
                set_log(chunk, lx + arm_x, surface + 2, lz, Block::Cactus);
                set_log(chunk, lx + arm_x, surface + 3, lz, Block::Cactus);
            }
        }
        TreeKind::Palm => {
            let height = rng.int(6, 9);
            let lean_x = rng.int(-1, 1);
            for step in 0..height {
                let offset = (step as f32 / height as f32 * lean_x as f32) as i32;
                set_log(chunk, lx + offset, surface + 1 + step, lz, Block::PalmLog);
            }
            let top = surface + height + 1;
            let tx = lx + lean_x;
            for dx in -2..=2 {
                for dz in -2..=2 {
                    if dx * dx + dz * dz > 5 {
                        continue;
                    }
                    set_leaf(chunk, tx + dx, top, lz + dz, Block::PalmLeaves);
                }
            }
            set_leaf(chunk, tx, top + 1, lz, Block::PalmLeaves);
        }
        TreeKind::Joshua => {
            let height = rng.int(3, 5);
            trunk(chunk, lx, lz, surface, height, Block::JoshuaLog);
            for arm in 0..2 + (rng.unit() < 0.5) as i32 {
                let angle = arm as f32 * 2.2 + rng.range(-0.6, 0.6);
                let ax = lx + (angle.cos() * 1.6) as i32;
                let az = lz + (angle.sin() * 1.6) as i32;
                let ay = surface + height + arm;
                set_log(chunk, ax, ay, az, Block::JoshuaLog);
                set_log(chunk, ax, ay + 1, az, Block::JoshuaLog);
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        if dx * dx + dz * dz > 2 {
                            continue;
                        }
                        set_leaf(chunk, ax + dx, ay + 2, az + dz, Block::JoshuaLeaves);
                    }
                }
            }
        }
        TreeKind::Larch => {
            let height = rng.int(8, 12);
            cone_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::LarchLog,
                Block::LarchLeaves,
                &mut rng,
            );
        }
        TreeKind::Fir => {
            let height = rng.int(8, 14);
            cone_tree(chunk, lx, lz, surface, height, Block::FirLog, Block::FirLeaves, &mut rng);
        }
        TreeKind::Cypress => {
            let height = rng.int(12, 20);
            cone_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::CypressLog,
                Block::CypressLeaves,
                &mut rng,
            );
        }
        TreeKind::Aspen => {
            let height = rng.int(8, 12);
            round_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::AspenLog,
                Block::AspenLeaves,
                1,
                &mut rng,
            );
        }
        TreeKind::Jacaranda => {
            let height = rng.int(5, 8);
            round_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::JacarandaLog,
                Block::JacarandaLeaves,
                3,
                &mut rng,
            );
        }
        TreeKind::Mahogany => {
            let height = rng.int(8, 13);
            branched_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::MahoganyLog,
                Block::MahoganyLeaves,
                &mut rng,
            );
        }
        TreeKind::Ebony => {
            let height = rng.int(6, 9);
            round_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::EbonyLog,
                Block::EbonyLeaves,
                3,
                &mut rng,
            );
        }
        TreeKind::WhiteMangrove => {
            let height = rng.int(5, 8);
            mangrove_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::WhiteMangroveLog,
                Block::WhiteMangroveLeaves,
                &mut rng,
            );
        }
        TreeKind::Cika => {
            let height = rng.int(7, 11);
            round_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::CikaLog,
                Block::CikaLeaves,
                4,
                &mut rng,
            );
        }
        TreeKind::Skyris => {
            let height = rng.int(10, 16);
            umbrella_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::SkyrisLog,
                Block::SkyrisLeaves,
                &mut rng,
            );
        }
        TreeKind::Holly => {
            let height = rng.int(5, 8);
            round_tree(
                chunk,
                lx,
                lz,
                surface,
                height,
                Block::HollyLog,
                Block::HollyLeaves,
                2,
                &mut rng,
            );
        }
    }
}

fn branched_tree(
    chunk: &mut Chunk,
    lx: i32,
    lz: i32,
    surface: i32,
    height: i32,
    log: Block,
    leaves: Block,
    rng: &mut Rng,
) {
    trunk(chunk, lx, lz, surface, height, log);
    let top = surface + height;
    leaf_blob(chunk, lx, top, lz, 2, 1, leaves, rng);
    let branches = 3 + (rng.unit() * 2.0) as i32;
    for index in 0..branches {
        let angle = index as f32 * std::f32::consts::TAU / branches as f32 + rng.unit() * 0.6;
        let direction_x = angle.cos();
        let direction_z = angle.sin();
        let length = 2 + (rng.unit() * 2.0) as i32;
        let start = top - 1 - (rng.unit() * 3.0) as i32;
        for step in 1..=length {
            let bx = lx + (direction_x * step as f32).round() as i32;
            let bz = lz + (direction_z * step as f32).round() as i32;
            set_log(chunk, bx, start + step, bz, log);
        }
        let end_x = lx + (direction_x * (length + 1) as f32).round() as i32;
        let end_z = lz + (direction_z * (length + 1) as f32).round() as i32;
        leaf_blob(chunk, end_x, start + length + 1, end_z, 2, 1, leaves, rng);
    }
}

fn mangrove_tree(
    chunk: &mut Chunk,
    lx: i32,
    lz: i32,
    surface: i32,
    height: i32,
    log: Block,
    leaves: Block,
    rng: &mut Rng,
) {
    trunk(chunk, lx, lz, surface, height, log);
    for index in 0..4 {
        let angle = index as f32 * std::f32::consts::FRAC_PI_2 + rng.unit() * 0.5;
        let root_x = lx + (angle.cos() * 2.0).round() as i32;
        let root_z = lz + (angle.sin() * 2.0).round() as i32;
        let root_height = 1 + (rng.unit() * 2.0) as i32;
        for y in surface..=surface + root_height {
            set_log(chunk, (lx + root_x) / 2, y, (lz + root_z) / 2, log);
        }
        set_log(chunk, root_x, surface, root_z, log);
        set_log(chunk, root_x, surface + 1, root_z, log);
    }
    flat_canopy(chunk, lx, surface + height, lz, 3, leaves);
}

fn umbrella_tree(
    chunk: &mut Chunk,
    lx: i32,
    lz: i32,
    surface: i32,
    height: i32,
    log: Block,
    leaves: Block,
    rng: &mut Rng,
) {
    trunk(chunk, lx, lz, surface, height, log);
    let top = surface + height;
    flat_canopy(chunk, lx, top, lz, 3 + (rng.unit() * 2.0) as i32, leaves);
    leaf_blob(chunk, lx, top + 1, lz, 3, 1, leaves, rng);
}

fn trunk(chunk: &mut Chunk, lx: i32, lz: i32, surface: i32, height: i32, log: Block) {
    for y in surface + 1..=surface + height {
        set_log(chunk, lx, y, lz, log);
    }
}

fn trunk_thick(chunk: &mut Chunk, lx: i32, lz: i32, surface: i32, height: i32, log: Block) {
    for y in surface + 1..=surface + height {
        for dx in 0..2 {
            for dz in 0..2 {
                set_log(chunk, lx + dx, y, lz + dz, log);
            }
        }
    }
}

fn round_tree(
    chunk: &mut Chunk,
    lx: i32,
    lz: i32,
    surface: i32,
    height: i32,
    log: Block,
    leaves: Block,
    radius: i32,
    rng: &mut Rng,
) {
    if radius >= 3 && height >= 7 {
        trunk_thick(chunk, lx, lz, surface, height, log);
    } else {
        trunk(chunk, lx, lz, surface, height, log);
    }
    let top = surface + height;
    let rings = (height / 3).clamp(1, 3);
    for ring in 0..rings {
        let y = top - 1 - ring * 2;
        let branches = 2 + (rng.unit() * 2.0) as i32;
        for index in 0..branches {
            let angle = index as f32 * std::f32::consts::TAU / branches as f32 + rng.unit() * 0.7;
            let direction_x = angle.cos();
            let direction_z = angle.sin();
            let length = 2 + (rng.unit() * radius.max(1) as f32) as i32;
            for step in 1..=length {
                let branch_x = lx + (direction_x * step as f32).round() as i32;
                let branch_z = lz + (direction_z * step as f32).round() as i32;
                set_log(chunk, branch_x, y + step / 2, branch_z, log);
            }
            let end_x = lx + (direction_x * (length + 1) as f32).round() as i32;
            let end_z = lz + (direction_z * (length + 1) as f32).round() as i32;
            leaf_blob(chunk, end_x, y + length / 2 + 1, end_z, 2, 1, leaves, rng);
        }
    }
    leaf_blob(chunk, lx, top, lz, (radius - 1).max(1), 1, leaves, rng);
}

fn cone_tree(
    chunk: &mut Chunk,
    lx: i32,
    lz: i32,
    surface: i32,
    height: i32,
    log: Block,
    leaves: Block,
    rng: &mut Rng,
) {
    if height >= 14 {
        trunk_thick(chunk, lx, lz, surface, height, log);
    } else {
        trunk(chunk, lx, lz, surface, height, log);
    }
    let top = surface + height;
    let layers = (height / 2).clamp(3, 12);
    for layer in 0..layers {
        let y = top - layer;
        let radius = ((layer as f32 * 0.45) as i32).min(3);
        if radius >= 1 {
            for direction in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
            ] {
                if rng.unit() < 0.35 {
                    continue;
                }
                let reach = radius.min(2);
                for step in 1..=reach {
                    set_log(
                        chunk,
                        lx + direction.0 * step,
                        y,
                        lz + direction.1 * step,
                        log,
                    );
                }
            }
        }
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz > radius * radius + 1 {
                    continue;
                }
                if rng.unit() < 0.2 && radius > 0 {
                    continue;
                }
                set_leaf(chunk, lx + dx, y, lz + dz, leaves);
            }
        }
    }
}

fn top_canopy(chunk: &mut Chunk, lx: i32, top: i32, lz: i32, radius: i32, leaves: Block) {
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            set_leaf(chunk, lx + dx, top, lz + dz, leaves);
            set_leaf(chunk, lx + dx, top + 1, lz + dz, leaves);
        }
    }
}

fn flat_canopy(chunk: &mut Chunk, lx: i32, top: i32, lz: i32, radius: i32, leaves: Block) {
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if dx * dx + dz * dz > radius * radius + 1 {
                continue;
            }
            set_leaf(chunk, lx + dx, top, lz + dz, leaves);
            if dx.abs() <= radius - 1 && dz.abs() <= radius - 1 {
                set_leaf(chunk, lx + dx, top + 1, lz + dz, leaves);
            }
        }
    }
}

fn leaf_blob(
    chunk: &mut Chunk,
    cx: i32,
    cy: i32,
    cz: i32,
    radius: i32,
    height: i32,
    leaves: Block,
    rng: &mut Rng,
) {
    for dy in -height..=height {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let distance = dx * dx + dz * dz + dy * dy * 2;
                if distance > radius * radius + 1 {
                    continue;
                }
                if rng.unit() < 0.08 {
                    continue;
                }
                set_leaf(chunk, cx + dx, cy + dy, cz + dz, leaves);
            }
        }
    }
}

fn set_log(chunk: &mut Chunk, x: i32, y: i32, z: i32, log: Block) {
    if !(0..CHUNK).contains(&x) || !(0..CHUNK).contains(&z) || !(MIN_Y..MAX_Y).contains(&y) {
        return;
    }
    let current = chunk.get_block_local(x, y, z);
    if current == Block::Air || current.is_leaves() {
        chunk.set_block_local(x, y, z, log);
    }
}

fn set_leaf(chunk: &mut Chunk, x: i32, y: i32, z: i32, leaves: Block) {
    if !(0..CHUNK).contains(&x) || !(0..CHUNK).contains(&z) || !(MIN_Y..MAX_Y).contains(&y) {
        return;
    }
    if chunk.get_block_local(x, y, z) == Block::Air {
        chunk.set_block_local(x, y, z, leaves);
    }
}
