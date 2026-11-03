struct UnderwaterFx {
    params: vec4<f32>,
    sun: vec4<f32>,
    water: vec4<f32>,
    surface: vec4<f32>,
    sun_dir: vec4<f32>,
    camera: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> fx: UnderwaterFx;

fn sample_view(uv: vec2<f32>) -> vec3<f32> {
    let clamped = clamp(uv, vec2<f32>(0.0015), vec2<f32>(0.9985));
    return textureSample(source_texture, source_sampler, clamped).rgb;
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let time = fx.params.x;
    let distortion = fx.params.y;
    let wave = vec2<f32>(
        sin(uv.y * 12.0 + time * 0.9) + 0.5 * sin(uv.y * 25.0 - time * 0.7),
        sin(uv.x * 9.0 - time * 0.8) + 0.5 * sin(uv.x * 21.0 + time * 0.6),
    );
    var color = sample_view(uv + wave * distortion);

    let daylight = clamp(fx.water.w, 0.0, 1.0);
    let tint = fx.water.rgb * 0.4 + vec3<f32>(0.03, 0.14, 0.20);
    color = mix(color, color * vec3<f32>(0.64, 0.88, 1.12), 0.45);
    color = mix(color, tint * 1.2, (1.0 - daylight) * 0.35);

    let centered = uv * 2.0 - vec2<f32>(1.0);
    let vignette = 1.0 - fx.params.w * dot(centered, centered);
    color = color * max(vignette, 0.0);

    return vec4<f32>(color, 1.0);
}
