use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
};

use crate::{
    blocks::Block,
    pack_atlas::{
        LeafQuad, BLOCK_ATLAS_SIZE, BLOCK_UV_RECTS, FLORA_ATLAS_SIZE, FLORA_UV_RECTS,
        LEAF_ATLAS_SIZE, LEAF_UV_RECTS, LEAF_VARIANTS, TEX_GRASS_BLOCK_SIDE_OVERLAY,
        TEX_WATER_STILL,
    },
    world::{hash2, section_origin_y, ChunkPos, SECTION_COUNT, SECTION_VOLUME, VoxelWorld, CHUNK, SECTION},
};

const WATER_SURFACE: f32 = crate::world::WATER_SURFACE_HEIGHT - crate::world::SEA_LEVEL as f32;

struct FaceDef {
    normal: [i32; 3],
    corners: [[i32; 3]; 4],
    uvs: [[f32; 2]; 4],
    axis_u: [i32; 3],
    axis_v: [i32; 3],
    corner_uv: [[i32; 2]; 4],
}

const FACES: [FaceDef; 6] = [
    FaceDef {
        normal: [0, 1, 0],
        corners: [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]],
        uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        axis_u: [1, 0, 0],
        axis_v: [0, 0, 1],
        corner_uv: [[0, 0], [0, 1], [1, 1], [1, 0]],
    },
    FaceDef {
        normal: [0, -1, 0],
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        axis_u: [1, 0, 0],
        axis_v: [0, 0, 1],
        corner_uv: [[0, 0], [1, 0], [1, 1], [0, 1]],
    },
    FaceDef {
        normal: [1, 0, 0],
        corners: [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]],
        uvs: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        axis_u: [0, 1, 0],
        axis_v: [0, 0, 1],
        corner_uv: [[0, 0], [1, 0], [1, 1], [0, 1]],
    },
    FaceDef {
        normal: [-1, 0, 0],
        corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        axis_u: [0, 1, 0],
        axis_v: [0, 0, 1],
        corner_uv: [[0, 0], [0, 1], [1, 1], [1, 0]],
    },
    FaceDef {
        normal: [0, 0, 1],
        corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        axis_u: [1, 0, 0],
        axis_v: [0, 1, 0],
        corner_uv: [[0, 0], [1, 0], [1, 1], [0, 1]],
    },
    FaceDef {
        normal: [0, 0, -1],
        corners: [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]],
        uvs: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        axis_u: [1, 0, 0],
        axis_v: [0, 1, 0],
        corner_uv: [[0, 0], [0, 1], [1, 1], [1, 0]],
    },
];

fn uv_rect(rects: &[[f32; 4]], tile: u8, size: [u32; 2]) -> (f32, f32, f32, f32) {
    let rect = rects[(tile as usize).min(rects.len() - 1)];
    let inset_u = 0.25 / size[0] as f32;
    let inset_v = 0.25 / size[1] as f32;
    (
        rect[0] + inset_u,
        rect[1] + inset_v,
        rect[2] - inset_u,
        rect[3] - inset_v,
    )
}

fn block_tile_uv(tile: u8) -> (f32, f32, f32, f32) {
    uv_rect(&BLOCK_UV_RECTS, tile, BLOCK_ATLAS_SIZE)
}

fn flora_tile_uv(tile: u8) -> (f32, f32, f32, f32) {
    uv_rect(&FLORA_UV_RECTS, tile, FLORA_ATLAS_SIZE)
}

fn leaf_tile_uv(tile: u8) -> (f32, f32, f32, f32) {
    uv_rect(&LEAF_UV_RECTS, tile, LEAF_ATLAS_SIZE)
}

fn block_tint(world: &VoxelWorld, block: Block, face_up: i32, x: i32, z: i32) -> Vec3 {
    if block.is_grass_block() && face_up == 1 {
        world.grass_tint_at(x, z)
    } else {
        Vec3::ONE
    }
}

pub(crate) struct MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshData {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    fn push_face(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        face: &FaceDef,
        tile: u8,
        tint: Vec3,
        ao: [f32; 4],
        water_top: f32,
        uv_full: bool,
        uv_shrink: f32,
        wind: f32,
        leaf_atlas: bool,
    ) {
        let (mut u0, mut v0, mut u1, mut v1) = if uv_full {
            (0.0, 0.0, 1.0, 1.0)
        } else if leaf_atlas {
            leaf_tile_uv(tile)
        } else {
            block_tile_uv(tile)
        };
        if uv_shrink > 0.0 {
            let du = (u1 - u0) * uv_shrink;
            let dv = (v1 - v0) * uv_shrink;
            u0 += du;
            v0 += dv;
            u1 -= du;
            v1 -= dv;
        }
        let start = self.positions.len() as u32;
        for i in 0..4 {
            let corner = face.corners[i];
            let y_offset = if water_top > 0.0 && corner[1] == 1 {
                water_top
            } else {
                corner[1] as f32
            };
            self.positions.push([
                x + corner[0] as f32,
                y + y_offset,
                z + corner[2] as f32,
            ]);
            self.normals.push([
                face.normal[0] as f32,
                face.normal[1] as f32,
                face.normal[2] as f32,
            ]);
            let uv = face.uvs[i];
            self.uvs
                .push([u0 + uv[0] * (u1 - u0), v0 + uv[1] * (v1 - v0)]);
            let shade = ao[i];
            self.colors.push([
                tint.x * shade,
                tint.y * shade,
                tint.z * shade,
                wind,
            ]);
        }
        self.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    }

    fn push_cross(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        tile: u8,
        tint: Vec3,
        rotation: f32,
        height: f32,
    ) {
        let (u0, v0, u1, v1) = flora_tile_uv(tile);
        let angle = rotation * std::f32::consts::FRAC_PI_4;
        let (sin, cos) = angle.sin_cos();
        let planes = [
            [[0.15f32, 0.15f32], [0.85, 0.85]],
            [[0.85, 0.15], [0.15, 0.85]],
        ];

        for plane in planes {
            let (start_x, start_z) = rotate_xz(plane[0][0], plane[0][1], sin, cos);
            let (end_x, end_z) = rotate_xz(plane[1][0], plane[1][1], sin, cos);
            let corners = [
                (start_x, 0.0, start_z, 0.0, 1.0),
                (end_x, 0.0, end_z, 1.0, 1.0),
                (end_x, height, end_z, 1.0, 0.0),
                (start_x, height, start_z, 0.0, 0.0),
            ];
            let base = self.positions.len() as u32;
            for (px, py, pz, tu, tv) in corners {
                self.positions.push([x + px, y + py, z + pz]);
                self.normals.push([0.0, 1.0, 0.0]);
                self.uvs
                    .push([u0 + tu * (u1 - u0), v0 + tv * (v1 - v0)]);
                self.colors.push([tint.x, tint.y, tint.z, py / height]);
            }
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    fn push_model_face(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        corners: [[f32; 3]; 4],
        uv_rect: [f32; 4],
        tile: u8,
    ) {
        let (tu0, tv0, tu1, tv1) = flora_tile_uv(tile);
        let [mu0, mv0, mu1, mv1] = uv_rect;
        let uvs = [
            [mu0 / 16.0, mv1 / 16.0],
            [mu1 / 16.0, mv1 / 16.0],
            [mu1 / 16.0, mv0 / 16.0],
            [mu0 / 16.0, mv0 / 16.0],
        ];
        let base = self.positions.len() as u32;
        for i in 0..4 {
            self.positions.push([
                x + corners[i][0],
                y + corners[i][1],
                z + corners[i][2],
            ]);
            self.normals.push([0.0, 1.0, 0.0]);
            self.uvs.push([
                tu0 + uvs[i][0] * (tu1 - tu0),
                tv0 + uvs[i][1] * (tv1 - tv0),
            ]);
            self.colors.push([1.0, 1.0, 1.0, 0.0]);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn push_model_box(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        from: [f32; 3],
        to: [f32; 3],
        uvs: [[f32; 4]; 6],
        tile: u8,
    ) {
        let [x0, y0, z0] = from;
        let [x1, y1, z1] = to;
        let faces = [
            ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], uvs[0]),
            ([[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]], uvs[1]),
            ([[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], uvs[2]),
            ([[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]], uvs[3]),
            ([[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], uvs[4]),
            ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], uvs[5]),
        ];
        for (corners, uv_rect) in faces {
            self.push_model_face(x, y, z, corners, uv_rect, tile);
        }
    }

    fn push_mushroom(&mut self, x: f32, y: f32, z: f32, block: Block) {
        let tile = block.texture(0);
        let (hat_from, hat_to, stem_from, stem_to, hat_uv, stem_uv) =
            match block {
                Block::RedMushroom => (
                    [6.0, 3.0, 6.0],
                    [10.0, 6.0, 10.0],
                    [7.0, 0.0, 7.0],
                    [9.0, 3.0, 9.0],
                    [
                        [4.0, 0.0, 8.0, 3.0],
                        [12.0, 0.0, 16.0, 3.0],
                        [0.0, 0.0, 4.0, 3.0],
                        [8.0, 0.0, 12.0, 3.0],
                        [12.0, 3.0, 16.0, 7.0],
                        [12.0, 7.0, 16.0, 11.0],
                    ],
                    [
                        [14.0, 12.0, 16.0, 15.0],
                        [14.0, 12.0, 16.0, 15.0],
                        [12.0, 12.0, 14.0, 15.0],
                        [12.0, 12.0, 14.0, 15.0],
                        [2.0, 12.0, 4.0, 14.0],
                        [2.0, 12.0, 4.0, 14.0],
                    ],
                ),
                _ => (
                    [5.0, 4.0, 5.0],
                    [11.0, 6.0, 11.0],
                    [7.0, 0.0, 7.0],
                    [9.0, 4.0, 9.0],
                    [
                        [6.0, 8.0, 12.0, 10.0],
                        [6.0, 8.0, 12.0, 10.0],
                        [6.0, 8.0, 12.0, 10.0],
                        [6.0, 8.0, 12.0, 10.0],
                        [6.0, 10.0, 12.0, 16.0],
                        [0.0, 10.0, 6.0, 16.0],
                    ],
                    [
                        [14.0, 11.0, 16.0, 15.0],
                        [14.0, 11.0, 16.0, 15.0],
                        [12.0, 11.0, 14.0, 15.0],
                        [12.0, 11.0, 14.0, 15.0],
                        [7.0, 3.0, 9.0, 5.0],
                        [7.0, 3.0, 9.0, 5.0],
                    ],
                ),
            };
        let scale = 1.0 / 16.0;
        let scaled = |v: [f32; 3]| [v[0] * scale, v[1] * scale, v[2] * scale];
        self.push_model_box(x, y, z, scaled(stem_from), scaled(stem_to), stem_uv, tile);
        self.push_model_box(x, y, z, scaled(hat_from), scaled(hat_to), hat_uv, tile);
    }

    fn push_baked_quad(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        quad: &LeafQuad,
        tile: u8,
        tint: Vec3,
        scale: f32,
        yaw: f32,
        jitter: [f32; 3],
    ) {
        let (u0, v0, u1, v1) = leaf_tile_uv(tile);
        let (sin, cos) = yaw.sin_cos();
        let base = self.positions.len() as u32;
        for i in 0..4 {
            let position = quad.positions[i];
            let px = (position[0] - 0.5) * scale + 0.5 + jitter[0];
            let py = (position[1] - 0.5) * scale + 0.5 + jitter[1];
            let pz = (position[2] - 0.5) * scale + 0.5 + jitter[2];
            let dx = px - 0.5;
            let dz = pz - 0.5;
            self.positions.push([
                x + 0.5 + dx * cos - dz * sin,
                y + py,
                z + 0.5 + dx * sin + dz * cos,
            ]);
            self.normals.push([0.0, 1.0, 0.0]);
            let uv = quad.uvs[i];
            self.uvs
                .push([u0 + uv[0] * (u1 - u0), v0 + uv[1] * (v1 - v0)]);
            self.colors.push([tint.x, tint.y, tint.z, 1.0]);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

fn rotate_xz(x: f32, z: f32, sin: f32, cos: f32) -> (f32, f32) {
    let dx = x - 0.5;
    let dz = z - 0.5;
    (0.5 + dx * cos - dz * sin, 0.5 + dx * sin + dz * cos)
}

fn vertex_ao(
    world: &VoxelWorld,
    x: i32,
    y: i32,
    z: i32,
    face: &FaceDef,
    cu: i32,
    cv: i32,
) -> f32 {    let su = if cu == 0 { -1 } else { 1 };
    let sv = if cv == 0 { -1 } else { 1 };
    let n = face.normal;
    let a = face.axis_u;
    let b = face.axis_v;
    let side_a = world.is_solid_rendered(
        x + n[0] + a[0] * su,
        y + n[1] + a[1] * su,
        z + n[2] + a[2] * su,
    ) as u8;
    let side_b = world.is_solid_rendered(
        x + n[0] + b[0] * sv,
        y + n[1] + b[1] * sv,
        z + n[2] + b[2] * sv,
    ) as u8;
    let corner = world.is_solid_rendered(
        x + n[0] + a[0] * su + b[0] * sv,
        y + n[1] + a[1] * su + b[1] * sv,
        z + n[2] + a[2] * su + b[2] * sv,
    ) as u8;
    let level = if side_a == 1 && side_b == 1 {
        0
    } else {
        3 - (side_a + side_b + corner)
    };
    0.72 + 0.28 * (level as f32 / 3.0)
}

fn sky_light(
    column_height: i32,
    x: i32,
    y: i32,
    z: i32,
    normal: [i32; 3],
    world: &VoxelWorld,
) -> f32 {
    let blocking = |block: Block| block.is_opaque() || block.is_leaves();
    if y >= column_height
        && (1..=3).all(|dy| !blocking(world.get_block(x, y + dy, z)))
    {
        return 1.0;
    }
    let n = Vec3::new(normal[0] as f32, normal[1] as f32, normal[2] as f32);
    let origin = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) + n * 0.55;
    let mut sum = 0.0f32;
    let mut count = 0.0f32;
    for dir in [
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Z,
        Vec3::NEG_Z,
    ] {
        if dir.dot(n) < -0.1 {
            continue;
        }
        sum += ray_visibility(world, origin, dir);
        count += 1.0;
    }
    let visibility = sum / count.max(1.0);
    (0.05 + 0.95 * visibility.powf(1.6)).min(1.0)
}

fn ray_visibility(world: &VoxelWorld, origin: Vec3, dir: Vec3) -> f32 {
    const REACH: f32 = 10.0;
    let mut t = 1.0;
    while t <= REACH {
        let point = origin + dir * t;
        let block = world.get_block(
            point.x.floor() as i32,
            point.y.floor() as i32,
            point.z.floor() as i32,
        );
        if block.is_opaque() || block.is_leaves() {
            return (t / REACH).clamp(0.0, 1.0);
        }
        t += 1.0;
    }
    1.0
}

#[derive(Default)]
pub struct SectionMeshes {
    pub opaque: Option<MeshData>,
    pub water: Option<MeshData>,
    pub overlay: Option<MeshData>,
    pub leaves: Option<MeshData>,
    pub flora: Option<MeshData>,
}

fn section_surrounded(world: &VoxelWorld, pos: ChunkPos, section: i32) -> bool {
    for (dx, dy, dz) in [
        (0, 1, 0),
        (0, -1, 0),
        (1, 0, 0),
        (-1, 0, 0),
        (0, 0, 1),
        (0, 0, -1),
    ] {
        let neighbor = section + dy;
        if !(0..SECTION_COUNT).contains(&neighbor) {
            continue;
        }
        let chunk = if dx != 0 || dz != 0 {
            world.chunk(ChunkPos::new(pos.x + dx, pos.z + dz))
        } else {
            world.chunk(pos)
        };
        let Some(chunk) = chunk else {
            return false;
        };
        if chunk.section_solid_count(neighbor) != SECTION_VOLUME {
            return false;
        }
    }
    true
}

pub fn build_section(
    world: &VoxelWorld,
    pos: ChunkPos,
    section: i32,
) -> SectionMeshes {
    let Some(chunk) = world.chunk(pos) else {
        return SectionMeshes::default();
    };
    if chunk.section_block_count(section) == 0 {
        return SectionMeshes::default();
    }
    if chunk.section_solid_count(section) == SECTION_VOLUME
        && section_surrounded(world, pos, section)
    {
        return SectionMeshes::default();
    }

    let origin_x = pos.origin_x();
    let origin_z = pos.origin_z();
    let origin_y = section_origin_y(section);
    let mut opaque = MeshData::new();
    let mut water = MeshData::new();
    let mut overlay = MeshData::new();
    let mut leaves = MeshData::new();
    let mut flora = MeshData::new();
    let seed = world.seed();

    for local_z in 0..CHUNK {
        for local_x in 0..CHUNK {
            let world_x = origin_x + local_x;
            let world_z = origin_z + local_z;
            for local_y in 0..SECTION {
                let world_y = origin_y + local_y;
                let block = chunk.get_block_local(local_x, world_y, local_z);
                if block.is_air() {
                    continue;
                }
                if world.is_occluded(world_x, world_y, world_z) {
                    continue;
                }

                if block.is_plant() {
                    if matches!(block, Block::RedMushroom | Block::BrownMushroom) {
                        flora.push_mushroom(
                            local_x as f32,
                            local_y as f32 + 0.002,
                            local_z as f32,
                            block,
                        );
                        continue;
                    }
                    let rotation = hash2(world_x, world_z, seed ^ 0x1c9e_4f21);
                    let wobble = hash2(world_x, world_z, seed ^ 0x7a3d_91b7);
                    let tint = if block.is_grass_plant() {
                        world.grass_tint_at(world_x, world_z)
                    } else {
                        Vec3::ONE
                    };
                    let height = match block {
                        Block::HighGrass => 1.25 + wobble * 0.4,
                        Block::PrairieGrass | Block::BeachGrass => 0.95 + wobble * 0.35,
                        Block::DesertGrass | Block::DeadGrass => 0.5 + wobble * 0.3,
                        Block::Sprout => 0.22 + wobble * 0.08,
                        Block::Clover => 0.3 + wobble * 0.15,
                        Block::Cattail => 1.15 + wobble * 0.45,
                        Block::Daisy => 0.55 + wobble * 0.2,
                        _ if block.is_grass_plant() => 0.9 + wobble * 0.35,
                        _ => 0.75 + wobble * 0.2,
                    };
                    flora.push_cross(
                        local_x as f32,
                        local_y as f32 + 0.002,
                        local_z as f32,
                        block.texture(0),
                        tint,
                        rotation,
                        height,
                    );
                    if !block.is_water_plant() {
                        continue;
                    }
                }

                if block.is_leaves() {
                    let leaf_tile = block.texture(0);
                    let tint = if matches!(
                        block,
                        Block::CherryLeaves | Block::MagnoliaLeaves | Block::JacarandaLeaves
                    ) {
                        Vec3::ONE
                    } else {
                        world.foliage_tint_at(world_x, world_z)
                    };
                    for face in FACES.iter() {
                        let normal = face.normal;
                        let neighbor = world.get_block(
                            world_x + normal[0],
                            world_y + normal[1],
                            world_z + normal[2],
                        );
                        if neighbor.is_opaque() || neighbor.is_leaves() {
                            continue;
                        }
                    let mut ao = [1.0f32; 4];
                        for (i, corner) in face.corner_uv.iter().enumerate() {
                            ao[i] = vertex_ao(
                                world,
                                world_x,
                                world_y,
                                world_z,
                                face,
                                corner[0],
                                corner[1],
                            );
                        }
                        leaves.push_face(
                            local_x as f32,
                            local_y as f32,
                            local_z as f32,
                            face,
                            leaf_tile,
                            tint,
                            ao,
                            0.0,
                            false,
                            0.0,
                            1.0,
                            true,
                        );
                    }
                    let count = LEAF_VARIANTS.len();
                    let first = (hash2(world_x, world_z, seed ^ 0x4c11_7a9d) * count as f32) as usize % count;
                    for variant_index in 0..3usize {
                        let variant = (first + variant_index * variant_index + variant_index) % count;
                        for (quad_index, quad) in LEAF_VARIANTS[variant].iter().enumerate() {
                            let salt = (variant_index as u32) * 131 + (quad_index as u32) * 17;
                            let yaw = hash2(world_x.wrapping_add(salt as i32), world_z, seed ^ 0x2a5e_13d7)
                                * std::f32::consts::TAU;
                            leaves.push_baked_quad(
                                local_x as f32,
                                local_y as f32,
                                local_z as f32,
                                quad,
                                leaf_tile,
                                tint,
                                1.12,
                                yaw,
                                [0.0, 0.0, 0.0],
                            );
                        }
                    }
                    continue;
                }

                if block.is_water() || block.is_water_plant() {
                    let above = world.get_block(world_x, world_y + 1, world_z);
                    let water_top = if above.is_water() || above.is_water_plant() {
                        WATER_SURFACE
                    } else if block.is_water_plant() {
                        WATER_SURFACE
                    } else {
                        match world.water_level(world_x, world_y, world_z) {
                            Some(level) => {
                                (WATER_SURFACE - level as f32 * 0.1).max(0.15)
                            }
                            None => WATER_SURFACE,
                        }
                    };
                    for face in FACES.iter() {
                        if face.normal[1] < 0 {
                            continue;
                        }
                        let neighbor = world.get_block(
                            world_x + face.normal[0],
                            world_y + face.normal[1],
                            world_z + face.normal[2],
                        );
                        if neighbor.is_water() || neighbor.is_water_plant() {
                            continue;
                        }
                        if neighbor.is_opaque()
                            && !world.is_occluded(
                                world_x + face.normal[0],
                                world_y + face.normal[1],
                                world_z + face.normal[2],
                            )
                        {
                            continue;
                        }
                        water.push_face(
                            local_x as f32,
                            local_y as f32,
                            local_z as f32,
                            face,
                            TEX_WATER_STILL,
                            Vec3::ONE,
                            [1.0; 4],
                            water_top,
                            true,
                            0.0,
                            1.0,
                            false,
                        );
                    }
                    continue;
                }

                let column = chunk.column(local_x, local_z);
                let column_height = column.height;
                for face in FACES.iter() {
                    let normal = face.normal;
                    let neighbor = world.get_block(
                        world_x + normal[0],
                        world_y + normal[1],
                        world_z + normal[2],
                    );
                    if neighbor.is_opaque()
                        && !world.is_occluded(
                            world_x + normal[0],
                            world_y + normal[1],
                            world_z + normal[2],
                        )
                    {
                        continue;
                    }
                    let water_depth01 = if neighbor.is_water() || neighbor.is_water_plant() {
                        ((column.water_top - world_y) as f32 / 14.0).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let water_filter = Vec3::new(
                        1.0 - 0.82 * water_depth01,
                        1.0 - 0.64 * water_depth01,
                        1.0 - 0.42 * water_depth01,
                    );
                    let mut ao = [1.0f32; 4];
                    for (i, corner) in face.corner_uv.iter().enumerate() {
                        ao[i] = vertex_ao(
                            world,
                            world_x,
                            world_y,
                            world_z,
                            face,
                            corner[0],
                            corner[1],
                        );
                    }
                    let light = sky_light(column_height, world_x, world_y, world_z, normal, world);
                    let uv_shrink = 0.02f32;
                    opaque.push_face(
                        local_x as f32,
                        local_y as f32,
                        local_z as f32,
                        face,
                        block.texture(normal[1]),
                        block_tint(world, block, normal[1], world_x, world_z) * light * water_filter,
                        ao,
                        0.0,
                        false,
                        uv_shrink,
                        water_depth01,
                        false,
                    );

                    if block.is_grass_block() && normal[1] == 0 {
                        overlay.push_face(
                            local_x as f32 + normal[0] as f32 * 0.012,
                            local_y as f32,
                            local_z as f32 + normal[2] as f32 * 0.012,
                            face,
                            TEX_GRASS_BLOCK_SIDE_OVERLAY,
                            world.grass_tint_at(world_x, world_z) * light * water_filter,
                            ao,
                            0.0,
                            false,
                            0.02,
                            water_depth01,
                            false,
                        );
                    }
                }

            }
        }
    }

    SectionMeshes {
        opaque: (!opaque.is_empty()).then_some(opaque),
        water: (!water.is_empty()).then_some(water),
        overlay: (!overlay.is_empty()).then_some(overlay),
        leaves: (!leaves.is_empty()).then_some(leaves),
        flora: (!flora.is_empty()).then_some(flora),
    }
}

impl MeshData {
    pub fn build(self) -> Mesh {
        self.into_mesh()
    }
}

pub fn sky_cube_mesh() -> Mesh {
    let positions = vec![
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let indices = vec![
        0, 1, 2, 0, 2, 3, 4, 7, 6, 4, 6, 5, 1, 5, 6, 1, 6, 2, 4, 0, 3, 4, 3, 7, 3, 2, 6, 3, 6, 7,
        0, 4, 5, 0, 5, 1,
    ];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
