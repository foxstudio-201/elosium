use bevy::{
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    math::reflection_matrix,
    prelude::*,
    render::render_resource::TextureFormat,
    render::view::DebandDither,
};

use crate::{
    materials::{GameMaterials, WaterMaterial, FOG_DENSITY},
    menu::{Settings, REFLECTION_SCALES},
    world::{VoxelWorld, WATER_SURFACE_HEIGHT},
};

pub const WATER_PLANE_Y: f32 = WATER_SURFACE_HEIGHT;
const TARGET_SCALE: f32 = 0.5;

#[derive(Component)]
pub struct ReflectionCamera;

#[derive(Resource)]
pub struct ReflectionTarget {
    pub image: Handle<Image>,
    pub screen_size: Vec2,
    pub scale: f32,
    plane_y: f32,
}

pub fn create_image(images: &mut Assets<Image>, window: &Window) -> (Handle<Image>, Vec2) {
    let screen = screen_size(window);
    let size = target_size(screen, TARGET_SCALE);
    let image = images.add(Image::new_target_texture(
        size.x,
        size.y,
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    (image, screen)
}

pub fn spawn_camera(commands: &mut Commands, image: Handle<Image>, screen_size: Vec2) {
    commands.insert_resource(ReflectionTarget {
        image: image.clone(),
        screen_size,
        scale: TARGET_SCALE,
        plane_y: WATER_PLANE_Y,
    });
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: -1,
            invert_culling: true,
            ..default()
        },
        Msaa::Sample4,
        Tonemapping::None,
        DebandDither::Disabled,
        DistanceFog {
            color: Color::srgb(0.72, 0.82, 0.95),
            falloff: FogFalloff::ExponentialSquared {
                density: FOG_DENSITY,
            },
            ..default()
        },
        RenderTarget::Image(image.into()),
        Transform::default(),
        Projection::Perspective(PerspectiveProjection::default()),
        ReflectionCamera,
    ));
}

pub fn sync_reflection(
    window: Single<&Window>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    handles: Res<GameMaterials>,
    mut target: ResMut<ReflectionTarget>,
    world: Res<VoxelWorld>,
    settings: Res<Settings>,
    main: Query<(&Transform, &Projection), (With<Camera3d>, Without<ReflectionCamera>)>,
    mut mirror: Query<
        (&mut Transform, &mut Projection, &mut Camera, &mut RenderTarget),
        With<ReflectionCamera>,
    >,
) {
    let screen = screen_size(&window);
    let scale = REFLECTION_SCALES[settings.reflection_resolution.min(REFLECTION_SCALES.len() - 1)];
    if screen != target.screen_size || scale != target.scale {
        target.screen_size = screen;
        target.scale = scale;
        let size = target_size(screen, scale);
        let image = images.add(Image::new_target_texture(
            size.x,
            size.y,
            TextureFormat::Bgra8UnormSrgb,
            None,
        ));
        let previous = std::mem::replace(&mut target.image, image.clone());
        images.remove(&previous);
        if let Ok((_, _, _, mut render_target)) = mirror.single_mut() {
            *render_target = RenderTarget::Image(image.clone().into());
        }
        if let Some(mut material) = materials.get_mut(&handles.water) {
            material.extension.reflection = image;
        }
    }

    let Ok((main_transform, Projection::Perspective(main_projection))) = main.single() else {
        return;
    };
    let Ok((mut mirror_transform, mut mirror_projection, mut mirror_camera, _)) =
        mirror.single_mut()
    else {
        return;
    };

    let plane_y = water_plane_near(&world, main_transform.translation)
        .unwrap_or(target.plane_y);
    target.plane_y = plane_y;

    mirror_camera.is_active = settings.reflections && main_transform.translation.y > plane_y + 0.05;

    let plane = Transform::from_translation(Vec3::new(0.0, plane_y, 0.0));
    let (transform, projection) =
        calculate_mirror_camera(main_transform, main_projection, &plane);
    if let Some(mut material) = materials.get_mut(&handles.water) {
        material.extension.params.timing.y = plane_y;
        material.extension.params.reflection = Vec4::new(
            screen.x,
            screen.y,
            if mirror_camera.is_active { 1.0 } else { 0.0 },
            0.0,
        );
    }
    *mirror_transform = transform;
    *mirror_projection = Projection::Perspective(projection);
}

fn water_plane_near(world: &VoxelWorld, eye: Vec3) -> Option<f32> {
    let cx = eye.x.floor() as i32;
    let cz = eye.z.floor() as i32;
    const RADIUS: i32 = 32;
    for radius in 0..=RADIUS {
        let mut found: Option<f32> = None;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx.abs() != radius && dz.abs() != radius {
                    continue;
                }
                let Some(column) = world.column_at(cx + dx, cz + dz) else {
                    continue;
                };
                if column.height >= column.water_top {
                    continue;
                }
                let top = column.water_top;
                let level = world.water_level(cx + dx, top, cz + dz).unwrap_or(0);
                let plane = top as f32 + 0.875 - level as f32 * 0.1;
                if plane >= eye.y - 0.05 {
                    continue;
                }
                found = Some(found.map_or(plane, |best: f32| best.max(plane)));
            }
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

fn screen_size(window: &Window) -> Vec2 {
    Vec2::new(
        window.physical_width().max(1) as f32,
        window.physical_height().max(1) as f32,
    )
}

fn target_size(screen: Vec2, scale: f32) -> UVec2 {
    UVec2::new(
        ((screen.x * scale) as u32).max(1),
        ((screen.y * scale) as u32).max(1),
    )
}

fn calculate_mirror_camera(
    main_transform: &Transform,
    main_projection: &PerspectiveProjection,
    plane: &Transform,
) -> (Transform, PerspectiveProjection) {
    let reflected = reflection_matrix(Vec3::Y);
    let reflection = Mat3::from_cols(
        reflected.x_axis.into(),
        reflected.y_axis.into(),
        reflected.z_axis.into(),
    );
    let mirror_scale = Vec3::new(1.0, -1.0, 1.0);
    let rotation = Quat::from_mat3(
        &(reflection
            * Mat3::from_quat(main_transform.rotation)
            * Mat3::from_diagonal(mirror_scale)),
    );
    let translation = Vec3::new(
        main_transform.translation.x,
        plane.translation.y * 2.0 - main_transform.translation.y,
        main_transform.translation.z,
    );
    let transform = Transform {
        translation,
        rotation,
        scale: mirror_scale,
    };

    let view_from_world = transform.compute_affine().matrix3.inverse();
    let keep_side = (view_from_world * (plane.rotation * Vec3::Y)).normalize();
    let point = view_from_world * (plane.translation - transform.translation);
    let distance = -keep_side.dot(point);

    let projection = PerspectiveProjection {
        near_clip_plane: keep_side.extend(distance),
        ..*main_projection
    };
    (transform, projection)
}
