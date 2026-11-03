import bevy_pbr::render::{
    forward_io::VertexOutput,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct WaterParams {
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    camera_pos: vec4<f32>,
    fog: vec4<f32>,
    fog_color: vec4<f32>,
    water_color: vec4<f32>,
    sky_zenith: vec4<f32>,
    timing: vec4<f32>,
    reflection: vec4<f32>,
    underwater: vec4<f32>,
};

@group(3) @binding(100) var<uniform> params: WaterParams;
@group(3) @binding(101) var reflection_texture: texture_2d<f32>;
@group(3) @binding(102) var reflection_sampler: sampler;
@group(3) @binding(103) var water_texture: texture_2d<f32>;
@group(3) @binding(104) var water_sampler: sampler;
@group(3) @binding(105) var normal_texture: texture_2d<f32>;
@group(3) @binding(106) var normal_sampler: sampler;

const FRAME_COUNT: f32 = 32.0;

fn hash21(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn caustic(point: vec2<f32>, time: f32) -> f32 {
    var nearest = 8.0;
    var second = 8.0;
    let cell = floor(point);
    let local = fract(point);
    for (var j = -1; j <= 1; j = j + 1) {
        for (var i = -1; i <= 1; i = i + 1) {
            let cell_offset = vec2<f32>(f32(i), f32(j));
            let id = cell + cell_offset;
            let phase = 6.2831853 * hash21(id);
            let center = vec2<f32>(
                0.5 + 0.34 * sin(time * 0.85 + phase),
                0.5 + 0.34 * cos(time * 1.02 + phase * 1.37),
            );
            let step_distance = length(cell_offset + center - local);
            if (step_distance < nearest) {
                second = nearest;
                nearest = step_distance;
            } else if (step_distance < second) {
                second = step_distance;
            }
        }
    }
    let edge = second - nearest;
    return pow(1.0 - smoothstep(0.0, 0.5, edge), 2.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let time = params.timing.x;

    let position = in.world_position.xyz;
    let world_normal = normalize(in.world_normal.xyz);
    let frame = floor(time * 10.0);
    let wrapped = frame - FRAME_COUNT * floor(frame / FRAME_COUNT);
    let water_uv = vec2<f32>(
        (wrapped + min(in.uv.x, 0.999)) / FRAME_COUNT,
        in.uv.y,
    );
    let tex = textureSample(water_texture, water_sampler, water_uv).rgb;

    let ripple_a = textureSample(
        normal_texture,
        normal_sampler,
        position.xz * 0.05 + vec2<f32>(time * 0.010, time * 0.006),
    ).rgb * 2.0 - vec3<f32>(1.0);
    let ripple_b = textureSample(
        normal_texture,
        normal_sampler,
        position.xz * 0.075 - vec2<f32>(time * 0.014, -time * 0.009),
    ).rgb * 2.0 - vec3<f32>(1.0);
    let ripple = normalize(ripple_a + ripple_b);

    var surface_normal = world_normal;
    if (world_normal.y > 0.5) {
        surface_normal = normalize(vec3<f32>(ripple.x * 0.2, 1.0, ripple.y * 0.2));
    }

    let camera_distance = distance(position, params.camera_pos.xyz);
    let sun = normalize(params.sun_dir.xyz);
    let view_dir = normalize(params.camera_pos.xyz - position);

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.world_normal = surface_normal;
    pbr_input.N = surface_normal;
    pbr_input.material.base_color = vec4<f32>(tex * params.water_color.rgb * 2.6 + params.water_color.rgb * 0.12, 1.0);
    pbr_input.material.metallic = 0.0;
    pbr_input.material.perceptual_roughness = 0.22;
    pbr_input.material.reflectance = vec3<f32>(0.35);
    let lit = apply_pbr_lighting(pbr_input);

    let reflected = reflect(-view_dir, surface_normal);
    let sky_height = clamp(reflected.y, 0.0, 1.0);
    var sky_reflection = mix(params.fog_color.rgb, params.sky_zenith.rgb, pow(sky_height, 0.45));
    sky_reflection *= vec3<f32>(0.62, 0.80, 1.02);
    sky_reflection += params.sun_color.rgb * pow(max(dot(reflected, sun), 0.0), 220.0) * 0.8;

    let camera_underwater = params.underwater.x;
    let above_water = 1.0 - camera_underwater;
    let top_face = clamp(world_normal.y, 0.0, 1.0) * above_water;
    let plane_y = select(params.timing.y, params.underwater.y, camera_underwater > 0.5);
    let near_plane = 1.0 - step(0.55, abs(position.y - plane_y));
    let mirror_uv = in.position.xy / max(params.reflection.xy, vec2<f32>(1.0));
    let inside = all(mirror_uv >= vec2<f32>(0.0)) && all(mirror_uv <= vec2<f32>(1.0));
    let use_terrain = select(0.0, 1.0, inside) * top_face * near_plane * params.reflection.z;
    let wobble = vec2<f32>(ripple.x, ripple.y)
        * (0.006 + 0.018 * clamp(camera_distance / 48.0, 0.0, 1.0))
        * top_face;
    let reflected_uv = clamp(
        mirror_uv + wobble * vec2<f32>(0.85, 0.6),
        vec2<f32>(0.0),
        vec2<f32>(1.0),
    );
    let terrain_reflection = textureSample(
        reflection_texture,
        reflection_sampler,
        select(clamp(mirror_uv, vec2<f32>(0.0), vec2<f32>(1.0)), reflected_uv, inside),
    ).rgb;
    let reflection = mix(sky_reflection, terrain_reflection, 0.92 * use_terrain);

    let fresnel = pow(1.0 - max(dot(surface_normal, view_dir), 0.0), 3.0);
    var color = mix(lit.rgb, reflection, 0.22 + fresnel * 0.50);

    let underside = select(0.0, 1.0, world_normal.y > 0.0 && view_dir.y < 0.0);
    var alpha = mix(0.72, 0.88, fresnel);
    if (camera_underwater > 0.5 && underside > 0.5) {
        let look_up = clamp(-view_dir.y, 0.0, 1.0);
        let daylight = clamp(params.underwater.z, 0.0, 1.0);
        let window = mix(
            params.sky_zenith.rgb * vec3<f32>(0.85, 1.05, 0.95) * 1.1,
            params.sky_zenith.rgb * vec3<f32>(0.9, 1.15, 1.0) * 1.8
                + vec3<f32>(0.16, 0.36, 0.34) * daylight,
            pow(look_up, 1.2),
        );
        let depth_below = max(params.underwater.y - params.camera_pos.y, 0.0);
        let dim = 1.0 - 0.5 * smoothstep(4.0, 26.0, depth_below);
        var surface = mix(window, params.water_color.rgb * 2.0, 0.65) * dim;
        let wave = 0.5
            + 0.5 * sin(position.x * 1.7 + time * 0.9 + ripple.x * 6.0)
                * sin(position.z * 1.3 - time * 0.7 + ripple.y * 5.0);
        let shimmer = caustic(
            position.xz * 1.15 + vec2<f32>(time * 0.06, -time * 0.04),
            time * 1.2,
        );
        surface *= 0.82 + 0.30 * wave;
        surface += vec3<f32>(0.16, 0.38, 0.42) * shimmer * daylight * 1.35;
        let glint = pow(max(dot(-view_dir, sun), 0.0), 12.0) * 0.35 * daylight;
        surface += params.sun_color.rgb * glint;
        let grazing = pow(1.0 - abs(dot(surface_normal, view_dir)), 3.0);
        let veil = clamp(0.22 + 0.28 * grazing + 0.10 * (1.0 - look_up), 0.16, 0.76);
        return vec4<f32>(surface, veil);
    }
    alpha = mix(alpha, 1.0, smoothstep(24.0, 110.0, camera_distance));

    let out = main_pass_post_lighting_processing(pbr_input, vec4<f32>(color, alpha));
    return vec4<f32>(out.rgb, alpha);
}
