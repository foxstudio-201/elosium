use bevy::{light::GlobalAmbientLight, prelude::*};

use crate::{
    materials::{CloudMaterial, GameMaterials, SkyMaterial, WaterMaterial, WindMaterial},
    menu::Settings,
    reflection::ReflectionCamera,
};

pub const DAY_LENGTH: f32 = 480.0;

#[derive(Resource)]
pub struct DayCycle {
    pub time: f32,
}

#[derive(Component)]
pub struct SunLight;

pub fn configured_start_time() -> f32 {
    std::env::var("ELOSIUM_TIME")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .map(|value| value.rem_euclid(1.0))
        .unwrap_or(0.18)
}

pub fn advance_environment(
    time: Res<Time>,
    mut cycle: ResMut<DayCycle>,
    mut sun_query: Query<(&mut DirectionalLight, &mut Transform), With<SunLight>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut camera: Query<(&GlobalTransform, &mut DistanceFog), (With<Camera3d>, Without<ReflectionCamera>)>,
    mut mirror_fog: Query<&mut DistanceFog, With<ReflectionCamera>>,
    handles: Res<GameMaterials>,
    mut winds: ResMut<Assets<WindMaterial>>,
    mut waters: ResMut<Assets<WaterMaterial>>,
    mut skies: ResMut<Assets<SkyMaterial>>,
    mut cloud_materials: ResMut<Assets<CloudMaterial>>,
    settings: Res<Settings>,
    environment: Res<crate::environment::EnvironmentState>,
) {
    cycle.time = (cycle.time + time.delta_secs() / DAY_LENGTH) % 1.0;
    let seconds = time.elapsed_secs();
    let angle = cycle.time * std::f32::consts::TAU;
    let sun = Vec3::new(angle.cos(), angle.sin(), 0.32).normalize();

    let day = smoothstep(0.0, 0.25, sun.y);
    let night = smoothstep(0.0, 0.22, -sun.y);
    let twilight = (1.0 - day - night).clamp(0.0, 1.0);
    let total = (day + twilight + night).max(0.0001);

    let blend = |day_color: Vec3, twilight_color: Vec3, night_color: Vec3| {
        (day_color * day + twilight_color * twilight + night_color * night) / total
    };

    let zenith = blend(
        Vec3::new(0.12, 0.28, 0.62),
        Vec3::new(0.10, 0.09, 0.24),
        Vec3::new(0.006, 0.010, 0.030),
    );
    let horizon = blend(
        Vec3::new(0.55, 0.72, 0.95),
        Vec3::new(0.95, 0.36, 0.13),
        Vec3::new(0.020, 0.035, 0.080),
    );
    let sky_sun = blend(
        Vec3::new(1.0, 0.95, 0.85),
        Vec3::new(1.0, 0.42, 0.15),
        Vec3::new(0.65, 0.72, 0.95),
    );

    let moon = sun.y < -0.04;
    let light_direction = if moon { -sun } else { sun };
    let light_color = if moon {
        Vec3::new(0.55, 0.68, 1.0)
    } else {
        Vec3::new(1.0, 0.55, 0.24).lerp(Vec3::new(1.0, 0.97, 0.90), smoothstep(0.0, 0.35, sun.y))
    };
    let illuminance = if moon {
        320.0 * smoothstep(0.0, 0.30, -sun.y)
    } else {
        7_500.0 * smoothstep(-0.02, 0.18, sun.y) + 700.0 * twilight
    };

    for (mut light, mut transform) in sun_query.iter_mut() {
        light.color = Color::srgb(light_color.x, light_color.y, light_color.z);
        light.illuminance = illuminance;
        light.shadow_maps_enabled = settings.shadows;
        transform.rotation = Transform::from_translation(light_direction * 120.0)
            .looking_at(Vec3::ZERO, Vec3::Y)
            .rotation;
    }

    let ambient_color = blend(
        Vec3::new(0.78, 0.86, 1.0),
        Vec3::new(0.95, 0.60, 0.35),
        Vec3::new(0.22, 0.30, 0.55),
    );
    ambient.color = Color::srgb(ambient_color.x, ambient_color.y, ambient_color.z);
    ambient.brightness = 420.0 * day + 380.0 * twilight + 70.0 * night;

    let Ok((camera_transform, mut fog)) = camera.single_mut() else {
        return;
    };
    let camera_position = camera_transform.translation();
    fog.color = Color::linear_rgb(horizon.x, horizon.y, horizon.z);
    fog.directional_light_color = Color::linear_rgb(
        light_color.x * 0.45 + horizon.x * 0.55,
        light_color.y * 0.45 + horizon.y * 0.55,
        light_color.z * 0.45 + horizon.z * 0.55,
    );
    fog.directional_light_exponent = 6.0;
    for mut mirrored in mirror_fog.iter_mut() {
        mirrored.color = fog.color;
        mirrored.directional_light_color = fog.directional_light_color;
        mirrored.directional_light_exponent = fog.directional_light_exponent;
    }

    let water_light = light_color * (0.14 + 1.05 * day + 0.55 * twilight + 0.12 * night);
    for handle in [&handles.wind, &handles.flora, &handles.leaves] {
        if let Some(mut material) = winds.get_mut(handle) {
            material.extension.settings.timing = Vec4::new(seconds, 0.06, 0.0, 0.0);
        }
    }
    if let Some(mut material) = waters.get_mut(&handles.water) {
        material.extension.params.sun_dir = light_direction.extend(0.16 + 0.22 * day);
        material.extension.params.sun_color = water_light.extend(0.0);
        material.extension.params.camera_pos = camera_position.extend(0.0);
        material.extension.params.fog_color = horizon.extend(0.0);
        material.extension.params.sky_zenith = zenith.extend(0.0);
        material.extension.params.timing.x = seconds;
        let pitch = (camera_transform.rotation() * Vec3::NEG_Z).y.asin();
        let horizon_uv = 0.5 + 0.5 * pitch.tan() / 0.7265;
        material.extension.params.timing.z = horizon_uv.clamp(0.02, 0.98);
    }
    let cloud_day = Vec3::new(1.0, 1.0, 1.02);
    let cloud_twilight = Vec3::new(1.0, 0.72, 0.52);
    let cloud_night = Vec3::new(0.14, 0.17, 0.26);
    let cloud_color = blend(cloud_day, cloud_twilight, cloud_night);
    let cloud_alphas = if environment.underwater {
        [0.0f32, 0.0, 0.0]
    } else {
        [0.85f32, 0.62, 0.45]
    };
    for (index, handle) in handles.cloud.iter().enumerate() {
        if let Some(mut material) = cloud_materials.get_mut(handle) {
            material.params.color =
                Vec4::new(cloud_color.x, cloud_color.y, cloud_color.z, cloud_alphas[index]);
        }
    }
    if let Some(mut material) = skies.get_mut(&handles.sky) {
        material.params.sun_dir = sun.extend(0.0);
        material.params.sun_color = sky_sun.extend(0.0);
        material.params.camera_pos = camera_position.extend(0.0);
        material.params.zenith = zenith.extend(0.0);
        material.params.horizon = horizon.extend(0.0);
        material.params.timing = Vec4::new(seconds, night, twilight, day);
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
