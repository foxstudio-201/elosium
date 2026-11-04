use bevy::prelude::*;

use crate::{
    blocks::Block,
    daynight::DayCycle,
    frontend::GameRules,
    hud::{Inventory, ItemStack},
    player::Player,
    streaming::ChunkRenderer,
    world::VoxelWorld,
};

pub struct CommandContext<'a> {
    pub day: &'a mut DayCycle,
    pub player: &'a mut Player,
    pub rules: &'a mut GameRules,
    pub inventory: &'a mut Inventory,
    pub world: &'a mut VoxelWorld,
    pub renderer: &'a mut ChunkRenderer,
    pub seed: u32,
}

const GAMEMODES: [&str; 4] = ["survival", "creative", "adventure", "spectator"];
const TIMES: [&str; 4] = ["day", "noon", "night", "midnight"];

const BLOCK_NAMES: &[(&str, Block)] = &[
    ("stone", Block::Stone),
    ("granite", Block::Granite),
    ("diorite", Block::Diorite),
    ("andesite", Block::Andesite),
    ("deepslate", Block::Deepslate),
    ("tuff", Block::Tuff),
    ("dirt", Block::Dirt),
    ("coarse_dirt", Block::CoarseDirt),
    ("grass_block", Block::GrassBlock),
    ("podzol", Block::Podzol),
    ("mud", Block::Mud),
    ("clay", Block::Clay),
    ("sand", Block::Sand),
    ("red_sand", Block::RedSand),
    ("sandstone", Block::Sandstone),
    ("red_sandstone", Block::RedSandstone),
    ("gravel", Block::Gravel),
    ("snow_block", Block::SnowBlock),
    ("ice", Block::Ice),
    ("packed_ice", Block::PackedIce),
    ("moss_block", Block::MossBlock),
    ("dripstone", Block::Dripstone),
    ("coal_ore", Block::CoalOre),
    ("iron_ore", Block::IronOre),
    ("copper_ore", Block::CopperOre),
    ("gold_ore", Block::GoldOre),
    ("redstone_ore", Block::RedstoneOre),
    ("lapis_ore", Block::LapisOre),
    ("diamond_ore", Block::DiamondOre),
    ("emerald_ore", Block::EmeraldOre),
    ("oak_log", Block::OakLog),
    ("oak_leaves", Block::OakLeaves),
    ("birch_log", Block::BirchLog),
    ("birch_leaves", Block::BirchLeaves),
    ("spruce_log", Block::SpruceLog),
    ("spruce_leaves", Block::SpruceLeaves),
    ("jungle_log", Block::JungleLog),
    ("jungle_leaves", Block::JungleLeaves),
    ("acacia_log", Block::AcaciaLog),
    ("acacia_leaves", Block::AcaciaLeaves),
    ("dark_oak_log", Block::DarkOakLog),
    ("dark_oak_leaves", Block::DarkOakLeaves),
    ("cherry_log", Block::CherryLog),
    ("cherry_leaves", Block::CherryLeaves),
    ("cactus", Block::Cactus),
    ("short_grass", Block::ShortGrass),
    ("dandelion", Block::Dandelion),
    ("water", Block::Water),
    ("lava", Block::Lava),
];

fn block_from_name(name: &str) -> Option<Block> {
    BLOCK_NAMES
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, block)| *block)
}

pub fn execute(message: &str, ctx: &mut CommandContext) -> String {
    let mut parts = message.split_whitespace();
    let Some(command) = parts.next() else {
        return String::new();
    };
    match command {
        "/help" => "Lệnh: /gamemode /time /tp /give /setblock /fill /clear /seed /clone /whoami".to_string(),
        "/seed" => format!("Seed: {}", ctx.seed),
        "/gamemode" => {
            let Some(value) = parts.next() else {
                return "Dùng: /gamemode survival|creative|adventure|spectator".to_string();
            };
            let Some(index) = GAMEMODES.iter().position(|mode| *mode == value) else {
                return format!("Chế độ không hợp lệ: {value}");
            };
            ctx.rules.gamemode = index as u8;
            format!("Đã đặt chế độ chơi: {}", GAMEMODES[index])
        }
        "/time" => match (parts.next(), parts.next()) {
            (Some("set"), Some(value)) => {
                let time = match value {
                    "day" => Some(0.18),
                    "noon" => Some(0.25),
                    "night" => Some(0.72),
                    "midnight" => Some(0.92),
                    _ => value.parse::<f32>().ok(),
                };
                match time {
                    Some(time) => {
                        ctx.day.time = time.rem_euclid(1.0);
                        format!("Đã đặt thời gian: {value}")
                    }
                    None => "Dùng: /time set day|noon|night|midnight|<0..1>".to_string(),
                }
            }
            (Some("add"), Some(value)) => match value.parse::<f32>() {
                Ok(delta) => {
                    ctx.day.time = (ctx.day.time + delta).rem_euclid(1.0);
                    format!("Đã thêm thời gian: {value}")
                }
                Err(_) => "Dùng: /time add <0..1>".to_string(),
            },
            _ => "Dùng: /time set <giá trị> | /time add <giá trị>".to_string(),
        },
        "/tp" => {
            let values: Vec<f32> = parts.filter_map(|part| part.parse().ok()).collect();
            if values.len() != 3 {
                return "Dùng: /tp <x> <y> <z>".to_string();
            }
            ctx.player.feet = Vec3::new(values[0], values[1], values[2]);
            ctx.player.prev_feet = ctx.player.feet;
            ctx.player.velocity = Vec3::ZERO;
            format!(
                "Đã dịch chuyển tới {:.1} {:.1} {:.1}",
                values[0], values[1], values[2]
            )
        }
        "/give" => {
            let Some(name) = parts.next() else {
                return "Dùng: /give <block> [số lượng]".to_string();
            };
            let Some(block) = block_from_name(name) else {
                return format!("Không có block: {name}");
            };
            let count = parts.next().and_then(|value| value.parse().ok()).unwrap_or(64);
            ctx.inventory.slots[ctx.inventory.selected] = Some(ItemStack {
                block,
                count: count.clamp(1, 64),
            });
            format!("Đã nhận {} x{}", name, count.clamp(1, 64))
        }
        "/setblock" => {
            let coords: Vec<i32> = parts
                .by_ref()
                .take(3)
                .filter_map(|part| part.parse().ok())
                .collect();
            let Some(name) = parts.next() else {
                return "Dùng: /setblock <x> <y> <z> <block>".to_string();
            };
            if coords.len() != 3 {
                return "Dùng: /setblock <x> <y> <z> <block>".to_string();
            }
            let Some(block) = block_from_name(name) else {
                return format!("Không có block: {name}");
            };
            let (x, y, z) = (coords[0], coords[1], coords[2]);
            ctx.world.set_block_recorded(x, y, z, block);
            ctx.renderer.mark_block_dirty(x, y, z);
            format!("Đã đặt {name} tại {x} {y} {z}")
        }
        "/fill" => {
            let mut values = [0i32; 6];
            let mut count = 0;
            while count < 6 {
                let Some(part) = parts.next() else {
                    return "Dùng: /fill <x1> <y1> <z1> <x2> <y2> <z2> <block>".to_string();
                };
                let Ok(value) = part.parse::<i32>() else {
                    return "Dùng: /fill <x1> <y1> <z1> <x2> <y2> <z2> <block>".to_string();
                };
                values[count] = value;
                count += 1;
            }
            let Some(name) = parts.next() else {
                return "Dùng: /fill <x1> <y1> <z1> <x2> <y2> <z2> <block>".to_string();
            };
            let Some(block) = block_from_name(name) else {
                return format!("Không có block: {name}");
            };
            let x0 = values[0].min(values[3]);
            let x1 = values[0].max(values[3]);
            let y0 = values[1].min(values[4]);
            let y1 = values[1].max(values[4]);
            let z0 = values[2].min(values[5]);
            let z1 = values[2].max(values[5]);
            let volume = (x1 - x0 + 1) as i64 * (y1 - y0 + 1) as i64 * (z1 - z0 + 1) as i64;
            if volume > 8192 {
                return "Khối quá lớn (tối đa 8192 block)".to_string();
            }
            for x in x0..=x1 {
                for y in y0..=y1 {
                    for z in z0..=z1 {
                        ctx.world.set_block_recorded(x, y, z, block);
                        ctx.renderer.mark_block_dirty(x, y, z);
                    }
                }
            }
            format!("Đã đặt {name} cho {volume} block")
        }
        "/clear" => {
            for slot in &mut ctx.inventory.slots {
                *slot = None;
            }
            "Đã xoá túi đồ".to_string()
        }
        _ => format!("Không rõ lệnh: {message} (thử /help)"),
    }
}

pub fn suggestions(input: &str) -> Vec<String> {
    let trailing_space = input.ends_with(' ');
    let trimmed = input.trim_end();
    if !trimmed.starts_with('/') {
        return Vec::new();
    }
    let mut words = trimmed.split_whitespace();
    let head = words.next().unwrap_or("");
    if trailing_space || words.next().is_some() {
        let args: Vec<&str> = trimmed.split_whitespace().skip(1).collect();
        let partial = if trailing_space {
            ""
        } else {
            args.last().copied().unwrap_or("")
        };
        let prefix = format!("{head} ");
        let mut out = Vec::new();
        let mut push = |value: &str, suffix: &str| {
            let candidate = format!("{prefix}{value}{suffix}");
            if candidate.starts_with(trimmed) {
                out.push(candidate);
            }
        };
        match head {
            "/gamemode" => {
                for mode in GAMEMODES {
                    if mode.starts_with(partial) {
                        push(mode, "");
                    }
                }
            }
            "/time" => {
                if args.len() <= 1 {
                    for word in ["set", "add"] {
                        if word.starts_with(partial) {
                            push(word, " ");
                        }
                    }
                } else if args.first() == Some(&"set") {
                    for word in TIMES {
                        if word.starts_with(partial) {
                            push(&format!("set {word}"), "");
                        }
                    }
                } else if args.first() == Some(&"add") {
                    out.push(format!("{prefix}add <0..1>"));
                }
                out.sort();
                out.dedup();
            }
            "/setblock" => {
                if args.len() == 4 {
                    for (name, _) in BLOCK_NAMES {
                        if name.starts_with(partial) {
                            push(name, "");
                        }
                    }
                }
            }
            "/give" => {
                if args.len() <= 1 {
                    for (name, _) in BLOCK_NAMES {
                        if name.starts_with(partial) {
                            push(name, "");
                        }
                    }
                }
            }
            "/tp" => {
                if args.is_empty() || trimmed.ends_with(' ') {
                    out.push(format!("{prefix}~ ~ ~"));
                }
            }
            _ => {}
        }
        return out.into_iter().take(8).collect();
    }
    let mut out: Vec<String> = ["/help", "/gamemode", "/time", "/tp", "/give", "/setblock", "/fill", "/clear", "/seed"]
        .iter()
        .filter(|command| command.starts_with(head))
        .map(|command| (*command).to_string())
        .collect();
    out.sort();
    out
}
