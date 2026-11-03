use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
};

use crate::{
    materials::{CloudMaterial, GameMaterials},
    player::Player,
    reflection::ReflectionCamera,
    world::hash2,
};

const CELLS: i32 = 192;

#[derive(Component)]
pub struct CloudLayer {
    pub index: usize,
    speed: Vec2,
    period: f32,
}

const LAYERS: [(f32, f32, f32, f32, u32, f32, f32); 3] = [
    (192.0, 12.0, 4.0, 0.54, 0x2c4a_1171, 0.7, 0.10),
    (204.0, 9.0, 3.0, 0.42, 0x91b2_0c3d, 0.45, 0.06),
    (216.0, 7.0, 2.0, 0.30, 0x5a17_9e21, 0.28, 0.035),
];

fn smooth(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn periodic_noise(x: f32, z: f32, lattice: i32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let z0 = z.floor() as i32;
    let tx = x - x0 as f32;
    let tz = z - z0 as f32;
    let wrap = |value: i32| value.rem_euclid(lattice);
    let a = hash2(wrap(x0), wrap(z0), seed);
    let b = hash2(wrap(x0 + 1), wrap(z0), seed);
    let c = hash2(wrap(x0), wrap(z0 + 1), seed);
    let d = hash2(wrap(x0 + 1), wrap(z0 + 1), seed);
    let u = smooth(tx);
    let v = smooth(tz);
    let ab = a + (b - a) * u;
    let cd = c + (d - c) * u;
    ab + (cd - ab) * v
}

fn present(i: i32, j: i32, density: f32, seed: u32) -> bool {
    let base = periodic_noise(i as f32 * 0.25, j as f32 * 0.25, 16, seed);
    let detail = periodic_noise(i as f32 * 0.5, j as f32 * 0.5, 32, seed ^ 0x9e37_79b9);
    let value = base * 0.82 + detail * 0.18;
    value > 1.0 - density
}

fn build_layer(cell: f32, thickness: f32, density: f32, seed: u32) -> Mesh {
    let half = CELLS as f32 * cell * 0.5;
    let index = |i: i32, j: i32| (i.rem_euclid(CELLS) + j.rem_euclid(CELLS) * CELLS) as usize;
    let filled: Vec<bool> = (0..(CELLS * CELLS))
        .map(|cell_index| {
            let i = cell_index % CELLS;
            let j = cell_index / CELLS;
            present(i, j, density, seed)
        })
        .collect();
    let occupied = |i: i32, j: i32| filled[index(i, j)];

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut quad = |corners: [[f32; 3]; 4], normal: [f32; 3], shade: f32| {
        let start = positions.len() as u32;
        for corner in corners {
            positions.push(corner);
            normals.push(normal);
            uvs.push([0.0, 0.0]);
            colors.push([shade, shade, shade, 1.0]);
        }
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    };

    for j in 0..CELLS {
        for i in 0..CELLS {
            if !occupied(i, j) {
                continue;
            }
            let shade = 0.9 + hash2(i, j, seed ^ 0x51f3_9c2b) * 0.1;
            let x0 = -half + i as f32 * cell;
            let z0 = -half + j as f32 * cell;
            let x1 = x0 + cell;
            let z1 = z0 + cell;
            let y0 = 0.0;
            let y1 = thickness;

            quad(
                [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                [0.0, 1.0, 0.0],
                shade,
            );
            if !occupied(i, j - 1) {
                quad(
                    [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
                    [0.0, 0.0, -1.0],
                    shade * 0.94,
                );
            }
            if !occupied(i, j + 1) {
                quad(
                    [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                    [0.0, 0.0, 1.0],
                    shade * 0.94,
                );
            }
            if !occupied(i - 1, j) {
                quad(
                    [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                    [-1.0, 0.0, 0.0],
                    shade * 0.9,
                );
            }
            if !occupied(i + 1, j) {
                quad(
                    [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
                    [1.0, 0.0, 0.0],
                    shade * 0.9,
                );
            }
            quad(
                [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                [0.0, -1.0, 0.0],
                shade * 0.82,
            );
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn spawn_clouds(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    handles: &GameMaterials,
) {
    for (index, &(altitude, cell, thickness, density, seed, speed_x, speed_z)) in
        LAYERS.iter().enumerate()
    {
        let mesh = build_layer(cell, thickness, density, seed);
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(handles.cloud[index].clone()),
            Transform::from_xyz(0.0, altitude, 0.0),
            CloudLayer {
                index,
                speed: Vec2::new(speed_x, speed_z),
                period: CELLS as f32 * cell,
            },
            RenderLayers::layer(3),
            NotShadowCaster,
        ));
    }
}

pub fn move_clouds(
    time: Res<Time>,
    player: Query<&Player, With<Camera3d>>,
    camera: Query<&GlobalTransform, (With<Camera3d>, Without<ReflectionCamera>)>,
    handles: Res<GameMaterials>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    mut layers: Query<(&CloudLayer, &mut Transform)>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    if let Ok(camera) = camera.single() {
        let position = camera.translation();
        for handle in &handles.cloud {
            if let Some(mut material) = materials.get_mut(handle) {
                material.params.camera = position.extend(0.0);
            }
        }
    }
    let seconds = time.elapsed_secs();
    for (layer, mut transform) in &mut layers {
        let drift_x = seconds * layer.speed.x;
        let drift_z = seconds * layer.speed.y;
        transform.translation.x =
            drift_x + ((player.body.x - drift_x) / layer.period).round() * layer.period;
        transform.translation.z =
            drift_z + ((player.body.z - drift_z) / layer.period).round() * layer.period;
    }
}
