mod blocks;
mod chat;
mod clouds;
mod daynight;
mod debug;
mod debug_boxes;
mod environment;
mod frontend;
mod interaction;
mod materials;
mod mc_worldgen;
mod menu;
mod mesher;
mod outline;
mod pack_atlas;
mod particles;
mod player;
mod player_model;
mod reflection;
mod storage;
mod streaming;
mod terrain;
mod vanilla_worldgen;
mod water;
mod water_fx;
mod world;
mod worldgen;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::{
    app::AppExit,
    camera::visibility::{CascadesVisibleEntities, RenderLayers},
    image::ImagePlugin,
    input::keyboard::KeyCode,
    light::{CascadeShadowConfigBuilder, FogVolume, VolumetricFog, VolumetricLight},
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
    window::{Window, WindowPlugin},
};

use daynight::{advance_environment, configured_start_time, DayCycle, SunLight};
use debug::{spawn_debug_overlay, toggle_debug, update_debug};
use debug_boxes::{spawn_debug_boxes, toggle_debug_boxes, update_debug_boxes, DebugBoxes};
use environment::{update_environment_effects, EnvironmentState, GlobalFogVolume};
use interaction::{
    break_block, follow_camera, place_block, spawn_crosshair,
    BreakTimer,
};
use materials::{create_materials, FOG_DENSITY};
use mesher::sky_cube_mesh;
use outline::{spawn_block_outline, update_block_outline};
use player::{
    advance_swing, apply_camera_view, enforce_mode2_cursor, grab_cursor,
    orbit_drag_control, player_controller, toggle_view, update_body_yaw, update_head_aim, Player,
    PlayerPose, SpawnPoint, ViewMode, EYE_HEIGHT,
};
use player_model::{
    animate_first_person_arm, animate_player_model, clear_clones_off_world,
    push_player_from_clones, spawn_clone_players, spawn_player_model, tick_clones,
    update_name_tags, SpawnClone,
};
use reflection::{create_image, spawn_camera, sync_reflection, ReflectionCamera};
use streaming::{stream_chunks, update_occluders, ChunkRenderer, LoadWarmup, RenderSettings};
use water::{tick_water, WaterFlowTimer};
use world::{ChunkPos, VoxelWorld};

#[derive(Component)]
struct Sky;

#[derive(Resource)]
struct AutoCapture {
    timer: Timer,
    stage: u8,
}

fn main() {
    std::fs::create_dir_all("screenshots").ok();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Elosium".into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins((
        MaterialPlugin::<materials::BlockMaterial>::default(),
        MaterialPlugin::<materials::WindMaterial>::default(),
        MaterialPlugin::<materials::WaterMaterial>::default(),
        MaterialPlugin::<materials::SkyMaterial>::default(),
        MaterialPlugin::<materials::CloudMaterial>::default(),
    ))
    .insert_resource(ClearColor(Color::srgb(0.55, 0.72, 0.95)))
    .insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.78, 0.86, 1.0),
        brightness: 420.0,
        ..default()
    })
    .insert_resource(DayCycle {
        time: configured_start_time(),
    })
    .insert_resource(EnvironmentState::default())
    .insert_resource(WaterFlowTimer::default())
    .insert_resource(BreakTimer::default())
    .insert_resource(debug::FpsCounter::default())
    .insert_resource(DebugBoxes::default())
    .insert_resource(debug::HudVisible(
        std::env::var("ELOSIUM_HUD_OFF").is_err()
            && std::env::var("ELOSIUM_BAKE_PANORAMA").is_err(),
    ))
    .insert_resource(debug::DebugVisible(
        std::env::var("ELOSIUM_DEBUG_OVERLAY").is_ok(),
    ))
    .insert_resource(RenderSettings {
        distance: if std::env::var("ELOSIUM_BAKE_PANORAMA").is_ok() {
            12
        } else {
            streaming::DEFAULT_RENDER_DISTANCE
        },
    })
    .insert_resource(LoadWarmup::default())
    .insert_resource(ViewMode {
        mode: if std::env::var("ELOSIUM_DUNGEONS").is_ok() {
            2
        } else if std::env::var("ELOSIUM_THIRD").is_ok() {
            1
        } else {
            0
        },
        offset: if std::env::var("ELOSIUM_THIRD").is_ok() {
            Vec3::new(0.0, 0.55, 2.8)
        } else {
            Vec3::ZERO
        },
        blend: if std::env::var("ELOSIUM_THIRD").is_ok() {
            1.0
        } else {
            0.0
        },
        distance: 0.0,
        orbit_yaw: 0.0,
        orbit_pitch: -0.95,
    })
    .add_message::<SpawnClone>()
    .add_plugins(menu::MenuPlugin)
    .add_plugins(chat::ChatPlugin)
    .add_plugins(water_fx::WaterFxPlugin)
    .add_plugins(frontend::FrontendPlugin)
    .add_plugins(particles::ParticlePlugin)
    .add_systems(Startup, (setup, frontend::spawn_screens).chain())
    .add_systems(
        Update,
        (
            (
                (
                    break_block.run_if(frontend::in_game),
                    update_block_outline,
                    place_block.run_if(frontend::in_game),
                    grab_cursor.run_if(frontend::in_game),
                    player_controller.run_if(frontend::in_game),
                    push_player_from_clones.run_if(frontend::in_game),
                    orbit_drag_control.run_if(frontend::in_game),
                    enforce_mode2_cursor.run_if(frontend::in_game),
                    update_head_aim,
                    update_body_yaw,
                    advance_swing,
                    toggle_view.run_if(frontend::in_game),
                    apply_camera_view,
                )
                    .chain(),
                (
                    update_occluders.run_if(frontend::streaming_active),
                    animate_player_model,
                    update_player_layers,
                    animate_first_person_arm,
                    spawn_clone_players,
                    tick_clones,
                    update_name_tags,
                    clear_clones_off_world,
                    update_plant_bend,
                    sync_reflection,
                    tick_water,
                    stream_chunks.run_if(frontend::streaming_active),
                    clouds::move_clouds,
                    advance_environment,
                    update_environment_effects,
                    follow_camera,
                    sky_follows_camera,
                )
                    .chain(),
            )
                .chain(),
            manual_screenshot,
            auto_screenshot.run_if(resource_exists::<AutoCapture>),
            debug::toggle_hud,
            debug::sync_hud_visibility,
            toggle_debug,
            update_debug,
            toggle_debug_boxes,
            update_debug_boxes,
            shadow_debug,
        ),
    );

    if std::env::var("ELOSIUM_CAPTURE").is_ok() {
        let delay = std::env::var("ELOSIUM_CAPTURE_DELAY")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(3.5);
        app.insert_resource(AutoCapture {
            timer: Timer::from_seconds(delay, TimerMode::Once),
            stage: 0,
        });
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    render: Res<RenderSettings>,
    mut view: ResMut<ViewMode>,
    asset_server: Res<AssetServer>,
    window: Single<&Window>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut block_materials: ResMut<Assets<materials::BlockMaterial>>,
    mut wind_materials: ResMut<Assets<materials::WindMaterial>>,
    mut water_materials: ResMut<Assets<materials::WaterMaterial>>,
    mut sky_materials: ResMut<Assets<materials::SkyMaterial>>,
    mut cloud_materials: ResMut<Assets<materials::CloudMaterial>>,
) {
    let (reflection_image, reflection_screen) = create_image(&mut images, &window);
    spawn_camera(&mut commands, reflection_image.clone(), reflection_screen);

    let handles = create_materials(
        &asset_server,
        &mut block_materials,
        &mut wind_materials,
        &mut water_materials,
        &mut standard_materials,
        &mut sky_materials,
        &mut cloud_materials,
        reflection_image,
    );
    clouds::spawn_clouds(&mut commands, &mut meshes, &handles);
    spawn_crosshair(&mut commands, &mut meshes, &mut standard_materials);
    spawn_debug_overlay(&mut commands, &asset_server);
    spawn_debug_boxes(&mut commands, &mut meshes, &mut standard_materials);
    spawn_block_outline(&mut commands, &mut meshes, &mut standard_materials);
    menu::spawn_menu(&mut commands, &asset_server);
    let arm_entity = spawn_player_model(
        &mut commands,
        &mut meshes,
        &mut standard_materials,
        &asset_server,
    );

    let preview = (std::env::var("ELOSIUM_CAPTURE").is_ok()
        && std::env::var("ELOSIUM_UI").is_err())
        || std::env::var("ELOSIUM_BAKE_PANORAMA").is_ok();
    let center = std::env::var("ELOSIUM_CENTER")
        .ok()
        .and_then(|value| {
            let (x, z) = value.split_once(',')?;
            Some(ChunkPos::new(x.trim().parse().ok()?, z.trim().parse().ok()?))
        })
        .unwrap_or(ChunkPos::new(3, 6));
    let mut voxel_world = VoxelWorld::new(4_038_368_238);
    let mut spawn_feet = Vec3::new(8.5, 100.0, 8.5);
    if preview {
        if std::env::var("ELOSIUM_BAKE_PANORAMA").is_err() {
            for dx in -render.distance..=render.distance {
                for dz in -render.distance..=render.distance {
                    voxel_world.load_chunk(ChunkPos::new(center.x + dx, center.z + dz));
                }
            }
        }
        spawn_feet = voxel_world.find_land_spawn(center);
        if std::env::var("ELOSIUM_BAKE_PANORAMA").is_ok() {
            let spawn_chunk = ChunkPos::from_block(
                spawn_feet.x.floor() as i32,
                spawn_feet.z.floor() as i32,
            );
            for dx in -render.distance - 1..=render.distance + 1 {
                for dz in -render.distance - 1..=render.distance + 1 {
                    voxel_world.load_chunk(ChunkPos::new(
                        spawn_chunk.x + dx,
                        spawn_chunk.z + dz,
                    ));
                }
            }
        }
    }

    if let Some(value) = std::env::var("ELOSIUM_POS").ok() {
        let parts: Vec<f32> = value
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect();
        if parts.len() == 3 {
            spawn_feet = Vec3::new(parts[0], parts[1], parts[2]);
        }
    }

    if std::env::var("ELOSIUM_SWIM_TEST").is_ok() {
        let base_x = spawn_feet.x.floor() as i32;
        let base_z = spawn_feet.z.floor() as i32;
        'water: for radius in (0i32..=800).step_by(8) {
            for dx in (-radius..=radius).step_by(8) {
                for dz in (-radius..=radius).step_by(8) {
                    if radius > 0 && dx.abs() != radius && dz.abs() != radius {
                        continue;
                    }
                    let x = base_x + dx;
                    let z = base_z + dz;
                    voxel_world.load_chunk(ChunkPos::from_block(x, z));
                    let Some(column) = voxel_world.column_at(x, z) else {
                        continue;
                    };
                    let depth = column.water_top - column.height;
                    if depth < 2 {
                        continue;
                    }
                    let mut open_sky = true;
                    for dy in 1..=48 {
                        if !voxel_world
                            .get_block(x, column.water_top + dy, z)
                            .is_air()
                        {
                            open_sky = false;
                            break;
                        }
                    }
                    if !open_sky {
                        continue;
                    }
                    let mut open = 0;
                    for (nx, nz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        if voxel_world
                            .get_block(x + nx * 2, column.water_top, z + nz * 2)
                            .is_water()
                        {
                            open += 1;
                        }
                    }
                    if open < 4 {
                        continue;
                    }
                    spawn_feet = Vec3::new(
                        x as f32 + 0.5,
                        (column.height + 1) as f32 + 0.01,
                        z as f32 + 0.5,
                    );
                    println!(
                        "swim test at {x},{z} depth {depth} top {} height {}",
                        column.water_top, column.height
                    );
                    break 'water;
                }
            }
        }
    }

    if std::env::var("ELOSIUM_DEEP_TEST").is_ok() {
        let x = spawn_feet.x.floor() as i32;
        let z = spawn_feet.z.floor() as i32;
        let surface = voxel_world.surface_height(x, z);
        println!("deep surface {surface}");
        println!(
            "deep surface_block {:?} above {:?}",
            voxel_world.get_block(x, surface, z),
            voxel_world.get_block(x, surface + 1, z)
        );
        let mut y = crate::world::MIN_Y;
        while y <= surface {
            println!("deep y={} {:?}", y, voxel_world.get_block(x, y, z));
            y += 32;
        }
        if let Some(column) = voxel_world.column_at(x, z) {
            println!(
                "deep biome {:?} height {} water {} stream {:.2}",
                column.biome, column.height, column.water_top, column.stream
            );
        }
    }

    let yaw = std::env::var("ELOSIUM_YAW")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(-0.22);
    let pitch = std::env::var("ELOSIUM_PITCH")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(-0.22);
    let (player_uuid, player_name) = player::load_identity();

    if view.mode == 2 {
        view.orbit_yaw = std::env::var("ELOSIUM_ORBIT_YAW")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(yaw);
        view.orbit_pitch = std::env::var("ELOSIUM_ORBIT_PITCH")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(-0.95);
    }

    let camera_entity = commands
        .spawn((
        Camera3d::default(),
        Msaa::Sample4,
        IsDefaultUiCamera,
        RenderLayers::from_layers(&[0, 1]),
        Projection::Perspective(PerspectiveProjection {
            fov: 72f32.to_radians(),
            near: 0.1,
            far: 1200.0,
            ..default()
        }),
        Transform::from_translation(spawn_feet + Vec3::Y * EYE_HEIGHT).with_rotation(
            Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch),
        ),
        Player {
            uuid: player_uuid,
            name: player_name,
            body: spawn_feet + Vec3::Y * EYE_HEIGHT,
            feet: spawn_feet,
            prev_feet: spawn_feet,
            velocity: Vec3::ZERO,
            yaw,
            pitch,
            on_ground: false,
            horizontal_collision: false,
            swinging: false,
            swing: 0.0,
            body_yaw: yaw,
            sneaking: false,
            sprinting: false,
            swimming: false,
            pose: PlayerPose::Standing,
            tick_accumulator: 0.0,
            no_jump_delay: 0,
            sprint_trigger: 0,
            prev_forward: 0.0,
            tick_speed: 0.0,
            flying: std::env::var("ELOSIUM_FLY").is_ok(),
            jump_trigger_time: 0,
            prev_jump: false,
            eye_height_current: EYE_HEIGHT,
            eye_height_prev: EYE_HEIGHT,
            render_feet: spawn_feet,
        },
        DistanceFog {
            color: Color::srgb(0.72, 0.82, 0.95),
            falloff: FogFalloff::ExponentialSquared {
                density: FOG_DENSITY,
            },
            ..default()
        },
        VolumetricFog {
            ambient_intensity: 0.0,
            step_count: 40,
            jitter: 0.0,
            ..default()
        },
    ))
        .id();
    commands.entity(camera_entity).add_child(arm_entity);

    commands.spawn((
        GlobalFogVolume,
        FogVolume {
            fog_color: Color::srgb(0.78, 0.85, 0.96),
            density_factor: 0.0045,
            absorption: 0.15,
            scattering: 0.35,
            scattering_asymmetry: 0.55,
            ..default()
        },
        Transform::from_scale(Vec3::new(420.0, 320.0, 420.0)),
    ));

    commands.spawn((
        SunLight,
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 7_500.0,
            ..default()
        },
        VolumetricLight,
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            maximum_distance: 240.0,
            first_cascade_far_bound: 48.0,
            ..default()
        }
        .build(),
        Transform::default(),
        RenderLayers::from_layers(&[0, 2]),
    ));

    commands.spawn((
        Sky,
        Mesh3d(meshes.add(sky_cube_mesh())),
        MeshMaterial3d(handles.sky.clone()),
        Transform::from_scale(Vec3::splat(800.0)),
    ));

    commands.insert_resource(handles);
    commands.insert_resource(ChunkRenderer::default());
    commands.insert_resource(SpawnPoint(spawn_feet));
    commands.insert_resource(voxel_world);

    println!(
        "Elosium: click để khóa chuột · WASD di chuyển · Shift chạy · Space nhảy · Chuột trái phá block · Esc nhả chuột · F12 chụp ảnh."
    );
}

fn update_player_layers(
    view: Res<ViewMode>,
    mut parts: Query<
        &mut RenderLayers,
        (
            With<player_model::PlayerPart>,
            Without<player_model::ClonePart>,
        ),
    >,
) {
    let layer = if view.mode == 0 { 2 } else { 0 };
    for mut layers in &mut parts {
        *layers = RenderLayers::layer(layer);
    }
}

fn shadow_debug(
    mut frame: Local<u32>,
    parts: Query<
        (Entity, Option<&RenderLayers>, &ViewVisibility),
        With<player_model::PlayerPart>,
    >,
    sun: Query<(&RenderLayers, &CascadesVisibleEntities), With<SunLight>>,
) {
    if std::env::var("ELOSIUM_SHADOW_DEBUG").is_err() {
        return;
    }
    *frame += 1;
    if *frame % 120 != 0 {
        return;
    }
    for (entity, layers, visibility) in &parts {
        let present = sun.iter().any(|(_, cascades)| {
            cascades.entities.values().any(|view_cascades| {
                view_cascades
                    .iter()
                    .any(|cascade| cascade.entities.contains(&entity))
            })
        });
        println!(
            "shadow part {entity:?} layers={layers:?} visible={} in_cascade={present}",
            visibility.get()
        );
    }
    for (layers, cascades) in &sun {
        let blocks: Vec<usize> = cascades
            .entities
            .values()
            .map(|view_cascades| {
                view_cascades
                    .iter()
                    .map(|cascade| cascade.entities.len())
                    .sum::<usize>()
            })
            .collect();
        println!("shadow sun layers={layers:?} caster_counts={blocks:?}");
    }
}

fn update_plant_bend(
    player: Query<&Player, With<Camera3d>>,
    handles: Res<materials::GameMaterials>,
    mut winds: ResMut<Assets<materials::WindMaterial>>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let Some(mut material) = winds.get_mut(&handles.flora) else {
        return;
    };
    let feet = player.render_feet;
    if std::env::var("ELOSIUM_BEND_TEST").is_ok() {
        material.extension.settings.player = Vec4::new(feet.x, feet.y, feet.z, 0.0);
        material.extension.settings.bend = Vec4::new(2.2, 0.4, 3.0, 0.0);
    } else {
        let speed = player.speed_mps().clamp(0.0, 8.0);
        let strength = 0.7 + (speed * 0.05).min(0.25);
        material.extension.settings.player = Vec4::new(feet.x, feet.y, feet.z, 0.0);
        material.extension.settings.bend = Vec4::new(strength, 0.32, 1.15, 0.0);
    }
}

fn sky_follows_camera(
    camera: Query<&GlobalTransform, (With<Camera3d>, Without<Sky>, Without<ReflectionCamera>)>,
    mut sky: Query<&mut Transform, With<Sky>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    if let Ok(mut transform) = sky.single_mut() {
        transform.translation = camera.translation();
    }
}

fn manual_screenshot(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::F12) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("screenshots/elosium-{stamp}.png")));
    }
}

fn auto_screenshot(
    time: Res<Time>,
    mut capture: ResMut<AutoCapture>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    capture.timer.tick(time.delta());
    if !capture.timer.just_finished() {
        return;
    }

    if capture.stage == 0 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("screenshots/elosium.png"));
        capture.stage = 1;
        capture.timer = Timer::from_seconds(1.2, TimerMode::Once);
    } else {
        exit.write(AppExit::Success);
    }
}
