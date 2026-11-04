use bevy::{
    light::{FogVolume, VolumetricFog},
    prelude::*,
};

use crate::{
    daynight::SunLight,
    materials::{
        BlockMaterial, CloudMaterial, FOG_DENSITY, GameMaterials, SkyMaterial, WaterMaterial,
    },
    menu::Settings,
    reflection::ReflectionCamera,
    world::VoxelWorld,
};

const AIR_VOLUME: Color = Color::srgb(0.78, 0.85, 0.96);

#[derive(Component)]
pub struct GlobalFogVolume;

#[derive(Resource)]
pub struct EnvironmentState {
    pub underwater: bool,
    pub water_surface: f32,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            underwater: false,
            water_surface: 0.0,
        }
    }
}

pub fn update_environment_effects(
    time: Res<Time>,
    world: Res<VoxelWorld>,
    mut camera: Query<
        (&Transform, &mut DistanceFog, Option<&mut VolumetricFog>),
        (With<Camera3d>, Without<ReflectionCamera>),
    >,
    mut mirror_fog: Query<&mut DistanceFog, With<ReflectionCamera>>,
    sun: Query<&DirectionalLight, With<SunLight>>,
    mut volume: Query<(&mut FogVolume, &mut Transform), (With<GlobalFogVolume>, Without<Camera3d>)>,
    handles: Res<GameMaterials>,
    mut waters: ResMut<Assets<WaterMaterial>>,
    mut blocks: ResMut<Assets<BlockMaterial>>,
    mut skies: ResMut<Assets<SkyMaterial>>,
    mut clouds: ResMut<Assets<CloudMaterial>>,
    settings: Res<Settings>,
    mut clear: ResMut<ClearColor>,
    mut state: ResMut<EnvironmentState>,
    mut out_of_water: Local<f32>,
) {
    let Ok((transform, mut fog, volumetric)) = camera.single_mut() else {
        return;
    };
    let eye = transform.translation;
    let depth = water_depth(&world, eye);
    let underwater = if depth > 0.015 {
        *out_of_water = 0.0;
        true
    } else if state.underwater {
        *out_of_water += time.delta_secs();
        *out_of_water <= 0.35
    } else {
        false
    };
    state.underwater = underwater;
    state.water_surface = if underwater { eye.y + depth } else { 0.0 };
    let underwater_flag = if underwater { 1.0 } else { 0.0 };
    let day = sun_daylight(&sun);
    let moon = (1.0 - day).clamp(0.0, 1.0);
    let godray = settings.godray_intensity.clamp(0.2, 3.0);
    let depth_below = (state.water_surface - eye.y).max(0.0);
    let deep = (depth_below / 28.0).clamp(0.0, 1.0);
    let light = 0.12 + 0.88 * day;
    let shallow = Vec3::new(0.10, 0.36, 0.44);
    let deep_color = Vec3::new(0.02, 0.09, 0.17);
    let tint = shallow.lerp(deep_color, deep) * light;
    let water_fog = Color::srgb(tint.x, tint.y, tint.z);

    if let Some(mut water) = waters.get_mut(&handles.water) {
        water.extension.params.underwater = Vec4::new(
            underwater_flag,
            if underwater { eye.y + depth } else { 0.0 },
            (0.18 + 0.82 * sun_daylight(&sun)) * underwater_flag,
            0.0,
        );
        if underwater {
            water.extension.params.fog_color = Vec4::new(tint.x, tint.y, tint.z, 0.0);
        }
    }
    if let Some(mut block) = blocks.get_mut(&handles.block) {
        block.extension.params.timing = Vec4::new(time.elapsed_secs(), underwater_flag, 0.9, 1.3);
        block.extension.params.surface = Vec4::new(
            if underwater { eye.y + depth } else { 0.0 },
            sun_daylight(&sun),
            0.0,
            0.0,
        );
    }
    if let Some(mut sky) = skies.get_mut(&handles.sky) {
        sky.params.water = if underwater {
            Vec4::new(1.0, tint.x, tint.y, tint.z)
        } else {
            Vec4::ZERO
        };
    }
    if underwater {
        for (index, handle) in handles.cloud.iter().enumerate() {
            if let Some(mut cloud) = clouds.get_mut(handle) {
                let color = cloud.params.color;
                cloud.params.color = Vec4::new(
                    color.x * 0.55 + tint.x * 0.45,
                    color.y * 0.55 + tint.y * 0.45,
                    color.z * 0.55 + tint.z * 0.45,
                    if index == 0 { 0.96 } else { 0.0 },
                );
                cloud.params.camera.w = 1.0;
            }
        }
    }

    if let Ok((mut fog_volume, mut fog_transform)) = volume.single_mut() {
        fog_transform.translation = eye;
        fog_transform.scale = Vec3::new(220.0, 180.0, 220.0);
        fog_volume.fog_color = if underwater { water_fog } else { AIR_VOLUME };
        fog_volume.density_factor = 0.0045;
        fog_volume.absorption = 0.02;
        fog_volume.scattering = 0.70;
        fog_volume.scattering_asymmetry = 0.40;
        fog_volume.light_intensity = (1.2 + 0.4 * moon) * godray.powf(0.7);
    }

    if let Some(mut volumetric) = volumetric {
        if underwater {
            volumetric.ambient_color = Color::linear_rgb(0.05, 0.16, 0.26);
            volumetric.ambient_intensity = 0.08 + 0.32 * day;
        } else {
            volumetric.ambient_color = Color::WHITE;
            volumetric.ambient_intensity = 0.0;
        }
    }

    if underwater {
        fog.color = water_fog;
        fog.directional_light_color = water_fog;
    }
    fog.directional_light_exponent = 6.0;
    fog.falloff = FogFalloff::ExponentialSquared {
        density: FOG_DENSITY
            * if underwater { 4.0 } else { 1.0 }
            / settings.fog_radius.max(0.1),
    };
    *clear = ClearColor(if underwater {
        water_fog
    } else {
        Color::srgb(0.55, 0.72, 0.95)
    });

    for mut mirrored in mirror_fog.iter_mut() {
        mirrored.falloff = fog.falloff.clone();
        mirrored.color = fog.color;
        mirrored.directional_light_color = fog.directional_light_color;
        mirrored.directional_light_exponent = fog.directional_light_exponent;
    }
}

fn sun_daylight(sun: &Query<&DirectionalLight, With<SunLight>>) -> f32 {
    sun.single()
        .map(|light| (light.illuminance / 7_500.0).clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

fn water_depth(world: &VoxelWorld, eye: Vec3) -> f32 {
    let x = eye.x.floor() as i32;
    let z = eye.z.floor() as i32;
    let mut y = eye.y.floor() as i32;
    let mut surface = None;
    loop {
        let block = world.get_block(x, y, z);
        if !block.is_water() && !block.is_water_plant() {
            break;
        }
        if block.is_water() {
            let level = world.water_level(x, y, z).unwrap_or(0) as f32;
            surface = Some(y as f32 + 0.875 - level * 0.1);
        }
        y += 1;
    }
    surface.map(|value| (value - eye.y).max(0.0)).unwrap_or(0.0)
}
