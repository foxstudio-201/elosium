use std::sync::OnceLock;

use crate::{biome_table::biome_from_index, blocks::Biome};

struct Entry {
    biome: u8,
    p: [[i32; 2]; 6],
}

struct Node {
    entry: i32,
    left: i32,
    right: i32,
    bmin: [i32; 6],
    bmax: [i32; 6],
}

struct Params {
    entries: Vec<Entry>,
    nodes: Vec<Node>,
    root: i32,
}

const DIMS: usize = 6;

fn build_nodes(idx: &mut [usize], entries: &[Entry], nodes: &mut Vec<Node>) -> i32 {
    if idx.is_empty() {
        return -1;
    }
    let mut bmin = [i32::MAX; DIMS];
    let mut bmax = [i32::MIN; DIMS];
    for &entry in idx.iter() {
        for dim in 0..DIMS {
            bmin[dim] = bmin[dim].min(entries[entry].p[dim][0]);
            bmax[dim] = bmax[dim].max(entries[entry].p[dim][1]);
        }
    }
    let mut axis = 0;
    let mut spread = 0;
    for dim in 0..DIMS {
        let mut lo = i32::MAX;
        let mut hi = i32::MIN;
        for &entry in idx.iter() {
            let center = (entries[entry].p[dim][0] + entries[entry].p[dim][1]) / 2;
            lo = lo.min(center);
            hi = hi.max(center);
        }
        if hi - lo > spread {
            spread = hi - lo;
            axis = dim;
        }
    }
    let node_index = nodes.len() as i32;
    nodes.push(Node {
        entry: -1,
        left: -1,
        right: -1,
        bmin,
        bmax,
    });
    if idx.len() == 1 || spread == 0 {
        nodes[node_index as usize].entry = idx[0] as i32;
        return node_index;
    }
    idx.sort_unstable_by_key(|&entry| {
        entries[entry].p[axis][0] + entries[entry].p[axis][1]
    });
    let mid = idx.len() / 2;
    let (left_idx, right_idx) = idx.split_at_mut(mid);
    let left = build_nodes(left_idx, entries, nodes);
    let right = build_nodes(right_idx, entries, nodes);
    nodes[node_index as usize].left = left;
    nodes[node_index as usize].right = right;
    node_index
}

fn point_distance(entry: &Entry, q: &[i32; DIMS]) -> i64 {
    let mut sum = 0i64;
    for dim in 0..DIMS {
        let [lo, hi] = entry.p[dim];
        let delta = if q[dim] < lo {
            (lo - q[dim]) as i64
        } else if q[dim] > hi {
            (q[dim] - hi) as i64
        } else {
            0
        };
        sum += delta * delta;
    }
    sum
}

fn box_distance(node: &Node, q: &[i32; DIMS]) -> i64 {
    let mut sum = 0i64;
    for dim in 0..DIMS {
        let delta = if q[dim] < node.bmin[dim] {
            (node.bmin[dim] - q[dim]) as i64
        } else if q[dim] > node.bmax[dim] {
            (q[dim] - node.bmax[dim]) as i64
        } else {
            0
        };
        sum += delta * delta;
    }
    sum
}

fn query(params: &Params, node: i32, q: &[i32; DIMS], best: &mut (i64, u8)) {
    if node < 0 {
        return;
    }
    let current = &params.nodes[node as usize];
    if box_distance(current, q) >= best.0 {
        return;
    }
    if current.entry >= 0 {
        let entry = &params.entries[current.entry as usize];
        let distance = point_distance(entry, q);
        if distance < best.0 {
            best.0 = distance;
            best.1 = entry.biome;
        }
    }
    let (left_bound, right_bound) = (
        if current.left >= 0 {
            box_distance(&params.nodes[current.left as usize], q)
        } else {
            i64::MAX
        },
        if current.right >= 0 {
            box_distance(&params.nodes[current.right as usize], q)
        } else {
            i64::MAX
        },
    );
    if left_bound <= right_bound {
        query(params, current.left, q, best);
        query(params, current.right, q, best);
    } else {
        query(params, current.right, q, best);
        query(params, current.left, q, best);
    }
}

fn params() -> &'static Params {
    static PARAMS: OnceLock<Params> = OnceLock::new();
    PARAMS.get_or_init(|| {
        let data: &[u8] = include_bytes!("../assets/worldgen/biome_params.bin");
        let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let mut offset = 4;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let biome = data[offset];
            offset += 1;
            let mut p = [[0i32; 2]; DIMS];
            for point in p.iter_mut() {
                let lo = i16::from_le_bytes([data[offset], data[offset + 1]]) as i32;
                let hi = i16::from_le_bytes([data[offset + 2], data[offset + 3]]) as i32;
                offset += 4;
                *point = [lo, hi];
            }
            entries.push(Entry { biome, p });
        }
        let mut nodes = Vec::with_capacity(count * 2);
        let mut indices: Vec<usize> = (0..count).collect();
        let root = build_nodes(&mut indices, &entries, &mut nodes);
        Params {
            entries,
            nodes,
            root,
        }
    })
}

pub fn select(climate: [f32; DIMS]) -> Biome {
    let params = params();
    let mut query_point = [0i32; DIMS];
    for dim in 0..DIMS {
        query_point[dim] = (climate[dim].clamp(-1.5, 1.5) * 10000.0).round() as i32;
    }
    let mut best = (i64::MAX, 0u8);
    query(params, params.root, &query_point, &mut best);
    biome_from_index(best.1)
}
