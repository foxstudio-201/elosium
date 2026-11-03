import bevy_pbr::render::forward_io::VertexOutput;

struct SkyParams {
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    camera_pos: vec4<f32>,
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    timing: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: SkyParams;

fn hash21(point: vec2<f32>) -> f32 {
    var p = fract(point * vec2<f32>(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

fn hash31(point: vec3<f32>) -> f32 {
    var p = fract(point * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

fn value_noise(point: vec2<f32>) -> f32 {
    let cell = floor(point);
    let fraction = fract(point);
    let factor = fraction * fraction * (3.0 - 2.0 * fraction);
    let a = hash21(cell);
    let b = hash21(cell + vec2<f32>(1.0, 0.0));
    let c = hash21(cell + vec2<f32>(0.0, 1.0));
    let d = hash21(cell + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, factor.x), mix(c, d, factor.x), factor.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let direction = normalize(in.world_position.xyz - params.camera_pos.xyz);
    let sky = clamp(direction.y, 0.0, 1.0);
    let sun = normalize(params.sun_dir.xyz);
    let night = params.timing.y;
    let time = params.timing.x;

    var color = mix(params.horizon.rgb, params.zenith.rgb, pow(sky, 0.45));

    let below = clamp(-direction.y, 0.0, 1.0);
    color = mix(color, params.horizon.rgb * 0.72, below);

    let sun_amount = max(dot(direction, sun), 0.0);
    let sun_visible = smoothstep(-0.03, 0.06, sun.y);
    color += params.sun_color.rgb * pow(sun_amount, 900.0) * 6.0 * sun_visible;
    color += params.sun_color.rgb * pow(sun_amount, 16.0) * 0.22 * sun_visible;

    let moon_direction = -sun;
    let moon_amount = max(dot(direction, moon_direction), 0.0);
    color += vec3<f32>(0.72, 0.80, 1.0) * pow(moon_amount, 1400.0) * 4.0 * night;
    color += vec3<f32>(0.30, 0.38, 0.65) * pow(moon_amount, 20.0) * 0.18 * night;

    if direction.y > 0.0 && night > 0.01 {
        let cell = floor(direction * 380.0);
        let star = step(0.9982, hash31(cell));
        let twinkle = 0.6 + 0.4 * sin(time * 2.0 + hash31(cell + vec3<f32>(7.31)) * 12.0);
        color += vec3<f32>(0.85, 0.90, 1.0) * star * night * twinkle;
    }

    return vec4<f32>(color, 1.0);
}
