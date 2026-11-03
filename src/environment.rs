use bevy::{
    light::{FogVolume, VolumetricFog},
    prelude::*,
};

use crate::{
    daynight::SunLight,
    materials::{BlockMaterial, FOG_DENSITY, GameMaterials, WaterMaterial},
    menu::Settings,
    reflection::ReflectionCamera,
    world::VoxelWorld,
};

const WATER_FOG_COLOR: Color = Color::srgb(0.03, 0.18, 0.32);
const WATER_FOG_DISTANCE: f32 = 96.0;
const AIR_VOLUME: Color = Color::srgb(0.78, 0.85, 0.96);

#[derive(Component)]
pub struct GlobalFogVolume;

#[derive(Resource)]
pub struct EnvironmentState {
    pub underwater: bool,
    pub water_surface: f32,
    water_vision_time: f32,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            underwater: false,
            water_surface: 0.0,
            water_vision_time: if std::env::var("ELOSIUM_WATER_VISION").is_ok() {
                600.0
            } else {
                0.0
            },
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
    settings: Res<Settings>,
    mut clear: ResMut<ClearColor>,
    mut state: ResMut<EnvironmentState>,
) {
    let Ok((transform, mut fog, volumetric)) = camera.single_mut() else {
        return;
    };
    let eye = transform.translation;
    let underwater = world
        .get_block(
            eye.x.floor() as i32,
            eye.y.floor() as i32,
            eye.z.floor() as i32,
        )
        .is_water();
    state.underwater = underwater;

    let delta = time.delta_secs();
    if underwater {
        state.water_vision_time = (state.water_vision_time + delta * 20.0).min(600.0);
    } else {
        state.water_vision_time = (state.water_vision_time - delta * 200.0).max(0.0);
    }

    let depth = if underwater {
        water_depth(&world, eye)
    } else {
        0.0
    };
    state.water_surface = if underwater { eye.y + depth } else { 0.0 };
    let underwater_flag = if underwater { 1.0 } else { 0.0 };
    if let Some(mut water) = waters.get_mut(&handles.water) {
        water.extension.params.underwater = Vec4::new(
            underwater_flag,
            if underwater { eye.y + depth } else { 0.0 },
            (0.18 + 0.82 * sun_daylight(&sun)) * underwater_flag,
            0.0,
        );
        if underwater {
            water.extension.params.fog_color = Vec4::new(0.02, 0.16, 0.28, 0.0);
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

    if let Ok((mut fog_volume, mut fog_transform)) = volume.single_mut() {
        if underwater {
            fog_transform.translation = eye;
            fog_transform.scale = Vec3::new(420.0, 320.0, 420.0);
            fog_volume.fog_color = Color::srgb(0.10, 0.32, 0.48);
            fog_volume.density_factor = 0.012;
            fog_volume.absorption = 0.0015;
            fog_volume.scattering = 0.35;
            fog_volume.scattering_asymmetry = 0.58;
            fog_volume.light_intensity = 2.2;
        } else {
            let godray = settings.godray_intensity.clamp(0.2, 3.0);
            fog_transform.translation = eye;
            fog_transform.scale = Vec3::new(420.0, 320.0, 420.0);
            fog_volume.fog_color = AIR_VOLUME;
            fog_volume.density_factor = 0.0025 * godray;
            fog_volume.absorption = 0.15;
            fog_volume.scattering = (0.35 * godray).min(0.85);
            fog_volume.scattering_asymmetry = 0.55;
            fog_volume.light_intensity = 1.0;
        }
    }

    if let Some(mut volumetric) = volumetric {
        if underwater {
            volumetric.ambient_color = Color::linear_rgb(0.05, 0.16, 0.26);
            volumetric.ambient_intensity = 0.7;
        } else {
            volumetric.ambient_color = Color::WHITE;
            volumetric.ambient_intensity = 0.0;
        }
    }

    if underwater {
        let vision = water_vision(state.water_vision_time);
        fog.color = WATER_FOG_COLOR;
        fog.directional_light_color = WATER_FOG_COLOR;
        fog.directional_light_exponent = 6.0;
        fog.falloff = FogFalloff::Linear {
            start: 0.0,
            end: WATER_FOG_DISTANCE * vision.max(0.25) * settings.fog_radius.max(0.1),
        };
        *clear = ClearColor(WATER_FOG_COLOR);
    } else {
        fog.falloff = FogFalloff::ExponentialSquared {
            density: FOG_DENSITY / settings.fog_radius.max(0.1),
        };
        fog.directional_light_exponent = 6.0;
        *clear = ClearColor(Color::srgb(0.55, 0.72, 0.95));
    }

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
    let mut top = None;
    while world.get_block(x, y, z).is_water() {
        top = Some(y);
        y += 1;
    }
    let Some(top) = top else {
        return 0.0;
    };
    let level = world.water_level(x, top, z).unwrap_or(0) as f32;
    let surface = top as f32 + 0.875 - level * 0.1;
    (surface - eye.y).max(0.0)
}

fn water_vision(time: f32) -> f32 {
    if time >= 600.0 {
        return 1.0;
    }
    let rise = (time / 100.0).clamp(0.0, 1.0);
    let settle = if time < 100.0 {
        0.0
    } else {
        ((time - 100.0) / 500.0).clamp(0.0, 1.0)
    };
    rise * 0.6 + settle * 0.399_999_98
}
