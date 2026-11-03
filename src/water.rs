use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    reflection::ReflectionCamera,
    streaming::ChunkRenderer,
    world::{Block, ChunkPos, VoxelWorld},
};

const TICK_SECONDS: f32 = 0.2;
const SCAN_RADIUS: i32 = 8;
const SCAN_HEIGHT: i32 = 6;
const MAX_CHANGES: usize = 48;

#[derive(Resource)]
pub struct WaterFlowTimer(pub Timer);

impl Default for WaterFlowTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(TICK_SECONDS, TimerMode::Repeating))
    }
}

pub fn tick_water(
    time: Res<Time>,
    mut timer: ResMut<WaterFlowTimer>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
    camera: Query<&Transform, (With<Camera3d>, Without<ReflectionCamera>)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let Ok(camera) = camera.single() else {
        return;
    };
    let center_x = camera.translation.x.floor() as i32;
    let center_y = camera.translation.y.floor() as i32;
    let center_z = camera.translation.z.floor() as i32;

    let mut changes: HashMap<(i32, i32, i32), u8> = HashMap::new();
    for x in (center_x - SCAN_RADIUS)..=(center_x + SCAN_RADIUS) {
        for z in (center_z - SCAN_RADIUS)..=(center_z + SCAN_RADIUS) {
            for y in (center_y - SCAN_HEIGHT)..=(center_y + SCAN_HEIGHT) {
                if world.get_block(x, y, z) != Block::Water {
                    continue;
                }
                let level = world.water_level(x, y, z).unwrap_or(0);
                if world.get_block(x, y - 1, z) == Block::Air {
                    offer(&mut changes, x, y - 1, z, 1);
                    continue;
                }
                if level >= 7 {
                    continue;
                }
                let next_level = if level == 0 { 1 } else { level + 1 };
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let neighbor_x = x + dx;
                    let neighbor_z = z + dz;
                    if world.get_block(neighbor_x, y, neighbor_z) == Block::Air {
                        offer(&mut changes, neighbor_x, y, neighbor_z, next_level);
                    }
                }
            }
        }
    }

    let mut applied = 0;
    for ((x, y, z), level) in changes {
        if applied >= MAX_CHANGES {
            break;
        }
        if world.get_block(x, y, z) != Block::Air {
            continue;
        }
        if !world.is_loaded(ChunkPos::from_block(x, z)) {
            continue;
        }
        world.set_water(x, y, z, level);
        renderer.mark_block_dirty(x, y, z);
        applied += 1;
    }
}

fn offer(
    changes: &mut HashMap<(i32, i32, i32), u8>,
    x: i32,
    y: i32,
    z: i32,
    level: u8,
) {
    changes
        .entry((x, y, z))
        .and_modify(|current| *current = (*current).min(level))
        .or_insert(level);
}
