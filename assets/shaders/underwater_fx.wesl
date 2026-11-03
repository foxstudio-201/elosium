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
    sun_screen: vec4<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> fx: UnderwaterFx;

const SHAFT_SAMPLES: i32 = 24;

fn sample_view(uv: vec2<f32>) -> vec3<f32> {
    let clamped = clamp(uv, vec2<f32>(0.0015), vec2<f32>(0.9985));
    return textureSample(source_texture, source_sampler, clamped).rgb;
}

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

fn light_shafts(uv: vec2<f32>, threshold: f32, strength: f32) -> vec3<f32> {
    if strength <= 0.0 {
        return vec3<f32>(0.0);
    }
    let delta = uv - fx.sun_screen.xy;
    let step = delta / f32(SHAFT_SAMPLES);
    var sample_uv = uv;
    var accumulation = vec3<f32>(0.0);
    var decay = 1.0;
    for (var i = 0; i < SHAFT_SAMPLES; i = i + 1) {
        sample_uv -= step;
        let sample = sample_view(sample_uv);
        let bright = max(luminance(sample) - threshold, 0.0);
        let glow = bright * bright;
        accumulation += sample * glow * decay;
        decay *= 0.985;
    }
    let falloff = exp(-length(delta) * 1.0);
    return accumulation * (strength * 2.4 / f32(SHAFT_SAMPLES)) * falloff;
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let time = fx.params.x;
    let distortion = fx.params.y;
    let underwater = fx.surface.z;
    let wave = vec2<f32>(
        sin(uv.y * 12.0 + time * 0.9) + 0.5 * sin(uv.y * 25.0 - time * 0.7),
        sin(uv.x * 9.0 - time * 0.8) + 0.5 * sin(uv.x * 21.0 + time * 0.6),
    );
    var color = sample_view(uv + wave * distortion * underwater);

    if underwater > 0.5 {
        color = mix(color, color * vec3<f32>(0.78, 0.96, 1.15), 0.55);
        let centered = uv * 2.0 - vec2<f32>(1.0);
        let vignette = 1.0 - fx.params.w * dot(centered, centered);
        color = color * max(vignette, 0.0);
    }

    if fx.sun_screen.z > 0.5 {
        color += light_shafts(uv, fx.sun.y, fx.sun_screen.w);
    }

    return vec4<f32>(color, 1.0);
}
