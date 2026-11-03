import bevy_pbr::render::{
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    mesh_functions,
    view_transformations::position_world_to_clip,
};

struct CloudParams {
    color: vec4<f32>,
    camera: vec4<f32>,
};

@group(3) @binding(0) var<uniform> params: CloudParams;

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
    out.color = decompressed.color;
    out.position = position_world_to_clip(out.world_position.xyz);

    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = 0;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let shade = 0.72 + 0.28 * max(normal.y, 0.0) - 0.12 * max(-normal.y, 0.0);
    let distance_to_camera = distance(in.world_position.xyz, params.camera.xyz);
    let fade = 1.0 - smoothstep(520.0, 820.0, distance_to_camera);
    return vec4<f32>(params.color.rgb * shade * in.color.rgb, params.color.a * fade);
}
