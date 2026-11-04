use bevy::prelude::Vec3;

use crate::pack_atlas::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Block {
    Air = 0,
    Bedrock,
    Deepslate,
    Tuff,
    Calcite,
    Stone,
    Granite,
    Diorite,
    Andesite,
    Gravel,
    Dirt,
    CoarseDirt,
    GrassBlock,
    Podzol,
    Mycelium,
    Mud,
    Clay,
    Sand,
    RedSand,
    Sandstone,
    RedSandstone,
    SnowBlock,
    Ice,
    PackedIce,
    MossBlock,
    Dripstone,
    CoalOre,
    DeepslateCoalOre,
    IronOre,
    DeepslateIronOre,
    CopperOre,
    DeepslateCopperOre,
    GoldOre,
    DeepslateGoldOre,
    RedstoneOre,
    DeepslateRedstoneOre,
    LapisOre,
    DeepslateLapisOre,
    DiamondOre,
    DeepslateDiamondOre,
    EmeraldOre,
    DeepslateEmeraldOre,
    Water,
    Lava,
    Cactus,
    OakLog,
    OakLeaves,
    BirchLog,
    BirchLeaves,
    SpruceLog,
    SpruceLeaves,
    JungleLog,
    JungleLeaves,
    AcaciaLog,
    AcaciaLeaves,
    DarkOakLog,
    DarkOakLeaves,
    CherryLog,
    CherryLeaves,
    BaobabLog,
    BaobabLeaves,
    BlackwoodLog,
    BlackwoodLeaves,
    EucalyptusLog,
    EucalyptusLeaves,
    MagnoliaLog,
    MagnoliaLeaves,
    MapleLog,
    MapleLeaves,
    PineLog,
    PineLeaves,
    RedwoodLog,
    RedwoodLeaves,
    WillowLog,
    WillowLeaves,
    DeadLog,
    ShortGrass,
    Fern,
    DeadBush,
    Dandelion,
    Poppy,
    Cornflower,
    OxeyeDaisy,
    Allium,
    AzureBluet,
    OrangeTulip,
    RedTulip,
    PinkTulip,
    WhiteTulip,
    LilyOfTheValley,
    RedMushroom,
    BrownMushroom,
    SugarCane,
    Chalk,
    ChalkGrass,
    Silt,
    Clover,
    Daisy,
    Cattail,
    PalmLog,
    PalmLeaves,
    JoshuaLog,
    JoshuaLeaves,
    LarchLog,
    LarchLeaves,
    FirLog,
    FirLeaves,
    CypressLog,
    CypressLeaves,
    AspenLog,
    AspenLeaves,
    JacarandaLog,
    JacarandaLeaves,
    MahoganyLog,
    MahoganyLeaves,
    EbonyLog,
    EbonyLeaves,
    WhiteMangroveLog,
    WhiteMangroveLeaves,
    CikaLog,
    CikaLeaves,
    SkyrisLog,
    SkyrisLeaves,
    HollyLog,
    HollyLeaves,
    Lavender,
    Rose,
    PinkDaffodil,
    HighGrass,
    DesertGrass,
    DuneGrass,
    DeadGrass,
    Glowflower,
    Sprout,
    Amaranth,
    CyanAmaranth,
    BlackRose,
    Crocus,
    IncanLily,
    JapaneseOrchid,
    BlueSage,
    PrairieGrass,
    BeachGrass,
    Hydrangea,
    FirecrackerFlower,
    PinkAllium,
    Seagrass,
    Kelp,
    KelpPlant,
}

impl Block {
    pub fn from_id(id: u8) -> Self {
        if id > Block::KelpPlant as u8 {
            return Block::Air;
        }
        unsafe { std::mem::transmute::<u8, Block>(id) }
    }

    pub fn is_air(self) -> bool {
        self == Block::Air
    }

    pub fn is_water(self) -> bool {
        self == Block::Water
    }

    pub fn is_water_plant(self) -> bool {
        matches!(self, Block::Seagrass | Block::Kelp | Block::KelpPlant)
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Lava)
            && !self.is_plant()
            && !self.is_leaves()
    }

    pub fn is_plant(self) -> bool {
        matches!(
            self,
            Block::ShortGrass
                | Block::Fern
                | Block::DeadBush
                | Block::Dandelion
                | Block::Poppy
                | Block::Cornflower
                | Block::OxeyeDaisy
                | Block::Allium
                | Block::AzureBluet
                | Block::OrangeTulip
                | Block::RedTulip
                | Block::PinkTulip
                | Block::WhiteTulip
                | Block::LilyOfTheValley
                | Block::RedMushroom
                | Block::BrownMushroom
                | Block::SugarCane
                | Block::Clover
                | Block::Daisy
                | Block::Cattail
                | Block::Lavender
                | Block::Rose
                | Block::PinkDaffodil
                | Block::HighGrass
                | Block::DesertGrass
                | Block::DuneGrass
                | Block::DeadGrass
                | Block::Glowflower
                | Block::Sprout
                | Block::Amaranth
                | Block::CyanAmaranth
                | Block::BlackRose
                | Block::Crocus
                | Block::IncanLily
                | Block::JapaneseOrchid
                | Block::BlueSage
                | Block::PrairieGrass
                | Block::BeachGrass
                | Block::Hydrangea
                | Block::FirecrackerFlower
                | Block::PinkAllium
                | Block::Seagrass
                | Block::Kelp
                | Block::KelpPlant
        )
    }

    pub fn is_grass_plant(self) -> bool {
        matches!(self, Block::ShortGrass | Block::Fern | Block::Clover | Block::HighGrass | Block::PrairieGrass | Block::BeachGrass | Block::DesertGrass)
    }

    pub fn is_targetable(self) -> bool {
        self.is_solid() || self.is_plant() || self.is_leaves()
    }

    pub fn is_leaves(self) -> bool {
        matches!(
            self,
            Block::OakLeaves
                | Block::BirchLeaves
                | Block::SpruceLeaves
                | Block::JungleLeaves
                | Block::AcaciaLeaves
                | Block::DarkOakLeaves
                | Block::CherryLeaves
                | Block::BaobabLeaves
                | Block::BlackwoodLeaves
                | Block::EucalyptusLeaves
                | Block::MagnoliaLeaves
                | Block::MapleLeaves
                | Block::PineLeaves
                | Block::RedwoodLeaves
                | Block::WillowLeaves
                | Block::PalmLeaves
                | Block::JoshuaLeaves
                | Block::LarchLeaves
                | Block::FirLeaves
                | Block::CypressLeaves
                | Block::AspenLeaves
                | Block::JacarandaLeaves
                | Block::MahoganyLeaves
                | Block::EbonyLeaves
                | Block::WhiteMangroveLeaves
                | Block::CikaLeaves
                | Block::SkyrisLeaves
                | Block::HollyLeaves
        )
    }

    pub fn is_grass_block(self) -> bool {
        matches!(self, Block::GrassBlock)
    }

    pub fn is_soil(self) -> bool {
        matches!(
            self,
            Block::GrassBlock
                | Block::Podzol
                | Block::Mycelium
                | Block::Dirt
                | Block::CoarseDirt
                | Block::ChalkGrass
        )
    }

    pub fn is_stone_like(self) -> bool {
        matches!(
            self,
            Block::Stone
                | Block::Deepslate
                | Block::Tuff
                | Block::Calcite
                | Block::Granite
                | Block::Diorite
                | Block::Andesite
                | Block::Sandstone
                | Block::RedSandstone
                | Block::Chalk
        )
    }

    pub fn is_opaque(self) -> bool {
        self.is_solid() && !self.is_leaves()
    }

    pub fn texture(self, face_y: i32) -> u8 {
        match self {
            Block::Air => TEX_MISSING,
            Block::Bedrock => TEX_BEDROCK,
            Block::Deepslate => {
                if face_y > 0 {
                    TEX_DEEPSLATE_TOP
                } else {
                    TEX_DEEPSLATE
                }
            }
            Block::Tuff => TEX_TUFF,
            Block::Calcite => TEX_CALCITE,
            Block::Stone => TEX_STONE,
            Block::Granite => TEX_GRANITE,
            Block::Diorite => TEX_DIORITE,
            Block::Andesite => TEX_ANDESITE,
            Block::Gravel => TEX_GRAVEL,
            Block::Dirt => TEX_DIRT,
            Block::CoarseDirt => TEX_COARSE_DIRT,
            Block::GrassBlock => match face_y {
                1.. => TEX_GRASS_BLOCK_TOP,
                0 => TEX_GRASS_BLOCK_SIDE,
                _ => TEX_DIRT,
            },
            Block::Podzol => match face_y {
                1.. => TEX_PODZOL_TOP,
                0 => TEX_PODZOL_SIDE,
                _ => TEX_DIRT,
            },
            Block::Mycelium => match face_y {
                1.. => TEX_MYCELIUM_TOP,
                0 => TEX_MYCELIUM_SIDE,
                _ => TEX_DIRT,
            },
            Block::Mud => TEX_MUD,
            Block::Clay => TEX_CLAY,
            Block::Sand => TEX_SAND,
            Block::RedSand => TEX_RED_SAND,
            Block::Sandstone => TEX_SANDSTONE,
            Block::RedSandstone => TEX_RED_SANDSTONE,
            Block::SnowBlock => TEX_SNOW,
            Block::Ice => TEX_ICE,
            Block::PackedIce => TEX_PACKED_ICE,
            Block::MossBlock => TEX_MOSS_BLOCK,
            Block::Dripstone => TEX_DRIPSTONE_BLOCK,
            Block::CoalOre => TEX_COAL_ORE,
            Block::DeepslateCoalOre => TEX_DEEPSLATE_COAL_ORE,
            Block::IronOre => TEX_IRON_ORE,
            Block::DeepslateIronOre => TEX_DEEPSLATE_IRON_ORE,
            Block::CopperOre => TEX_COPPER_ORE,
            Block::DeepslateCopperOre => TEX_DEEPSLATE_COPPER_ORE,
            Block::GoldOre => TEX_GOLD_ORE,
            Block::DeepslateGoldOre => TEX_DEEPSLATE_GOLD_ORE,
            Block::RedstoneOre => TEX_REDSTONE_ORE,
            Block::DeepslateRedstoneOre => TEX_DEEPSLATE_REDSTONE_ORE,
            Block::LapisOre => TEX_LAPIS_ORE,
            Block::DeepslateLapisOre => TEX_DEEPSLATE_LAPIS_ORE,
            Block::DiamondOre => TEX_DIAMOND_ORE,
            Block::DeepslateDiamondOre => TEX_DEEPSLATE_DIAMOND_ORE,
            Block::EmeraldOre => TEX_EMERALD_ORE,
            Block::DeepslateEmeraldOre => TEX_DEEPSLATE_EMERALD_ORE,
            Block::Water => TEX_WATER_STILL,
            Block::Lava => TEX_LAVA_STILL,
            Block::Cactus => TEX_CACTUS_SIDE,
            Block::OakLog => log_tile(face_y, TEX_OAK_LOG, TEX_OAK_LOG_TOP),
            Block::OakLeaves => TEX_OAK_LEAVES,
            Block::BirchLog => log_tile(face_y, TEX_BIRCH_LOG, TEX_BIRCH_LOG_TOP),
            Block::BirchLeaves => TEX_BIRCH_LEAVES,
            Block::SpruceLog => log_tile(face_y, TEX_SPRUCE_LOG, TEX_SPRUCE_LOG_TOP),
            Block::SpruceLeaves => TEX_SPRUCE_LEAVES,
            Block::JungleLog => log_tile(face_y, TEX_JUNGLE_LOG, TEX_JUNGLE_LOG_TOP),
            Block::JungleLeaves => TEX_JUNGLE_LEAVES,
            Block::AcaciaLog => log_tile(face_y, TEX_ACACIA_LOG, TEX_ACACIA_LOG_TOP),
            Block::AcaciaLeaves => TEX_ACACIA_LEAVES,
            Block::DarkOakLog => log_tile(face_y, TEX_DARK_OAK_LOG, TEX_DARK_OAK_LOG_TOP),
            Block::DarkOakLeaves => TEX_DARK_OAK_LEAVES,
            Block::CherryLog => log_tile(face_y, TEX_CHERRY_LOG, TEX_CHERRY_LOG_TOP),
            Block::CherryLeaves => TEX_CHERRY_LEAVES,
            Block::BaobabLog => log_tile(face_y, TEX_BAOBAB_LOG, TEX_BAOBAB_LOG_TOP),
            Block::BaobabLeaves => TEX_BAOBAB_LEAVES,
            Block::BlackwoodLog => log_tile(face_y, TEX_BLACKWOOD_LOG, TEX_BLACKWOOD_LOG_TOP),
            Block::BlackwoodLeaves => TEX_BLACKWOOD_LEAVES,
            Block::EucalyptusLog => log_tile(face_y, TEX_EUCALYPTUS_LOG, TEX_EUCALYPTUS_LOG_TOP),
            Block::EucalyptusLeaves => TEX_EUCALYPTUS_LEAVES,
            Block::MagnoliaLog => log_tile(face_y, TEX_MAGNOLIA_LOG, TEX_MAGNOLIA_LOG_TOP),
            Block::MagnoliaLeaves => TEX_MAGNOLIA_LEAVES,
            Block::MapleLog => log_tile(face_y, TEX_MAPLE_LOG, TEX_MAPLE_LOG_TOP),
            Block::MapleLeaves => TEX_MAPLE_LEAVES,
            Block::PineLog => log_tile(face_y, TEX_PINE_LOG, TEX_PINE_LOG_TOP),
            Block::PineLeaves => TEX_PINE_LEAVES,
            Block::RedwoodLog => log_tile(face_y, TEX_REDWOOD_LOG, TEX_REDWOOD_LOG_TOP),
            Block::RedwoodLeaves => TEX_REDWOOD_LEAVES,
            Block::WillowLog => log_tile(face_y, TEX_WILLOW_LOG, TEX_WILLOW_LOG_TOP),
            Block::WillowLeaves => TEX_WILLOW_LEAVES,
            Block::DeadLog => log_tile(face_y, TEX_DEAD_LOG, TEX_DEAD_LOG_TOP),
            Block::ShortGrass => TEX_GRASS,
            Block::Fern => TEX_FERN,
            Block::DeadBush => TEX_DEAD_BUSH,
            Block::Dandelion => TEX_DANDELION,
            Block::Poppy => TEX_POPPY,
            Block::Cornflower => TEX_CORNFLOWER,
            Block::OxeyeDaisy => TEX_OXEYE_DAISY,
            Block::Allium => TEX_ALLIUM,
            Block::AzureBluet => TEX_AZURE_BLUET,
            Block::OrangeTulip => TEX_ORANGE_TULIP,
            Block::RedTulip => TEX_RED_TULIP,
            Block::PinkTulip => TEX_PINK_TULIP,
            Block::WhiteTulip => TEX_WHITE_TULIP,
            Block::LilyOfTheValley => TEX_LILY_OF_THE_VALLEY,
            Block::RedMushroom => TEX_RED_MUSHROOM,
            Block::BrownMushroom => TEX_BROWN_MUSHROOM,
            Block::SugarCane => TEX_SUGAR_CANE,
            Block::Chalk => TEX_CHALK,
            Block::ChalkGrass => match face_y {
                1.. => TEX_CHALK,
                0 => TEX_CHALK_GRASS_SIDE,
                _ => TEX_CHALK,
            },
            Block::Silt => TEX_SILT_DIRT,
            Block::Clover => TEX_CLOVER,
            Block::Daisy => TEX_DAISY,
            Block::Cattail => TEX_CATTAIL_TOP,
            Block::PalmLog => log_tile(face_y, TEX_PALM_LOG, TEX_PALM_LOG_TOP),
            Block::PalmLeaves => TEX_PALM_LEAVES_SIDE,
            Block::JoshuaLog => log_tile(face_y, TEX_JOSHUA_LOG, TEX_JOSHUA_LOG_TOP),
            Block::JoshuaLeaves => TEX_JOSHUA_LEAVES_TOP,
            Block::LarchLog => log_tile(face_y, TEX_LARCH_LOG, TEX_LARCH_LOG_TOP),
            Block::LarchLeaves => TEX_LARCH_LEAVES,
            Block::FirLog => log_tile(face_y, TEX_FIR_LOG, TEX_FIR_LOG_TOP),
            Block::FirLeaves => TEX_FIR_LEAVES,
            Block::CypressLog => log_tile(face_y, TEX_CYPRESS_LOG, TEX_CYPRESS_LOG_TOP),
            Block::CypressLeaves => TEX_CYPRESS_LEAVES,
            Block::AspenLog => log_tile(face_y, TEX_ASPEN_LOG, TEX_ASPEN_LOG_TOP),
            Block::AspenLeaves => TEX_ASPEN_LEAVES,
            Block::JacarandaLog => log_tile(face_y, TEX_JACARANDA_LOG, TEX_JACARANDA_LOG_TOP),
            Block::JacarandaLeaves => TEX_JACARANDA_LEAVES,
            Block::MahoganyLog => log_tile(face_y, TEX_MAHOGANY_LOG, TEX_MAHOGANY_LOG_TOP),
            Block::MahoganyLeaves => TEX_MAHOGANY_LEAVES,
            Block::EbonyLog => log_tile(face_y, TEX_EBONY_LOG, TEX_EBONY_LOG_TOP),
            Block::EbonyLeaves => TEX_EBONY_LEAVES,
            Block::WhiteMangroveLog => log_tile(face_y, TEX_WHITE_MANGROVE_LOG, TEX_WHITE_MANGROVE_LOG_TOP),
            Block::WhiteMangroveLeaves => TEX_WHITE_MANGROVE_LEAVES,
            Block::CikaLog => log_tile(face_y, TEX_CIKA_LOG, TEX_CIKA_LOG_TOP),
            Block::CikaLeaves => TEX_CIKA_LEAVES,
            Block::SkyrisLog => log_tile(face_y, TEX_SKYRIS_LOG, TEX_SKYRIS_LOG_TOP),
            Block::SkyrisLeaves => TEX_SKYRIS_LEAVES,
            Block::HollyLog => log_tile(face_y, TEX_HOLLY_LOG, TEX_HOLLY_LOG_TOP),
            Block::HollyLeaves => TEX_HOLLY_LEAVES,
            Block::Lavender => TEX_LAVENDER,
            Block::Rose => TEX_ROSE,
            Block::PinkDaffodil => TEX_PINK_DAFFODIL,
            Block::HighGrass => TEX_HIGH_GRASS_PLANT,
            Block::DesertGrass => TEX_DESERT_GRASS,
            Block::DuneGrass => TEX_DUNE_GRASS,
            Block::DeadGrass => TEX_DEAD_GRASS,
            Block::Glowflower => TEX_GLOWFLOWER,
            Block::Sprout => TEX_SPROUT,
            Block::Amaranth => TEX_AMARANTH,
            Block::CyanAmaranth => TEX_CYAN_AMARANTH,
            Block::BlackRose => TEX_BLACK_ROSE,
            Block::Crocus => TEX_CROCUS,
            Block::IncanLily => TEX_INCAN_LILY,
            Block::JapaneseOrchid => TEX_JAPANESE_ORCHID,
            Block::BlueSage => TEX_BLUE_SAGE,
            Block::PrairieGrass => TEX_PRAIRIE_GRASS,
            Block::BeachGrass => TEX_BEACH_GRASS,
            Block::Hydrangea => TEX_HYDRANGEA_BUSH,
            Block::FirecrackerFlower => TEX_FIRECRACKER_FLOWER_BUSH,
            Block::PinkAllium => TEX_PINK_ALLIUM_FLOWER_BUSH,
            Block::Seagrass => TEX_SEAGRASS,
            Block::Kelp => TEX_KELP,
            Block::KelpPlant => TEX_KELP_PLANT,
        }
    }
}

fn log_tile(face_y: i32, side: u8, top: u8) -> u8 {
    if face_y != 0 {
        top
    } else {
        side
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Biome {
    Ocean,
    Beach,
    FrozenTundra,
    FrozenPineTaiga,
    ColdBorealTaiga,
    Spires,
    IcyHeights,
    PumpkinFields,
    AutumnalMapleForest,
    MapleForest,
    BorealTaiga,
    GoldenBorealTaiga,
    PoppyFields,
    SilverBirchForest,
    Orchard,
    Shrubland,
    TemperateGrove,
    PineTaiga,
    BlackwoodTaiga,
    FlowerFields,
    RockyMeadow,
    WillowForest,
    CloverPlains,
    BarleyFields,
    Prairie,
    Grassland,
    Redwoods,
    DeciduousForest,
    MauveHills,
    BambooForest,
    BaobabSavanna,
    SaguaroDesert,
    Outback,
    EucalyptusForest,
    DryBushland,
    JoshuaDesert,
    Rainforest,
}

impl Biome {
    pub fn from_id(id: u8) -> Self {
        match id {
            0 => Biome::Ocean,
            1 => Biome::Beach,
            2 => Biome::FrozenTundra,
            3 => Biome::FrozenPineTaiga,
            4 => Biome::ColdBorealTaiga,
            5 => Biome::Spires,
            6 => Biome::IcyHeights,
            7 => Biome::PumpkinFields,
            8 => Biome::AutumnalMapleForest,
            9 => Biome::MapleForest,
            10 => Biome::BorealTaiga,
            11 => Biome::GoldenBorealTaiga,
            12 => Biome::PoppyFields,
            13 => Biome::SilverBirchForest,
            14 => Biome::Orchard,
            15 => Biome::Shrubland,
            16 => Biome::TemperateGrove,
            17 => Biome::PineTaiga,
            18 => Biome::BlackwoodTaiga,
            19 => Biome::FlowerFields,
            20 => Biome::RockyMeadow,
            21 => Biome::WillowForest,
            22 => Biome::CloverPlains,
            23 => Biome::BarleyFields,
            24 => Biome::Prairie,
            25 => Biome::Grassland,
            26 => Biome::Redwoods,
            27 => Biome::DeciduousForest,
            28 => Biome::MauveHills,
            29 => Biome::BambooForest,
            30 => Biome::BaobabSavanna,
            31 => Biome::SaguaroDesert,
            32 => Biome::Outback,
            33 => Biome::EucalyptusForest,
            34 => Biome::DryBushland,
            35 => Biome::JoshuaDesert,
            _ => Biome::Rainforest,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeKind {
    Oak,
    Birch,
    Spruce,
    Jungle,
    Acacia,
    DarkOak,
    Cherry,
    Baobab,
    Blackwood,
    Eucalyptus,
    Magnolia,
    Maple,
    Pine,
    Redwood,
    Willow,
    Dead,
    Cactus,
    Palm,
    Joshua,
    Larch,
    Fir,
    Cypress,
    Aspen,
    Jacaranda,
    Mahogany,
    Ebony,
    WhiteMangrove,
    Cika,
    Skyris,
    Holly,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlantKind {
    None,
    Grass,
    Fern,
    Flowers,
    FlowerField,
    Prairie,
    DeadBush,
    Mushroom,
}

#[derive(Clone, Copy)]
pub struct BiomeInfo {
    pub name: &'static str,
    pub surface: Block,
    pub filler: Block,
    pub underwater: Block,
    pub grass: [u8; 3],
    pub foliage: [u8; 3],
    pub trees: &'static [(TreeKind, f32)],
    pub plants: PlantKind,
    pub plant_density: f32,
    pub hilliness: f32,
    pub offset: f32,
    pub ridges: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Primary,
    Secondary,
}

const PLAINS_GRASS: [u8; 3] = [140, 178, 90];
const FOREST_GRASS: [u8; 3] = [122, 168, 78];
const COLD_GRASS: [u8; 3] = [112, 148, 112];
const DARK_GRASS: [u8; 3] = [96, 128, 82];
const DRY_GRASS: [u8; 3] = [168, 168, 92];

pub fn srgb(color: [u8; 3]) -> Vec3 {
    let to_linear = |channel: u8| {
        let value = channel as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    Vec3::new(
        to_linear(color[0]),
        to_linear(color[1]),
        to_linear(color[2]),
    )
}

pub fn info(biome: Biome) -> BiomeInfo {
    let (name, surface, filler, underwater, grass, foliage, trees, plants, density, hilliness, offset, ridges): (
        &'static str,
        Block,
        Block,
        Block,
        [u8; 3],
        [u8; 3],
        &'static [(TreeKind, f32)],
        PlantKind,
        f32,
        f32,
        f32,
        f32,
    ) = match biome {
            Biome::Ocean => (
                "Biển", Block::Gravel, Block::Sand, Block::Gravel,
                [110, 142, 96], [110, 142, 96], &[] as &[(TreeKind, f32)], PlantKind::None,
                0.0, 1.0, 0.0, 0.0,
            ),
            Biome::Beach => (
                "Bãi biển", Block::Sand, Block::Sand, Block::Sand,
                PLAINS_GRASS, PLAINS_GRASS,
                &[(TreeKind::Palm, 0.004), (TreeKind::WhiteMangrove, 0.002)], PlantKind::None, 0.0, 0.6, -1.0, 0.0,
            ),
            Biome::FrozenTundra => (
                "Tundra băng", Block::SnowBlock, Block::Dirt, Block::Gravel,
                COLD_GRASS, [150, 176, 170],
                &[(TreeKind::Spruce, 0.006), (TreeKind::Dead, 0.004)],
                PlantKind::None, 0.0, 0.8, 0.0, 0.1,
            ),
            Biome::FrozenPineTaiga => (
                "Taiga thông tuyết", Block::SnowBlock, Block::Dirt, Block::Gravel,
                COLD_GRASS, [120, 150, 130],
                &[(TreeKind::Pine, 0.05), (TreeKind::Spruce, 0.02), (TreeKind::Fir, 0.02)],
                PlantKind::None, 0.0, 1.1, 2.0, 0.3,
            ),
            Biome::ColdBorealTaiga => (
                "Taiga lạnh", Block::Podzol, Block::Dirt, Block::Gravel,
                COLD_GRASS, [104, 138, 104],
                &[(TreeKind::Pine, 0.07), (TreeKind::Spruce, 0.02), (TreeKind::Cypress, 0.01), (TreeKind::Fir, 0.01)],
                PlantKind::Fern, 0.08, 1.2, 2.0, 0.4,
            ),
            Biome::Spires => (
                "Tháp đá", Block::ChalkGrass, Block::Chalk, Block::Gravel,
                COLD_GRASS, [130, 156, 150],
                &[(TreeKind::Pine, 0.008), (TreeKind::Fir, 0.004)],
                PlantKind::None, 0.0, 1.9, 12.0, 1.3,
            ),
            Biome::IcyHeights => (
                "Núi băng", Block::SnowBlock, Block::Chalk, Block::Gravel,
                COLD_GRASS, [140, 166, 160],
                &[(TreeKind::Dead, 0.004)],
                PlantKind::None, 0.0, 2.0, 20.0, 1.5,
            ),
            Biome::PumpkinFields => (
                "Đồng bí ngô", Block::GrassBlock, Block::Dirt, Block::Gravel,
                PLAINS_GRASS, FOREST_GRASS,
                &[(TreeKind::Oak, 0.008)],
                PlantKind::Flowers, 0.10, 0.7, 0.0, 0.0,
            ),
            Biome::AutumnalMapleForest => (
                "Rừng phong thu", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [158, 176, 82], [232, 132, 46],
                &[(TreeKind::Maple, 0.08), (TreeKind::Oak, 0.02), (TreeKind::Aspen, 0.02), (TreeKind::Holly, 0.01)],
                PlantKind::Grass, 0.10, 0.9, 0.0, 0.2,
            ),
            Biome::MapleForest => (
                "Rừng phong", Block::GrassBlock, Block::Dirt, Block::Gravel,
                FOREST_GRASS, [124, 176, 84],
                &[(TreeKind::Maple, 0.07), (TreeKind::Oak, 0.02), (TreeKind::Holly, 0.01), (TreeKind::Aspen, 0.01)],
                PlantKind::Grass, 0.10, 0.9, 0.0, 0.1,
            ),
            Biome::BorealTaiga => (
                "Taiga phương bắc", Block::GrassBlock, Block::Dirt, Block::Gravel,
                DARK_GRASS, [96, 136, 88],
                &[(TreeKind::Pine, 0.07), (TreeKind::Spruce, 0.02)],
                PlantKind::Fern, 0.08, 1.2, 3.0, 0.4,
            ),
            Biome::GoldenBorealTaiga => (
                "Taiga vàng", Block::Podzol, Block::Dirt, Block::Gravel,
                [150, 166, 84], [236, 196, 78],
                &[(TreeKind::Larch, 0.05), (TreeKind::Birch, 0.02)],
                PlantKind::Fern, 0.08, 1.1, 1.0, 0.2,
            ),
            Biome::PoppyFields => (
                "Đồng hoa anh túc", Block::GrassBlock, Block::Dirt, Block::Gravel,
                PLAINS_GRASS, FOREST_GRASS,
                &[(TreeKind::Oak, 0.008)],
                PlantKind::FlowerField, 0.16, 0.7, 0.0, 0.0,
            ),
            Biome::SilverBirchForest => (
                "Rừng bạch dương bạc", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [150, 176, 112], [150, 186, 116],
                &[(TreeKind::Birch, 0.08), (TreeKind::Aspen, 0.03), (TreeKind::Holly, 0.01)],
                PlantKind::Grass, 0.10, 0.8, 0.0, 0.1,
            ),
            Biome::Orchard => (
                "Vườn cây ăn quả", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [128, 178, 96], [122, 180, 100],
                &[(TreeKind::Cherry, 0.04), (TreeKind::Oak, 0.015), (TreeKind::Jacaranda, 0.01), (TreeKind::Cika, 0.01)],
                PlantKind::Flowers, 0.10, 0.8, 0.0, 0.0,
            ),
            Biome::Shrubland => (
                "Bụi rậm", Block::GrassBlock, Block::CoarseDirt, Block::Gravel,
                DRY_GRASS, [150, 162, 96],
                &[(TreeKind::Oak, 0.01)],
                PlantKind::DeadBush, 0.05, 0.8, 0.0, 0.1,
            ),
            Biome::TemperateGrove => (
                "Lùm ôn đới", Block::GrassBlock, Block::Dirt, Block::Gravel,
                PLAINS_GRASS, FOREST_GRASS,
                &[(TreeKind::Oak, 0.05), (TreeKind::Birch, 0.01), (TreeKind::Skyris, 0.01), (TreeKind::Jacaranda, 0.01)],
                PlantKind::Flowers, 0.11, 0.8, 0.0, 0.1,
            ),
            Biome::PineTaiga => (
                "Taiga thông", Block::GrassBlock, Block::Dirt, Block::Gravel,
                COLD_GRASS, [106, 142, 106],
                &[(TreeKind::Pine, 0.08)],
                PlantKind::Fern, 0.08, 1.1, 2.0, 0.3,
            ),
            Biome::BlackwoodTaiga => (
                "Taiga gỗ đen", Block::Podzol, Block::Dirt, Block::Gravel,
                DARK_GRASS, [84, 118, 74],
                &[(TreeKind::Blackwood, 0.07), (TreeKind::Cypress, 0.01)],
                PlantKind::Mushroom, 0.05, 1.1, 2.0, 0.3,
            ),
            Biome::FlowerFields => (
                "Đồng hoa", Block::GrassBlock, Block::Dirt, Block::Gravel,
                PLAINS_GRASS, [140, 180, 92],
                &[(TreeKind::Oak, 0.010), (TreeKind::Jacaranda, 0.004)],
                PlantKind::FlowerField, 0.20, 0.7, 0.0, 0.0,
            ),
            Biome::RockyMeadow => (
                "Đồng cỏ đá", Block::GrassBlock, Block::Stone, Block::Gravel,
                [140, 168, 96], [130, 166, 96],
                &[(TreeKind::Spruce, 0.006)],
                PlantKind::Grass, 0.09, 1.5, 6.0, 0.7,
            ),
            Biome::WillowForest => (
                "Rừng liễu", Block::GrassBlock, Block::Dirt, Block::Silt,
                [150, 176, 112], [150, 180, 120],
                &[(TreeKind::Willow, 0.06), (TreeKind::Oak, 0.01), (TreeKind::WhiteMangrove, 0.02), (TreeKind::Cypress, 0.02)],
                PlantKind::Grass, 0.10, 0.9, 0.0, 0.1,
            ),
            Biome::CloverPlains => (
                "Đồng cỏ ba lá", Block::GrassBlock, Block::Dirt, Block::Gravel,
                PLAINS_GRASS, [146, 186, 96],
                &[(TreeKind::Oak, 0.010)],
                PlantKind::FlowerField, 0.15, 0.7, 0.0, 0.0,
            ),
            Biome::BarleyFields => (
                "Đồng lúa mạch", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [150, 176, 96], [150, 180, 98],
                &[(TreeKind::Oak, 0.009)],
                PlantKind::Prairie, 0.16, 0.7, 0.0, 0.0,
            ),
            Biome::Prairie => (
                "Thảo nguyên", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [156, 176, 94], [152, 176, 96],
                &[(TreeKind::Oak, 0.009)],
                PlantKind::Prairie, 0.13, 0.7, 0.0, 0.0,
            ),
            Biome::Grassland => (
                "Đồng cỏ", Block::GrassBlock, Block::Dirt, Block::Gravel,
                PLAINS_GRASS, [140, 180, 92],
                &[(TreeKind::Oak, 0.008), (TreeKind::Aspen, 0.003)],
                PlantKind::Grass, 0.10, 0.7, 0.0, 0.0,
            ),
            Biome::Redwoods => (
                "Rừng gỗ đỏ", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [120, 148, 88], [110, 146, 82],
                &[(TreeKind::Redwood, 0.02), (TreeKind::Pine, 0.01), (TreeKind::Skyris, 0.01)],
                PlantKind::Fern, 0.10, 1.3, 2.0, 0.4,
            ),
            Biome::DeciduousForest => (
                "Rừng rụng lá", Block::GrassBlock, Block::Dirt, Block::Gravel,
                FOREST_GRASS, [126, 170, 90],
                &[(TreeKind::Oak, 0.05), (TreeKind::Birch, 0.03), (TreeKind::Ebony, 0.02), (TreeKind::Holly, 0.01)],
                PlantKind::Grass, 0.10, 0.9, 0.0, 0.2,
            ),
            Biome::MauveHills => (
                "Đồi tím", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [150, 166, 118], [140, 158, 124],
                &[(TreeKind::Magnolia, 0.04), (TreeKind::Jacaranda, 0.02)],
                PlantKind::Flowers, 0.10, 1.4, 5.0, 0.5,
            ),
            Biome::BambooForest => (
                "Rừng tre", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [124, 176, 84], [122, 178, 82],
                &[(TreeKind::Jungle, 0.04), (TreeKind::Mahogany, 0.01)],
                PlantKind::Prairie, 0.16, 0.9, 0.0, 0.2,
            ),
            Biome::BaobabSavanna => (
                "Xavan bao báp", Block::GrassBlock, Block::CoarseDirt, Block::Gravel,
                DRY_GRASS, [164, 170, 88],
                &[(TreeKind::Baobab, 0.012), (TreeKind::Acacia, 0.010), (TreeKind::Cika, 0.004)],
                PlantKind::Prairie, 0.10, 0.8, 0.0, 0.1,
            ),
            Biome::SaguaroDesert => (
                "Sa mạc xương rồng", Block::Sand, Block::Sandstone, Block::Sand,
                DRY_GRASS, [176, 168, 96],
                &[(TreeKind::Cactus, 0.006), (TreeKind::Joshua, 0.004), (TreeKind::Dead, 0.004)],
                PlantKind::DeadBush, 0.02, 0.9, 0.0, 0.0,
            ),
            Biome::Outback => (
                "Vùng nội địa đỏ", Block::CoarseDirt, Block::RedSand, Block::RedSand,
                [186, 150, 88], [176, 150, 90],
                &[(TreeKind::Eucalyptus, 0.012), (TreeKind::Palm, 0.006), (TreeKind::Acacia, 0.008)],
                PlantKind::DeadBush, 0.04, 1.0, 2.0, 0.2,
            ),
            Biome::EucalyptusForest => (
                "Rừng bạch đàn", Block::GrassBlock, Block::Dirt, Block::Gravel,
                [150, 174, 104], [160, 182, 110],
                &[(TreeKind::Eucalyptus, 0.05), (TreeKind::Ebony, 0.01), (TreeKind::Jacaranda, 0.005)],
                PlantKind::Grass, 0.09, 0.9, 0.0, 0.2,
            ),
            Biome::DryBushland => (
                "Bụi khô", Block::CoarseDirt, Block::Dirt, Block::Gravel,
                [182, 162, 94], [176, 154, 92],
                &[(TreeKind::Dead, 0.02)],
                PlantKind::DeadBush, 0.06, 0.8, 0.0, 0.1,
            ),
            Biome::JoshuaDesert => (
                "Sa mạc Joshua", Block::Sand, Block::RedSandstone, Block::Sand,
                DRY_GRASS, [178, 160, 96],
                &[(TreeKind::Joshua, 0.008), (TreeKind::Dead, 0.006)],
                PlantKind::DeadBush, 0.012, 0.8, 0.0, 0.0,
            ),
            Biome::Rainforest => (
                "Rừng nhiệt đới", Block::GrassBlock, Block::Dirt, Block::Mud,
                [104, 182, 72], [92, 190, 72],
                &[(TreeKind::Jungle, 0.10), (TreeKind::DarkOak, 0.02), (TreeKind::Mahogany, 0.03), (TreeKind::WhiteMangrove, 0.01)],
                PlantKind::Prairie, 0.20, 1.1, 0.0, 0.2,
            ),
        };
    BiomeInfo {
        name,
        surface,
        filler,
        underwater,
        grass,
        foliage,
        trees,
        plants,
        plant_density: density,
        hilliness,
        offset,
        ridges,
    }
}

pub fn surfaces(biome: Biome) -> &'static [(Block, f32)] {
    use Block::*;
    match biome {
        Biome::Ocean => &[
            (Gravel, 0.45),
            (Sand, 0.25),
            (Clay, 0.15),
            (Stone, 0.10),
            (Dirt, 0.05),
        ],
        Biome::Beach => &[(Sand, 0.78), (Gravel, 0.12), (CoarseDirt, 0.10)],
        Biome::FrozenTundra | Biome::FrozenPineTaiga => &[
            (SnowBlock, 0.60),
            (GrassBlock, 0.20),
            (Gravel, 0.12),
            (Podzol, 0.08),
        ],
        Biome::IcyHeights => &[
            (SnowBlock, 0.58),
            (Chalk, 0.20),
            (Stone, 0.17),
            (Ice, 0.05),
        ],
        Biome::Spires => &[
            (ChalkGrass, 0.55),
            (Chalk, 0.30),
            (Stone, 0.10),
            (Gravel, 0.05),
        ],
        Biome::ColdBorealTaiga => &[
            (Podzol, 0.45),
            (GrassBlock, 0.30),
            (CoarseDirt, 0.15),
            (Gravel, 0.10),
        ],
        Biome::BorealTaiga | Biome::PineTaiga | Biome::Redwoods => &[
            (GrassBlock, 0.50),
            (Podzol, 0.30),
            (CoarseDirt, 0.12),
            (Gravel, 0.08),
        ],
        Biome::BlackwoodTaiga => &[
            (GrassBlock, 0.42),
            (Podzol, 0.28),
            (Mycelium, 0.18),
            (CoarseDirt, 0.12),
        ],
        Biome::GoldenBorealTaiga => &[
            (Podzol, 0.50),
            (GrassBlock, 0.25),
            (CoarseDirt, 0.15),
            (Gravel, 0.10),
        ],
        Biome::AutumnalMapleForest
        | Biome::MapleForest
        | Biome::SilverBirchForest
        | Biome::DeciduousForest
        | Biome::EucalyptusForest => &[
            (GrassBlock, 0.58),
            (Podzol, 0.17),
            (CoarseDirt, 0.15),
            (Dirt, 0.10),
        ],
        Biome::Orchard | Biome::TemperateGrove => {
            &[(GrassBlock, 0.70), (Dirt, 0.15), (CoarseDirt, 0.15)]
        }
        Biome::PumpkinFields
        | Biome::PoppyFields
        | Biome::FlowerFields
        | Biome::CloverPlains
        | Biome::BarleyFields
        | Biome::Prairie
        | Biome::Grassland => &[(GrassBlock, 0.72), (CoarseDirt, 0.16), (Dirt, 0.12)],
        Biome::Shrubland => &[
            (GrassBlock, 0.48),
            (CoarseDirt, 0.27),
            (Dirt, 0.15),
            (Gravel, 0.10),
        ],
        Biome::DryBushland => &[
            (CoarseDirt, 0.50),
            (GrassBlock, 0.20),
            (Dirt, 0.15),
            (Gravel, 0.15),
        ],
        Biome::RockyMeadow | Biome::MauveHills => &[
            (GrassBlock, 0.45),
            (Stone, 0.25),
            (Gravel, 0.15),
            (CoarseDirt, 0.15),
        ],
        Biome::WillowForest => &[
            (GrassBlock, 0.50),
            (Silt, 0.25),
            (Mud, 0.15),
            (Dirt, 0.10),
        ],
        Biome::BambooForest | Biome::Rainforest => &[
            (GrassBlock, 0.50),
            (Mud, 0.20),
            (MossBlock, 0.15),
            (Dirt, 0.15),
        ],
        Biome::BaobabSavanna => &[
            (GrassBlock, 0.45),
            (CoarseDirt, 0.35),
            (RedSand, 0.10),
            (Gravel, 0.10),
        ],
        Biome::SaguaroDesert => &[
            (Sand, 0.55),
            (Sandstone, 0.25),
            (RedSand, 0.12),
            (Gravel, 0.08),
        ],
        Biome::Outback => &[
            (CoarseDirt, 0.40),
            (RedSand, 0.35),
            (RedSandstone, 0.15),
            (Gravel, 0.10),
        ],
        Biome::JoshuaDesert => &[
            (Sand, 0.50),
            (RedSand, 0.25),
            (RedSandstone, 0.15),
            (Gravel, 0.10),
        ],
    }
}

static PRIMARY: [[Biome; 5]; 5] = [
    [
        Biome::FrozenTundra,
        Biome::FrozenTundra,
        Biome::FrozenPineTaiga,
        Biome::ColdBorealTaiga,
        Biome::ColdBorealTaiga,
    ],
    [
        Biome::PumpkinFields,
        Biome::AutumnalMapleForest,
        Biome::MapleForest,
        Biome::BorealTaiga,
        Biome::BorealTaiga,
    ],
    [
        Biome::Orchard,
        Biome::Shrubland,
        Biome::TemperateGrove,
        Biome::PineTaiga,
        Biome::BlackwoodTaiga,
    ],
    [
        Biome::BarleyFields,
        Biome::Prairie,
        Biome::Grassland,
        Biome::Redwoods,
        Biome::Redwoods,
    ],
    [
        Biome::BaobabSavanna,
        Biome::BaobabSavanna,
        Biome::SaguaroDesert,
        Biome::Outback,
        Biome::EucalyptusForest,
    ],
];

static SECONDARY: [[Biome; 5]; 5] = [
    [
        Biome::FrozenTundra,
        Biome::FrozenTundra,
        Biome::FrozenPineTaiga,
        Biome::ColdBorealTaiga,
        Biome::Spires,
    ],
    [
        Biome::PoppyFields,
        Biome::SilverBirchForest,
        Biome::MapleForest,
        Biome::BorealTaiga,
        Biome::GoldenBorealTaiga,
    ],
    [
        Biome::FlowerFields,
        Biome::RockyMeadow,
        Biome::WillowForest,
        Biome::CloverPlains,
        Biome::BlackwoodTaiga,
    ],
    [
        Biome::MauveHills,
        Biome::RockyMeadow,
        Biome::BambooForest,
        Biome::Redwoods,
        Biome::Redwoods,
    ],
    [
        Biome::DryBushland,
        Biome::DryBushland,
        Biome::JoshuaDesert,
        Biome::Rainforest,
        Biome::Rainforest,
    ],
];

static ICY: [Biome; 5] = [
    Biome::FrozenTundra,
    Biome::FrozenTundra,
    Biome::FrozenPineTaiga,
    Biome::ColdBorealTaiga,
    Biome::IcyHeights,
];

static VARIANT_PRIMARY: [[Biome; 5]; 5] = [
    [
        Biome::FrozenPineTaiga,
        Biome::FrozenTundra,
        Biome::FrozenTundra,
        Biome::ColdBorealTaiga,
        Biome::Spires,
    ],
    [
        Biome::AutumnalMapleForest,
        Biome::AutumnalMapleForest,
        Biome::MapleForest,
        Biome::BorealTaiga,
        Biome::GoldenBorealTaiga,
    ],
    [
        Biome::Orchard,
        Biome::Shrubland,
        Biome::TemperateGrove,
        Biome::PineTaiga,
        Biome::BlackwoodTaiga,
    ],
    [
        Biome::Prairie,
        Biome::DeciduousForest,
        Biome::Grassland,
        Biome::Redwoods,
        Biome::Redwoods,
    ],
    [
        Biome::BaobabSavanna,
        Biome::BaobabSavanna,
        Biome::SaguaroDesert,
        Biome::Outback,
        Biome::EucalyptusForest,
    ],
];

static VARIANT_SECONDARY: [[Biome; 5]; 5] = [
    [
        Biome::FrozenPineTaiga,
        Biome::FrozenTundra,
        Biome::FrozenTundra,
        Biome::ColdBorealTaiga,
        Biome::Spires,
    ],
    [
        Biome::PoppyFields,
        Biome::SilverBirchForest,
        Biome::MapleForest,
        Biome::BorealTaiga,
        Biome::GoldenBorealTaiga,
    ],
    [
        Biome::FlowerFields,
        Biome::RockyMeadow,
        Biome::MauveHills,
        Biome::CloverPlains,
        Biome::BlackwoodTaiga,
    ],
    [
        Biome::MauveHills,
        Biome::RockyMeadow,
        Biome::BambooForest,
        Biome::Redwoods,
        Biome::Redwoods,
    ],
    [
        Biome::DryBushland,
        Biome::DryBushland,
        Biome::JoshuaDesert,
        Biome::Rainforest,
        Biome::Rainforest,
    ],
];

pub fn select_biome(temperature: f32, humidity: f32, region: Region, variant: bool) -> Biome {
    let t = if temperature < 0.2 {
        0
    } else if temperature < 0.4 {
        1
    } else if temperature < 0.58 {
        2
    } else if temperature < 0.78 {
        3
    } else {
        4
    };
    let h = if humidity < 0.24 {
        0
    } else if humidity < 0.42 {
        1
    } else if humidity < 0.58 {
        2
    } else if humidity < 0.76 {
        3
    } else {
        4
    };
    match (region, variant) {
        (Region::Primary, false) => PRIMARY[t][h],
        (Region::Primary, true) => VARIANT_PRIMARY[t][h],
        (Region::Secondary, false) => {
            if t == 0 && h == 4 {
                ICY[4]
            } else {
                SECONDARY[t][h]
            }
        }
        (Region::Secondary, true) => VARIANT_SECONDARY[t][h],
    }
}

pub const CREATIVE_BLOCKS: &[Block] = &[
    Block::Bedrock,
    Block::Deepslate,
    Block::Tuff,
    Block::Calcite,
    Block::Stone,
    Block::Granite,
    Block::Diorite,
    Block::Andesite,
    Block::Gravel,
    Block::Dirt,
    Block::CoarseDirt,
    Block::GrassBlock,
    Block::Podzol,
    Block::Mycelium,
    Block::Mud,
    Block::Clay,
    Block::Sand,
    Block::RedSand,
    Block::Sandstone,
    Block::RedSandstone,
    Block::SnowBlock,
    Block::Ice,
    Block::PackedIce,
    Block::MossBlock,
    Block::Dripstone,
    Block::CoalOre,
    Block::DeepslateCoalOre,
    Block::IronOre,
    Block::DeepslateIronOre,
    Block::CopperOre,
    Block::DeepslateCopperOre,
    Block::GoldOre,
    Block::DeepslateGoldOre,
    Block::RedstoneOre,
    Block::DeepslateRedstoneOre,
    Block::LapisOre,
    Block::DeepslateLapisOre,
    Block::DiamondOre,
    Block::DeepslateDiamondOre,
    Block::EmeraldOre,
    Block::DeepslateEmeraldOre,
    Block::Water,
    Block::Lava,
    Block::Cactus,
    Block::OakLog,
    Block::OakLeaves,
    Block::BirchLog,
    Block::BirchLeaves,
    Block::SpruceLog,
    Block::SpruceLeaves,
    Block::JungleLog,
    Block::JungleLeaves,
    Block::AcaciaLog,
    Block::AcaciaLeaves,
    Block::DarkOakLog,
    Block::DarkOakLeaves,
    Block::CherryLog,
    Block::CherryLeaves,
    Block::BaobabLog,
    Block::BaobabLeaves,
    Block::BlackwoodLog,
    Block::BlackwoodLeaves,
    Block::EucalyptusLog,
    Block::EucalyptusLeaves,
    Block::MagnoliaLog,
    Block::MagnoliaLeaves,
    Block::MapleLog,
    Block::MapleLeaves,
    Block::PineLog,
    Block::PineLeaves,
    Block::RedwoodLog,
    Block::RedwoodLeaves,
    Block::WillowLog,
    Block::WillowLeaves,
    Block::DeadLog,
    Block::ShortGrass,
    Block::Fern,
    Block::DeadBush,
    Block::Dandelion,
    Block::Poppy,
    Block::Cornflower,
    Block::OxeyeDaisy,
    Block::Allium,
    Block::AzureBluet,
    Block::OrangeTulip,
    Block::RedTulip,
    Block::PinkTulip,
    Block::WhiteTulip,
    Block::LilyOfTheValley,
    Block::RedMushroom,
    Block::BrownMushroom,
    Block::SugarCane,
    Block::Chalk,
    Block::ChalkGrass,
    Block::Silt,
    Block::Clover,
    Block::Daisy,
    Block::Cattail,
    Block::PalmLog,
    Block::PalmLeaves,
    Block::JoshuaLog,
    Block::JoshuaLeaves,
    Block::LarchLog,
    Block::LarchLeaves,
    Block::FirLog,
    Block::FirLeaves,
    Block::CypressLog,
    Block::CypressLeaves,
    Block::AspenLog,
    Block::AspenLeaves,
    Block::JacarandaLog,
    Block::JacarandaLeaves,
    Block::MahoganyLog,
    Block::MahoganyLeaves,
    Block::EbonyLog,
    Block::EbonyLeaves,
    Block::WhiteMangroveLog,
    Block::WhiteMangroveLeaves,
    Block::CikaLog,
    Block::CikaLeaves,
    Block::SkyrisLog,
    Block::SkyrisLeaves,
    Block::HollyLog,
    Block::HollyLeaves,
    Block::Lavender,
    Block::Rose,
    Block::PinkDaffodil,
    Block::HighGrass,
    Block::DesertGrass,
    Block::DuneGrass,
    Block::DeadGrass,
    Block::Glowflower,
    Block::Sprout,
    Block::Amaranth,
    Block::CyanAmaranth,
    Block::BlackRose,
    Block::Crocus,
    Block::IncanLily,
    Block::JapaneseOrchid,
    Block::BlueSage,
    Block::PrairieGrass,
    Block::BeachGrass,
    Block::Hydrangea,
    Block::FirecrackerFlower,
    Block::PinkAllium,
    Block::Seagrass,
    Block::Kelp,
    Block::KelpPlant,
];
