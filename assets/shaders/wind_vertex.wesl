import bevy_pbr::render::{
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    mesh_functions,
    view_transformations::position_world_to_clip,
};

struct WindSettings {
    timing: vec4<f32>,
    player: vec4<f32>,
    bend: vec4<f32>,
};

@group(3) @binding(100) var<uniform> wind: WindSettings;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let decompressed = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(decompressed.position, 1.0),
    );
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        decompressed.normal,
        vertex_in.instance_index,
    );
    out.uv = decompressed.uv;

    let weight = decompressed.color.a;
    out.color = vec4<f32>(decompressed.color.rgb, 1.0);

    let world = out.world_position.xyz;
    let phase = wind.timing.x * 1.5 + world.x * 0.35 + world.z * 0.27;
    let sway = sin(phase) * 0.65 + sin(phase * 2.1 + world.y * 0.9) * 0.35;
    let amplitude = wind.timing.y * weight;
    out.world_position.x += sway * amplitude;
    out.world_position.z += sway * amplitude * 0.6;

    let root = world;
    let offset = root.xz - wind.player.xz;
    let distance = length(offset);
    let body = max(wind.bend.y, 0.001);
    let reach = max(wind.bend.z, 0.001);
    let horizontal = 1.0 - smoothstep(body, body + reach, distance);
    let vertical = 1.0 - smoothstep(1.8, 3.0, abs(root.y - wind.player.y));
    let influence = horizontal * vertical;
    let push = influence * wind.bend.x * weight;
    if (push > 0.0001) {
        let direction = select(
            vec2<f32>(0.0, 1.0),
            offset / max(distance, 0.001),
            distance > 0.001,
        );
        let spread = smoothstep(0.0, body, distance);
        out.world_position.x += direction.x * push * spread;
        out.world_position.z += direction.y * push * spread;
        out.world_position.y -= push * 0.75;
    }

    out.position = position_world_to_clip(out.world_position.xyz);

    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = 0;

    return out;
}
