import bevy_pbr::render::{
    forward_io::VertexOutput,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings as view_bindings,
    shadows,
    view_transformations::position_world_to_view,
};

struct CausticParams {
    timing: vec4<f32>,
    surface: vec4<f32>,
};

@group(3) @binding(100) var<uniform> params: CausticParams;

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
                0.5 + 0.32 * sin(time * 0.8 + phase),
                0.5 + 0.32 * cos(time * 0.93 + phase * 1.37),
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
    return pow(1.0 - smoothstep(0.0, 0.11, edge), 3.0);
}

fn caustic_coords(world: vec3<f32>, normal: vec3<f32>) -> vec2<f32> {
    let n = abs(normal);
    if (n.y >= n.x && n.y >= n.z) {
        return world.xz;
    }
    if (n.x >= n.z) {
        return world.zy;
    }
    return world.xy;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let lit = apply_pbr_lighting(pbr_input);
    var color = lit.rgb;

    let water_coverage = clamp(in.color.a, 0.0, 1.0);
    if (water_coverage > 0.02) {
        let time = params.timing.x;
        let surface = normalize(in.world_normal.xyz);
        var sun_dir = vec3<f32>(0.0, 1.0, 0.0);
        var sun_shadow = 1.0;
        if (view_bindings::lights.n_directional_lights > 0u) {
            sun_dir = view_bindings::lights.directional_lights[0].direction_to_light.xyz;
            let view_z = position_world_to_view(in.world_position.xyz).z;
            sun_shadow = shadows::fetch_directional_shadow(
                0u,
                in.world_position,
                surface,
                view_z,
                in.position.xy,
            );
        }
        let facing_sun = mix(0.35, 1.0, smoothstep(0.0, 0.35, dot(surface, sun_dir)));
        let point = caustic_coords(in.world_position.xyz, surface) * params.timing.w;
        let layer_a = caustic(point + vec2<f32>(time * 0.035, time * 0.026), time * 0.9);
        let layer_b = caustic(
            point * 1.37 + vec2<f32>(11.0, 5.0) - vec2<f32>(time * 0.031, time * 0.042),
            time * 1.15,
        );
        let depth = water_coverage * 14.0;
        let fade = 1.0 - 0.6 * smoothstep(6.0, 30.0, depth);
        let daylight = clamp(params.surface.y, 0.0, 1.0);
        let pattern = (layer_a * 0.5 + layer_b * 0.32)
            * facing_sun
            * fade
            * daylight
            * sun_shadow;
        color += vec3<f32>(0.55, 0.92, 1.0) * pattern * params.timing.z;
    }

    let out = main_pass_post_lighting_processing(pbr_input, vec4<f32>(color, 1.0));
    return out;
}
