use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::Vec3;

use crate::{
    blocks::{Biome, Block},
    world::{ChunkPos, ColumnInfo, SECTION_VOLUME},
};

const MAGIC: &[u8; 4] = b"ELS4";
const VERSION: u8 = 1;
const SECTOR: u64 = 4096;
const HEADER_BYTES: u64 = 8192;
const REGION_SIZE: i32 = 32;
const MAX_SECTORS: u64 = 255;

pub struct ChunkData {
    pub columns: Vec<ColumnInfo>,
    pub sections: Vec<(i32, Vec<Block>)>,
    pub min_section: i32,
    pub max_section: i32,
}

pub struct ChunkView<'a> {
    pub columns: &'a [ColumnInfo],
    pub sections: &'a [(i32, &'a [Block])],
    pub min_section: i32,
    pub max_section: i32,
}

pub struct WorldStorage {
    root: PathBuf,
}

impl WorldStorage {
    pub fn new(world_dir: &Path) -> Self {
        let root = world_dir.join("region");
        fs::create_dir_all(&root).ok();
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) == Some("els") {
                    let target = path.with_extension("mca");
                    if !target.exists() {
                        fs::rename(&path, &target).ok();
                    }
                }
            }
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);
        fs::write(
            world_dir.join("session.lock"),
            format!("ELOSIUM {stamp}\n"),
        )
        .ok();
        Self { root }
    }

    pub fn load_chunk(&self, pos: ChunkPos) -> Option<ChunkData> {
        let mut file = File::open(self.region_path(pos))
            .or_else(|_| File::open(self.region_path_ext(pos, "els")))
            .ok()?;
        let slot = region_slot(pos);
        let entry = read_entry(&mut file, slot).ok()?;
        if entry == 0 {
            return None;
        }
        let sectors = (entry & 0xFF) as u64;
        if sectors == 0 {
            return None;
        }
        let offset = ((entry >> 8) as u64) * SECTOR;
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut size = [0u8; 4];
        file.read_exact(&mut size).ok()?;
        let size = u32::from_le_bytes(size) as u64;
        if size == 0 || size + 4 > sectors * SECTOR {
            return None;
        }
        let mut payload = vec![0u8; size as usize];
        file.read_exact(&mut payload).ok()?;
        decode_chunk(&payload)
    }

    pub fn save_chunk(&self, pos: ChunkPos, data: &ChunkView<'_>) {
        let payload = encode_chunk(data);
        let slot = region_slot(pos);
        let Ok(mut file) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.region_path(pos))
        else {
            return;
        };
        let mut length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        if length < HEADER_BYTES {
            if file.set_len(HEADER_BYTES).is_err() {
                return;
            }
            length = HEADER_BYTES;
        }
        let mut record = Vec::with_capacity(payload.len() + 4);
        record.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        record.extend_from_slice(&payload);
        let sectors = (record.len() as u64).div_ceil(SECTOR);
        if sectors == 0 || sectors > MAX_SECTORS {
            return;
        }
        let offset = length.div_ceil(SECTOR) * SECTOR;
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return;
        }
        if file.write_all(&record).is_err() {
            return;
        }
        let pad = (sectors * SECTOR) as usize - record.len();
        if pad > 0 && file.write_all(&vec![0u8; pad]).is_err() {
            return;
        }
        let entry = ((offset / SECTOR) as u32) << 8 | sectors as u32;
        if file.seek(SeekFrom::Start(slot * 4)).is_ok() {
            file.write_all(&entry.to_le_bytes()).ok();
        }
    }

    fn region_path(&self, pos: ChunkPos) -> PathBuf {
        self.region_path_ext(pos, "mca")
    }

    fn region_path_ext(&self, pos: ChunkPos, extension: &str) -> PathBuf {
        self.root.join(format!(
            "r.{}.{}.{}",
            pos.x.div_euclid(REGION_SIZE),
            pos.z.div_euclid(REGION_SIZE),
            extension
        ))
    }
}

fn region_slot(pos: ChunkPos) -> u64 {
    let x = pos.x.rem_euclid(REGION_SIZE) as u64;
    let z = pos.z.rem_euclid(REGION_SIZE) as u64;
    x + z * REGION_SIZE as u64
}

fn read_entry(file: &mut File, slot: u64) -> std::io::Result<u32> {
    file.seek(SeekFrom::Start(slot * 4))?;
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn encode_chunk(data: &ChunkView<'_>) -> Vec<u8> {
    let mut out = Vec::with_capacity(24 * 1024);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&data.min_section.to_le_bytes());
    out.extend_from_slice(&data.max_section.to_le_bytes());
    for column in data.columns {
        out.extend_from_slice(&(column.height as i16).to_le_bytes());
        out.extend_from_slice(&(column.water_top as i16).to_le_bytes());
        out.push(column.biome as u8);
        for value in [
            column.grass.x,
            column.grass.y,
            column.grass.z,
            column.foliage.x,
            column.foliage.y,
            column.foliage.z,
        ] {
            out.push((value.clamp(0.0, 1.0) * 255.0) as u8);
        }
        out.push((column.stream.clamp(0.0, 1.0) * 255.0) as u8);
    }
    out.extend_from_slice(&(data.sections.len() as u16).to_le_bytes());
    for (index, blocks) in data.sections {
        out.extend_from_slice(&index.to_le_bytes());
        let mut runs: Vec<(u16, u8)> = Vec::with_capacity(64);
        for &block in blocks.iter() {
            let id = block as u8;
            if let Some(last) = runs.last_mut()
                && last.1 == id
                && last.0 < u16::MAX
            {
                last.0 += 1;
                continue;
            }
            runs.push((1, id));
        }
        out.extend_from_slice(&(runs.len() as u16).to_le_bytes());
        for (count, id) in runs {
            out.extend_from_slice(&count.to_le_bytes());
            out.push(id);
        }
    }
    out
}

fn decode_chunk(payload: &[u8]) -> Option<ChunkData> {
    let mut cursor = 0usize;
    if payload.len() < 13 || &payload[..4] != MAGIC || payload[4] != VERSION {
        return None;
    }
    cursor += 5;
    let min_section = read_i32(payload, &mut cursor)?;
    let max_section = read_i32(payload, &mut cursor)?;
    let mut columns = Vec::with_capacity(256);
    for _ in 0..256 {
        let height = read_i16(payload, &mut cursor)? as i32;
        let water_top = read_i16(payload, &mut cursor)? as i32;
        let biome = Biome::from_id(read_u8(payload, &mut cursor)?);
        let mut colors = [0f32; 6];
        for value in &mut colors {
            *value = read_u8(payload, &mut cursor)? as f32 / 255.0;
        }
        let stream = read_u8(payload, &mut cursor)? as f32 / 255.0;
        columns.push(ColumnInfo {
            height,
            water_top,
            biome,
            climate: [0.5, 0.5],
            grass: Vec3::new(colors[0], colors[1], colors[2]),
            foliage: Vec3::new(colors[3], colors[4], colors[5]),
            stream,
        });
    }
    let section_count = read_u16(payload, &mut cursor)? as usize;
    let mut sections = Vec::with_capacity(section_count);
    for _ in 0..section_count {
        let index = read_i32(payload, &mut cursor)?;
        let run_count = read_u16(payload, &mut cursor)? as usize;
        let mut blocks = Vec::with_capacity(SECTION_VOLUME);
        for _ in 0..run_count {
            let count = read_u16(payload, &mut cursor)? as usize;
            let block = Block::from_id(read_u8(payload, &mut cursor)?);
            let remaining = SECTION_VOLUME.saturating_sub(blocks.len());
            for _ in 0..count.min(remaining) {
                blocks.push(block);
            }
        }
        if blocks.len() != SECTION_VOLUME {
            return None;
        }
        sections.push((index, blocks));
    }
    Some(ChunkData {
        columns,
        sections,
        min_section,
        max_section,
    })
}

fn read_u8(payload: &[u8], cursor: &mut usize) -> Option<u8> {
    let value = *payload.get(*cursor)?;
    *cursor += 1;
    Some(value)
}

fn read_u16(payload: &[u8], cursor: &mut usize) -> Option<u16> {
    let bytes: [u8; 2] = payload.get(*cursor..*cursor + 2)?.try_into().ok()?;
    *cursor += 2;
    Some(u16::from_le_bytes(bytes))
}

fn read_i16(payload: &[u8], cursor: &mut usize) -> Option<i16> {
    let bytes: [u8; 2] = payload.get(*cursor..*cursor + 2)?.try_into().ok()?;
    *cursor += 2;
    Some(i16::from_le_bytes(bytes))
}

fn read_i32(payload: &[u8], cursor: &mut usize) -> Option<i32> {
    let bytes: [u8; 4] = payload.get(*cursor..*cursor + 4)?.try_into().ok()?;
    *cursor += 4;
    Some(i32::from_le_bytes(bytes))
}
