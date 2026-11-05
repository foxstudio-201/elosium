use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::PathBuf,
    rc::Rc,
};

use serde_json::Value;

const DATA_ROOT: &str = "assets/worldgen/data/minecraft/worldgen";

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

pub fn surface_height(x: i32, z: i32, seed: u32, world_type: u8) -> Option<i32> {
    if world_type == 1 {
        return None;
    }
    SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let recreate = match slot.as_ref() {
            Some(session) => session.seed != seed || session.kind != world_type,
            None => true,
        };
        if recreate {
            *slot = Some(Session::new(seed, world_type));
        }
        slot.as_mut().and_then(|session| session.surface_height(x, z))
    })
}

pub fn climate_sample(x: i32, z: i32, seed: u32, world_type: u8) -> Option<[f32; 6]> {
    if world_type == 1 {
        return None;
    }
    SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let recreate = match slot.as_ref() {
            Some(session) => session.seed != seed || session.kind != world_type,
            None => true,
        };
        if recreate {
            *slot = Some(Session::new(seed, world_type));
        }
        let session = slot.as_mut()?;
        let names = ["temperature", "vegetation", "continents", "erosion", "ridges_folded"];
        let mut out = [0.0f32; 6];
        for (index, name) in names.iter().enumerate() {
            let full = format!("minecraft:overworld/{name}");
            let node = session.function(&full)?;
            let value = session.eval(&node, x as f32, 0.0, z as f32);
            match *name {
                "ridges_folded" => out[5] = value,
                _ => out[index] = value,
            }
        }
        Some(out)
    })
}

struct Session {
    seed: u32,
    kind: u8,
    prefix: String,
    functions: HashMap<String, Option<Rc<Value>>>,
    noises: HashMap<String, Rc<NormalNoise>>,
    cache_2d: HashMap<(String, i32, i32), f32>,
    heights: HashMap<(i32, i32), i32>,
}

impl Session {
    fn new(seed: u32, kind: u8) -> Self {
        let prefix = match kind {
            2 => "overworld_large_biomes",
            3 => "overworld_amplified",
            _ => "overworld",
        };
        Self {
            seed,
            kind,
            prefix: prefix.into(),
            functions: HashMap::new(),
            noises: HashMap::new(),
            cache_2d: HashMap::new(),
            heights: HashMap::new(),
        }
    }

    fn surface_height(&mut self, x: i32, z: i32) -> Option<i32> {
        if let Some(height) = self.heights.get(&(x, z)) {
            return Some(*height);
        }
        if self.heights.len() > 400_000 {
            self.heights.clear();
            self.cache_2d.clear();
        }
        let name = format!("minecraft:{}/preliminary_surface_level", self.prefix);
        let node = self.function(&name)?;
        let height = self.find_top_surface(&node, x as f32, z as f32)?;
        self.heights.insert((x, z), height);
        Some(height)
    }

    fn function(&mut self, name: &str) -> Option<Rc<Value>> {
        if let Some(cached) = self.functions.get(name) {
            return cached.clone();
        }
        let relative = name.strip_prefix("minecraft:").unwrap_or(name);
        let path = PathBuf::from(DATA_ROOT)
            .join("density_function")
            .join(format!("{relative}.json"));
        let parsed = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(Rc::new);
        self.functions.insert(name.to_string(), parsed.clone());
        parsed
    }

    fn two_dimensional(name: &str) -> bool {
        let tail = name.rsplit('/').next().unwrap_or(name);
        matches!(
            tail,
            "continents"
                | "erosion"
                | "ridges"
                | "ridges_folded"
                | "offset"
                | "factor"
                | "temperature"
                | "vegetation"
                | "depth"
                | "jaggedness"
                | "shift_x"
                | "shift_z"
        )
    }

    fn eval(&mut self, node: &Value, x: f32, y: f32, z: f32) -> f32 {
        match node {
            Value::Null => 0.0,
            Value::Number(number) => number.as_f64().unwrap_or(0.0) as f32,
            Value::String(name) => {
                if Self::two_dimensional(name) {
                    let key = (name.clone(), x.floor() as i32, z.floor() as i32);
                    if let Some(value) = self.cache_2d.get(&key) {
                        return *value;
                    }
                    let Some(function) = self.function(name) else {
                        return 0.0;
                    };
                    let value = self.eval(&function, x, y, z);
                    self.cache_2d.insert(key, value);
                    value
                } else {
                    let Some(function) = self.function(name) else {
                        return 0.0;
                    };
                    self.eval(&function, x, y, z)
                }
            }
            Value::Object(object) => {
                let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
                match kind {
                    "minecraft:constant" => {
                        object.get("value").and_then(Value::as_f64).unwrap_or(0.0) as f32
                    }
                    "minecraft:abs" => self.eval(&object["input"], x, y, z).abs(),
                    "minecraft:square" => {
                        let v = self.eval(&object["input"], x, y, z);
                        v * v
                    }
                    "minecraft:cube" => {
                        let v = self.eval(&object["input"], x, y, z);
                        v * v * v
                    }
                    "minecraft:negate" => -self.eval(&object["input"], x, y, z),
                    "minecraft:half_negative" => {
                        let v = self.eval(&object["input"], x, y, z);
                        if v > 0.0 { v } else { v * 0.5 }
                    }
                    "minecraft:quarter_negative" => {
                        let v = self.eval(&object["input"], x, y, z);
                        if v > 0.0 { v } else { v * 0.25 }
                    }
                    "minecraft:squeeze" => {
                        let v = self.eval(&object["input"], x, y, z).clamp(-1.0, 1.0);
                        v / 2.0 - v * v * v / 24.0
                    }
                    "minecraft:add" => {
                        self.eval(&object["left"], x, y, z) + self.eval(&object["right"], x, y, z)
                    }
                    "minecraft:sub" => {
                        self.eval(&object["left"], x, y, z) - self.eval(&object["right"], x, y, z)
                    }
                    "minecraft:mul" => {
                        self.eval(&object["left"], x, y, z) * self.eval(&object["right"], x, y, z)
                    }
                    "minecraft:div" => {
                        let denominator = self.eval(&object["right"], x, y, z);
                        if denominator.abs() < 1e-8 {
                            0.0
                        } else {
                            self.eval(&object["left"], x, y, z) / denominator
                        }
                    }
                    "minecraft:min" => self
                        .eval(&object["left"], x, y, z)
                        .min(self.eval(&object["right"], x, y, z)),
                    "minecraft:max" => self
                        .eval(&object["left"], x, y, z)
                        .max(self.eval(&object["right"], x, y, z)),
                    "minecraft:clamp" => {
                        let value = self.eval(&object["input"], x, y, z);
                        let min = object["min"].as_f64().unwrap_or(-1.0) as f32;
                        let max = object["max"].as_f64().unwrap_or(1.0) as f32;
                        value.clamp(min, max)
                    }
                    "minecraft:lerp" => {
                        let alpha = self.eval(&object["alpha"], x, y, z);
                        let first = self.eval(&object["first"], x, y, z);
                        let second = self.eval(&object["second"], x, y, z);
                        first + alpha * (second - first)
                    }
                    "minecraft:gradient" => {
                        let axis = object.get("axis").and_then(Value::as_str).unwrap_or("y");
                        let coordinate = match axis {
                            "x" => x,
                            "z" => z,
                            _ => y,
                        };
                        let from = object.get("from_coordinate").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                        let to = object.get("to_coordinate").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                        let from_value = object.get("from_value").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                        let to_value = object.get("to_value").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                        let span = to - from;
                        if span.abs() < 1e-8 {
                            from_value
                        } else {
                            let t = ((coordinate - from) / span).clamp(0.0, 1.0);
                            from_value + t * (to_value - from_value)
                        }
                    }
                    "minecraft:noise" => {
                        let noise_name = object.get("noise").and_then(Value::as_str).unwrap_or("");
                        let xz_scale = object.get("xz_scale").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                        let y_scale = object.get("y_scale").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                        let mut shift_x = 0.0;
                        let mut shift_z = 0.0;
                        if let Some(node) = object.get("shift_x") {
                            shift_x = self.eval(node, x, y, z);
                        }
                        if let Some(node) = object.get("shift_z") {
                            shift_z = self.eval(node, x, y, z);
                        }
                        let noise = self.noise(noise_name);
                        noise.sample(
                            (x + shift_x) * xz_scale,
                            y * y_scale,
                            (z + shift_z) * xz_scale,
                        )
                    }
                    "minecraft:shift_a" => {
                        let noise_name = object.get("noise").and_then(Value::as_str).unwrap_or("");
                        let noise = self.noise(noise_name);
                        noise.sample(x * 0.25, 0.0, z * 0.25) * 4.0
                    }
                    "minecraft:shift_b" => {
                        let noise_name = object.get("noise").and_then(Value::as_str).unwrap_or("");
                        let noise = self.noise(noise_name);
                        noise.sample(z * 0.25, 0.0, x * 0.25) * 4.0
                    }
                    "minecraft:spline" => self.eval_spline(&object["spline"], x, y, z),
                    "minecraft:range_choice" => {
                        let value = self.eval(&object["input"], x, y, z);
                        let min = object.get("min_inclusive").and_then(Value::as_f64).unwrap_or(f32::MIN as f64) as f32;
                        let max = object.get("max_exclusive").and_then(Value::as_f64).unwrap_or(f32::MAX as f64) as f32;
                        if value >= min && value < max {
                            self.eval(&object["when_in_range"], x, y, z)
                        } else {
                            self.eval(&object["when_out_of_range"], x, y, z)
                        }
                    }
                    "minecraft:cache" | "minecraft:cache_2d" | "minecraft:cache_once" => {
                        self.eval(&object["input"], x, y, z)
                    }
                    "minecraft:blend_density" => self.eval(&object["input"], x, y, z),
                    "minecraft:blend_alpha" => 1.0,
                    "minecraft:blend_offset" => 0.0,
                    "minecraft:interpolated" => self.eval(&object["input"], x, y, z),
                    "minecraft:find_top_surface" => {
                        self.eval_find_top_surface(node, x, z).unwrap_or(0) as f32
                    }
                    "minecraft:old_blended_noise" => 0.0,
                    "minecraft:shifta" | "minecraft:shiftb" => 0.0,
                    _ => 0.0,
                }
            }
            _ => 0.0,
        }
    }

    fn eval_spline(&mut self, spline: &Value, x: f32, y: f32, z: f32) -> f32 {
        let Some(points) = spline.get("points").and_then(Value::as_array) else {
            return 0.0;
        };
        if points.is_empty() {
            return 0.0;
        }
        let coordinate = match spline.get("coordinate") {
            Some(Value::String(name)) => self.eval(&Value::String(name.clone()), x, y, z),
            Some(node) => self.eval(node, x, y, z),
            None => 0.0,
        };
        let first = points[0]["location"].as_f64().unwrap_or(-1.0) as f32;
        let last = points[points.len() - 1]["location"]
            .as_f64()
            .unwrap_or(1.0) as f32;
        if coordinate <= first {
            let start_value = self.spline_value(&points[0]["value"], x, y, z);
            let derivative = points[0]["derivative"].as_f64().unwrap_or(0.0) as f32;
            return start_value + derivative * (coordinate - first);
        }
        if coordinate >= last {
            let end = &points[points.len() - 1];
            let end_value = self.spline_value(&end["value"], x, y, z);
            let derivative = end["derivative"].as_f64().unwrap_or(0.0) as f32;
            return end_value + derivative * (coordinate - last);
        }
        let position = coordinate;
        let mut index = 0usize;
        while index + 2 < points.len()
            && position >= points[index + 1]["location"].as_f64().unwrap_or(0.0) as f32
        {
            index += 1;
        }
        let start = &points[index];
        let end = &points[index + 1];
        let start_location = start["location"].as_f64().unwrap_or(0.0) as f32;
        let end_location = end["location"].as_f64().unwrap_or(1.0) as f32;
        let span = end_location - start_location;
        let start_value = self.spline_value(&start["value"], x, y, z);
        if span <= 0.0 {
            return start_value;
        }
        let end_value = self.spline_value(&end["value"], x, y, z);
        let start_derivative = start["derivative"].as_f64().unwrap_or(0.0) as f32;
        let end_derivative = end["derivative"].as_f64().unwrap_or(0.0) as f32;
        let t = ((position - start_location) / span).clamp(0.0, 1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * start_value
            + h10 * span * start_derivative
            + h01 * end_value
            + h11 * span * end_derivative
    }

    fn spline_value(&mut self, value: &Value, x: f32, y: f32, z: f32) -> f32 {
        match value {
            Value::Number(number) => number.as_f64().unwrap_or(0.0) as f32,
            Value::Object(_) => self.eval_spline(value, x, y, z),
            _ => 0.0,
        }
    }

    fn eval_find_top_surface(&mut self, object: &Value, x: f32, z: f32) -> Option<i32> {
        let lower = object
            .get("lower_bound")
            .and_then(Value::as_f64)
            .unwrap_or(-64.0) as i32;
        let upper = match object.get("upper_bound") {
            Some(node) => self.eval(node, x, 0.0, z) as i32,
            None => 320,
        }
        .clamp(lower, 320);
        let cell = object
            .get("cell_height")
            .and_then(Value::as_i64)
            .unwrap_or(8) as i32;
        let density = &object["density"];
        let mut y = upper;
        while y >= lower {
            if self.eval(density, x, y as f32, z) > 0.0 {
                return Some(y);
            }
            y -= cell.max(1);
        }
        Some(lower)
    }

    fn find_top_surface(&mut self, node: &Value, x: f32, z: f32) -> Option<i32> {
        if node.get("type").and_then(Value::as_str) == Some("minecraft:find_top_surface") {
            return self.eval_find_top_surface(node, x, z);
        }
        None
    }

    fn noise(&mut self, name: &str) -> Rc<NormalNoise> {
        if let Some(noise) = self.noises.get(name) {
            return noise.clone();
        }
        let relative = name.strip_prefix("minecraft:").unwrap_or(name);
        let path = PathBuf::from(DATA_ROOT)
            .join("noise")
            .join(format!("{relative}.json"));
        let noise = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|value| Rc::new(NormalNoise::new(self.seed, name, &value)))
            .unwrap_or_else(|| Rc::new(NormalNoise::empty()));
        self.noises.insert(name.to_string(), noise.clone());
        noise
    }
}

struct NormalNoise {
    first_octave: i32,
    amplitudes: Vec<f32>,
    permutation: [u8; 512],
    scale: f32,
}

impl NormalNoise {
    fn empty() -> Self {
        Self {
            first_octave: 0,
            amplitudes: Vec::new(),
            permutation: [0; 512],
            scale: 1.0,
        }
    }

    fn new(seed: u32, name: &str, value: &Value) -> Self {
        let first_octave = value
            .get("base_octave")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        let base_amplitude = value
            .get("base_amplitude")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32;
        let amplitudes: Vec<f32> = value
            .get("amplitude_modifiers")
            .and_then(Value::as_array)
            .map(|mods| {
                mods.iter()
                    .map(|modifier| {
                        base_amplitude * modifier.as_f64().unwrap_or(0.0) as f32
                    })
                    .collect()
            })
            .unwrap_or_default();
        let salt = hash_string(name);
        let scale = if name.ends_with("temperature") || name.ends_with("vegetation") {
            1.0
        } else {
            3.0
        };
        Self {
            first_octave,
            amplitudes,
            permutation: permutation(seed as u64 ^ salt),
            scale,
        }
    }

    fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        if self.amplitudes.is_empty() {
            return 0.0;
        }
        let normalization: f32 = self.amplitudes.iter().map(|a| a.abs()).sum();
        if normalization <= 1e-6 {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut frequency = 2f32.powi(self.first_octave);
        for amplitude in &self.amplitudes {
            if *amplitude != 0.0 {
                sum += perlin(&self.permutation, x * frequency, y * frequency, z * frequency)
                    * amplitude;
            }
            frequency *= 2.0;
        }
        sum / normalization * self.scale
    }
}

fn hash_string(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn permutation(seed: u64) -> [u8; 512] {
    let mut values: [u8; 256] = [0; 256];
    for (index, value) in values.iter_mut().enumerate() {
        *value = index as u8;
    }
    let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
    for index in (1..256).rev() {
        state = splitmix(state);
        let pick = (state >> 33) as usize % (index + 1);
        values.swap(index, pick);
    }
    let mut permutation = [0u8; 512];
    permutation[..256].copy_from_slice(&values);
    permutation[256..].copy_from_slice(&values);
    permutation
}

fn splitmix(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn gradient(hash: u8, x: f32, y: f32, z: f32) -> f32 {
    match hash & 15 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        11 => -y - z,
        12 => x + y,
        13 => -y + z,
        14 => y - x,
        _ => -y - z,
    }
}

fn perlin(permutation: &[u8; 512], x: f32, y: f32, z: f32) -> f32 {
    let floor_x = x.floor();
    let floor_y = y.floor();
    let floor_z = z.floor();
    let ix = floor_x as i32 & 255;
    let iy = floor_y as i32 & 255;
    let iz = floor_z as i32 & 255;
    let x = x - floor_x;
    let y = y - floor_y;
    let z = z - floor_z;
    let u = fade(x);
    let v = fade(y);
    let w = fade(z);
    let a = permutation[ix as usize] as usize + iy as usize;
    let aa = permutation[a] as usize + iz as usize;
    let ab = permutation[a + 1] as usize + iz as usize;
    let b = permutation[ix as usize + 1] as usize + iy as usize;
    let ba = permutation[b] as usize + iz as usize;
    let bb = permutation[b + 1] as usize + iz as usize;
    let lerp = |t: f32, a: f32, b: f32| a + t * (b - a);
    lerp(
        w,
        lerp(
            v,
            lerp(
                u,
                gradient(permutation[aa], x, y, z),
                gradient(permutation[ba], x - 1.0, y, z),
            ),
            lerp(
                u,
                gradient(permutation[ab], x, y - 1.0, z),
                gradient(permutation[bb], x - 1.0, y - 1.0, z),
            ),
        ),
        lerp(
            v,
            lerp(
                u,
                gradient(permutation[aa + 1], x, y, z - 1.0),
                gradient(permutation[ba + 1], x - 1.0, y, z - 1.0),
            ),
            lerp(
                u,
                gradient(permutation[ab + 1], x, y - 1.0, z - 1.0),
                gradient(permutation[bb + 1], x - 1.0, y - 1.0, z - 1.0),
            ),
        ),
    )
}
