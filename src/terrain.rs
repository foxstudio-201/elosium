use crate::{
    vanilla_worldgen::{
        Coord, NoiseDef, Spline, SplineValue, CONTINENTALNESS, EROSION, JAGGED_NOISE, JAGGED_ROOT,
        OFFSET_BASE, OFFSET_NOISE, OFFSET_ROOT, RIDGE,
    },
    world::value_noise,
};

struct NoiseContext {
    continents: f32,
    erosion: f32,
    ridges: f32,
    ridges_folded: f32,
}

fn normal_noise(definition: &NoiseDef, x: f32, z: f32, seed: u32) -> f32 {
    let mut sum = 0.0f32;
    let mut normalization = 0.0f32;
    let mut frequency = (definition.base_octave as f32).exp2();
    for (octave, modifier) in definition.mods.iter().enumerate() {
        if *modifier != 0.0 {
            let salt = seed.wrapping_add((octave as u32).wrapping_mul(0x9e37_79b9));
            let sample = value_noise(x * frequency, z * frequency, salt) * 2.0 - 1.0;
            sum += definition.base_amplitude * modifier * sample;
            normalization += definition.base_amplitude * modifier.abs();
        }
        frequency *= 2.0;
    }
    if normalization <= 1e-6 {
        0.0
    } else {
        sum / normalization * 2.0
    }
}

fn shift_a(x: f32, z: f32, seed: u32) -> f32 {
    normal_noise(&OFFSET_NOISE, x * 0.25, z * 0.25, seed ^ 0x5eed_1111) * 4.0
}

fn shift_b(x: f32, z: f32, seed: u32) -> f32 {
    normal_noise(&OFFSET_NOISE, z * 0.25, x * 0.25, seed ^ 0x5eed_2222) * 4.0
}

fn shaped(definition: &NoiseDef, x: f32, z: f32, seed: u32) -> f32 {
    let sx = x * 0.25 + shift_a(x, z, seed);
    let sz = z * 0.25 + shift_b(x, z, seed);
    normal_noise(definition, sx, sz, seed)
}

fn coordinate(coord: Coord, context: &NoiseContext) -> f32 {
    match coord {
        Coord::Continents => context.continents,
        Coord::Erosion => context.erosion,
        Coord::Ridges => context.ridges,
        Coord::RidgesFolded => context.ridges_folded,
    }
}

fn value(value: &SplineValue, context: &NoiseContext) -> f32 {
    match value {
        SplineValue::Const(number) => *number,
        SplineValue::Nested(spline) => evaluate(spline, context),
    }
}

fn evaluate(spline: &Spline, context: &NoiseContext) -> f32 {
    let points = spline.points;
    if points.len() == 1 {
        return value(&points[0].value, context);
    }
    let first = points[0].location;
    let last = points[points.len() - 1].location;
    let position = coordinate(spline.coord, context).clamp(first, last);
    let mut index = 0usize;
    while index + 2 < points.len() && position >= points[index + 1].location {
        index += 1;
    }
    let start = &points[index];
    let end = &points[index + 1];
    let span = end.location - start.location;
    if span <= 0.0 {
        return value(&end.value, context);
    }
    let t = ((position - start.location) / span).clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * value(&start.value, context)
        + h10 * span * start.derivative
        + h01 * value(&end.value, context)
        + h11 * span * end.derivative
}

pub fn surface_height(x: i32, z: i32, seed: u32) -> i32 {
    let fx = x as f32;
    let fz = z as f32;
    let continents = shaped(&CONTINENTALNESS, fx, fz, seed ^ 0x3c17_51ab);
    let erosion = shaped(&EROSION, fx, fz, seed ^ 0x9d43_27e1);
    let ridges = shaped(&RIDGE, fx, fz, seed ^ 0x0a95_6b3f);
    let ridges_folded = ((ridges.abs() - 0.666_666_7).abs() - 0.333_333_34) * -3.0;
    let context = NoiseContext {
        continents,
        erosion,
        ridges,
        ridges_folded,
    };
    let offset = OFFSET_BASE + evaluate(OFFSET_ROOT, &context);
    let jaggedness = evaluate(JAGGED_ROOT, &context);
    let jagged = normal_noise(&JAGGED_NOISE, fx * 1500.0, fz * 1500.0, seed ^ 0x51aa_2d77);
    let jagged = jaggedness * if jagged < 0.0 { jagged * 0.5 } else { jagged };
    let height = -64.0 + (1.5 + offset + jagged) * 128.0;
    height.clamp(-64.0, 320.0).floor() as i32
}
