use bevy::{
    core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType, RenderApp},
    shader::ShaderRef,
};

use crate::{
    daynight::SunLight,
    environment::{update_environment_effects, EnvironmentState},
    player::Player,
};

#[derive(Component, ExtractComponent, Clone, Copy, Default, ShaderType)]
#[extract_app(RenderApp)]
pub struct UnderwaterFx {
    pub params: Vec4,
    pub sun: Vec4,
    pub water: Vec4,
    pub surface: Vec4,
    pub sun_dir: Vec4,
    pub camera: Vec4,
    pub forward: Vec4,
    pub right: Vec4,
    pub up: Vec4,
}

impl FullscreenMaterial for UnderwaterFx {
    fn fragment_shader() -> ShaderRef {
        "shaders/underwater_fx.wesl".into()
    }
}

pub struct WaterFxPlugin;

impl Plugin for WaterFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FullscreenMaterialPlugin::<UnderwaterFx>::default())
            .add_systems(
                Update,
                sync_underwater_fx.after(update_environment_effects),
            );
    }
}

fn sync_underwater_fx(
    mut commands: Commands,
    state: Res<EnvironmentState>,
    time: Res<Time>,
    sun: Query<(&GlobalTransform, &DirectionalLight), With<SunLight>>,
    mut cameras: Query<
        (
            Entity,
            &GlobalTransform,
            &Camera,
            &Projection,
            Option<&mut UnderwaterFx>,
        ),
        (With<Player>, With<Camera3d>),
    >,
) {
    let Ok((entity, camera_transform, camera, projection, fx)) = cameras.single_mut() else {
        return;
    };
    let mut daylight = 0.0;
    let mut to_sun = Vec3::Y;
    if let Ok((sun_transform, light)) = sun.single() {
        daylight = (light.illuminance / 7_500.0).clamp(0.0, 1.0);
        to_sun = (sun_transform.rotation() * Vec3::Z).normalize();
    }
    let (tan_y, tan_x) = match projection {
        Projection::Perspective(perspective) => {
            let tan_y = (perspective.fov * 0.5).tan();
            let aspect = camera
                .logical_viewport_size()
                .map(|size| size.x / size.y.max(1.0))
                .unwrap_or(1.0);
            (tan_y, tan_y * aspect)
        }
        _ => (0.5, 0.5),
    };
    let translation = camera_transform.translation();
    let rotation = camera_transform.rotation();
    let forward = rotation * Vec3::NEG_Z;
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let underwater = state.underwater;
    let value = UnderwaterFx {
        params: Vec4::new(time.elapsed_secs(), 0.0008, 0.55, 0.12),
        sun: Vec4::new(daylight, 0.0, 0.0, 0.0),
        water: Vec4::new(0.18, 0.48, 0.70, daylight),
        surface: Vec4::new(
            state.water_surface,
            (state.water_surface - translation.y).max(0.0),
            if underwater { 1.0 } else { 0.0 },
            0.0,
        ),
        sun_dir: Vec4::new(to_sun.x, to_sun.y, to_sun.z, 0.0),
        camera: Vec4::new(translation.x, translation.y, translation.z, tan_y),
        forward: Vec4::new(forward.x, forward.y, forward.z, tan_x),
        right: Vec4::new(right.x, right.y, right.z, 0.0),
        up: Vec4::new(up.x, up.y, up.z, 0.0),
    };
    if let Some(mut current) = fx {
        *current = value;
    } else {
        commands.entity(entity).insert(value);
    }
}
