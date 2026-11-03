use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
};

use crate::player::{Player, PlayerPose, ViewMode};

const SKIN_SIZE: f32 = 64.0;
const MODEL_SCALE: f32 = 0.9375 / 16.0;

#[derive(Component)]
pub struct PlayerModelRoot;

#[derive(Component)]
pub struct FirstPersonArm;

#[derive(Component)]
pub struct PartPivot(pub Vec3);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPart {
    Head,
    Body,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
}

#[derive(Component)]
pub struct PlayerClone {
    pub name: String,
    pub uuid: u128,
    pub velocity: Vec3,
    pub on_ground: bool,
}

pub const CLONE_WIDTH: f32 = 0.6;
pub const CLONE_HEIGHT: f32 = 1.8;
const CLONE_SKIN: f32 = 0.001;

#[derive(Component)]
pub struct ClonePart;

#[derive(Component)]
pub struct NameTag(pub Entity);

#[derive(Message)]
pub struct SpawnClone {
    pub name: String,
}

const PARTS: [(PlayerPart, Vec3, Vec3, Vec3, Vec2); 6] = [
    (
        PlayerPart::Head,
        Vec3::new(0.0, 24.0, 0.0),
        Vec3::new(-4.0, 0.0, -4.0),
        Vec3::new(4.0, 8.0, 4.0),
        Vec2::new(0.0, 0.0),
    ),
    (
        PlayerPart::Body,
        Vec3::new(0.0, 24.0, 0.0),
        Vec3::new(-4.0, -12.0, -2.0),
        Vec3::new(4.0, 0.0, 2.0),
        Vec2::new(16.0, 16.0),
    ),
    (
        PlayerPart::ArmRight,
        Vec3::new(6.0, 22.0, 0.0),
        Vec3::new(-2.0, -10.0, -2.0),
        Vec3::new(2.0, 2.0, 2.0),
        Vec2::new(40.0, 16.0),
    ),
    (
        PlayerPart::ArmLeft,
        Vec3::new(-6.0, 22.0, 0.0),
        Vec3::new(-2.0, -10.0, -2.0),
        Vec3::new(2.0, 2.0, 2.0),
        Vec2::new(32.0, 48.0),
    ),
    (
        PlayerPart::LegRight,
        Vec3::new(2.0, 12.0, 0.0),
        Vec3::new(-2.0, -12.0, -2.0),
        Vec3::new(2.0, 0.0, 2.0),
        Vec2::new(0.0, 16.0),
    ),
    (
        PlayerPart::LegLeft,
        Vec3::new(-2.0, 12.0, 0.0),
        Vec3::new(-2.0, -12.0, -2.0),
        Vec3::new(2.0, 0.0, 2.0),
        Vec2::new(16.0, 48.0),
    ),
];

fn skin_material(
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/player_skin.png")),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        cull_mode: None,
        ..default()
    })
}

fn spawn_model_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    root: Entity,
    layer: usize,
    clone: bool,
) {
    for (kind, pivot, min, max, texture) in PARTS {
        let mut entity = commands.spawn((
            kind,
            Mesh3d(meshes.add(part_mesh(min, max, texture))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(pivot),
            PartPivot(pivot),
            RenderLayers::layer(layer),
        ));
        if clone {
            entity.insert(ClonePart);
        }
        let entity = entity.id();
        commands.entity(root).add_child(entity);
    }
}

pub fn spawn_player_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> Entity {
    let material = skin_material(materials, asset_server);
    let root = commands
        .spawn((
            PlayerModelRoot,
            Transform::from_scale(Vec3::splat(MODEL_SCALE)),
            Visibility::Inherited,
        ))
        .id();

    spawn_model_parts(commands, meshes, material.clone(), root, 2, false);

    commands
        .spawn((
            FirstPersonArm,
            Mesh3d(meshes.add(part_mesh(
                Vec3::new(-2.0, -12.0, -2.0),
                Vec3::new(2.0, 0.0, 2.0),
                Vec2::new(40.0, 16.0),
            ))),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            RenderLayers::layer(1),
        ))
        .id()
}

pub fn spawn_clone_players(
    mut messages: MessageReader<SpawnClone>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    screen: Res<crate::frontend::ScreenState>,
    world: Res<crate::world::VoxelWorld>,
    players: Query<&Player, With<Camera3d>>,
    mut demo: Local<bool>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let mut names: Vec<String> = messages.read().map(|message| message.name.clone()).collect();
    if !*demo && screen.screen == crate::frontend::Screen::Playing {
        *demo = true;
        if let Ok(name) = std::env::var("ELOSIUM_CLONE") {
            names.push(if name.trim().is_empty() {
                "Clone".to_string()
            } else {
                name
            });
        }
    }
    if names.is_empty() {
        return;
    }
    let forward = Vec3::new(-player.body_yaw.sin(), 0.0, -player.body_yaw.cos());
    let base = player.feet + forward * 3.2;
    let base_x = base.x.floor() as i32;
    let base_z = base.z.floor() as i32;
    let top = player.feet.y.floor() as i32 + 2;
    let mut ground = top;
    for y in (top - 5..=top).rev() {
        if world.get_block(base_x, y, base_z).is_solid() {
            ground = y + 1;
            break;
        }
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(1);
    for (index, name) in names.into_iter().enumerate() {
        let hash = name.bytes().fold(1_469_598_103_934_665_603u64, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(1_099_511_628_211)
        });
        let uuid = ((seed as u128) << 64) ^ (hash as u128) ^ (index as u128);
        let feet = Vec3::new(base.x, ground as f32, base.z);
        let clone = PlayerClone {
            name,
            uuid,
            velocity: Vec3::ZERO,
            on_ground: false,
        };
        let tag = format!("{}\n{}", clone.name, &format!("{:032x}", clone.uuid)[..8]);
        let root = commands
            .spawn((
                clone,
                Transform::from_translation(feet).with_rotation(Quat::from_rotation_y(
                    player.body_yaw + std::f32::consts::PI,
                )),
                Visibility::Inherited,
            ))
            .id();
        let model = commands
            .spawn((
                Transform::from_scale(Vec3::splat(MODEL_SCALE)),
                Visibility::Inherited,
            ))
            .id();
        commands.entity(root).add_child(model);
        let material = skin_material(&mut materials, &asset_server);
        spawn_model_parts(&mut commands, &mut meshes, material, model, 0, true);
        commands.spawn((
            NameTag(root),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(150),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(6), px(2)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.42)),
            Visibility::Hidden,
            children![(
                Text::new(tag),
                TextFont {
                    font: asset_server.load("fonts/mono.ttf").into(),
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            )],
        ));
    }
}

fn clone_collides(world: &crate::world::VoxelWorld, feet: Vec3) -> bool {
    let half = CLONE_WIDTH * 0.5;
    let min = Vec3::new(feet.x - half, feet.y, feet.z - half) + Vec3::splat(CLONE_SKIN);
    let max = Vec3::new(feet.x + half, feet.y + CLONE_HEIGHT, feet.z + half)
        - Vec3::splat(CLONE_SKIN);
    for x in min.x.floor() as i32..=max.x.floor() as i32 {
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                if world.is_solid(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

fn sweep_clone_axis(
    world: &crate::world::VoxelWorld,
    feet: &mut Vec3,
    amount: f32,
    axis: usize,
) -> f32 {
    if amount == 0.0 {
        return 0.0;
    }
    let start = *feet;
    let mut probe = start;
    match axis {
        0 => probe.x += amount,
        1 => probe.y += amount,
        _ => probe.z += amount,
    }
    if !clone_collides(world, probe) {
        *feet = probe;
        return amount;
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..20 {
        let mid = (low + high) * 0.5;
        let mut point = start;
        match axis {
            0 => point.x += amount * mid,
            1 => point.y += amount * mid,
            _ => point.z += amount * mid,
        }
        if clone_collides(world, point) {
            high = mid;
        } else {
            low = mid;
        }
    }
    let clipped = amount * low;
    let mut end = start;
    match axis {
        0 => end.x += clipped,
        1 => end.y += clipped,
        _ => end.z += clipped,
    }
    *feet = end;
    clipped
}

pub fn tick_clones(
    time: Res<Time>,
    world: Res<crate::world::VoxelWorld>,
    mut clones: Query<(&mut PlayerClone, &mut Transform)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut clone, mut transform) in &mut clones {
        let mut feet = transform.translation;
        let mut velocity = clone.velocity;
        velocity.y = (velocity.y - 22.0 * dt).max(-60.0);
        velocity.x *= (1.0 - 6.0 * dt).max(0.0);
        velocity.z *= (1.0 - 6.0 * dt).max(0.0);
        let delta = velocity * dt;
        let moved_y = sweep_clone_axis(&world, &mut feet, delta.y, 1);
        let vertical_hit = moved_y != delta.y;
        let on_ground = vertical_hit && delta.y < 0.0;
        if vertical_hit {
            velocity.y = 0.0;
        }
        sweep_clone_axis(&world, &mut feet, delta.x, 0);
        sweep_clone_axis(&world, &mut feet, delta.z, 2);
        if on_ground && velocity.y < 0.0 {
            velocity.y = 0.0;
        }
        clone.velocity = velocity;
        clone.on_ground = on_ground;
        if feet != transform.translation {
            transform.translation = feet;
        }
    }
}

pub fn push_player_from_clones(
    world: Res<crate::world::VoxelWorld>,
    mut players: Query<&mut Player, With<Camera3d>>,
    mut clones: Query<(&mut PlayerClone, &Transform), Without<Player>>,
) {
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    for (mut clone, transform) in &mut clones {
        let clone_feet = transform.translation;
        let player_half = crate::player::PLAYER_WIDTH * 0.5;
        let clone_half = CLONE_WIDTH * 0.5;
        let reach = player_half + clone_half;
        let dx = player.feet.x - clone_feet.x;
        let dz = player.feet.z - clone_feet.z;
        let vertical = player.feet.y < clone_feet.y + CLONE_HEIGHT
            && player.feet.y + crate::player::PLAYER_HEIGHT > clone_feet.y;
        let overlap_x = reach - dx.abs();
        let overlap_z = reach - dz.abs();
        if !vertical || overlap_x <= 0.0 || overlap_z <= 0.0 {
            continue;
        }
        let (push_x, push_z) = if overlap_x < overlap_z {
            let sign = if dx.abs() < 1e-4 { 1.0 } else { dx.signum() };
            (sign * overlap_x, 0.0)
        } else {
            let sign = if dz.abs() < 1e-4 { 1.0 } else { dz.signum() };
            (0.0, sign * overlap_z)
        };
        let mut target = player.feet + Vec3::new(push_x, 0.0, push_z);
        if clone_collides(&world, target) {
            let mut lifted = target;
            lifted.y += 0.35;
            if !clone_collides(&world, lifted) {
                target = lifted;
            } else {
                continue;
            }
        }
        player.feet = target;
        player.velocity.x *= 0.4;
        player.velocity.z *= 0.4;
        clone.velocity.x -= push_x * 0.35;
        clone.velocity.z -= push_z * 0.35;
    }
}

pub fn clear_clones_off_world(
    screen: Res<crate::frontend::ScreenState>,
    mut commands: Commands,
    clones: Query<Entity, With<PlayerClone>>,
    tags: Query<Entity, With<NameTag>>,
) {
    if screen.screen == crate::frontend::Screen::Playing {
        return;
    }
    for entity in &tags {
        commands.entity(entity).despawn();
    }
    for entity in &clones {
        commands.entity(entity).despawn();
    }
}

pub fn update_name_tags(
    screen: Res<crate::frontend::ScreenState>,
    camera: Query<(&GlobalTransform, &Camera), (With<Player>, With<Camera3d>)>,
    clones: Query<&GlobalTransform, With<PlayerClone>>,
    mut tags: Query<(&NameTag, &mut Node, &mut Visibility)>,
) {
    let Ok((camera_transform, camera)) = camera.single() else {
        return;
    };
    for (tag, mut node, mut visibility) in &mut tags {
        let Ok(transform) = clones.get(tag.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if screen.screen != crate::frontend::Screen::Playing {
            *visibility = Visibility::Hidden;
            continue;
        }
        let world = transform.translation() + Vec3::Y * 2.05;
        let relative = world - camera_transform.translation();
        if (camera_transform.rotation() * Vec3::NEG_Z).dot(relative) <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        let Ok(point) = camera.world_to_viewport(camera_transform, world) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        node.left = px(point.x - 75.0);
        node.top = px(point.y - 26.0);
        *visibility = Visibility::Inherited;
    }
}

pub fn animate_first_person_arm(
    time: Res<Time>,
    view: Res<ViewMode>,
    hud: Res<crate::debug::HudVisible>,
    player: Query<&Player, With<Camera3d>>,
    mut arm: Query<(&mut Transform, &mut Visibility), With<FirstPersonArm>>,
    mut last_feet: Local<Option<Vec3>>,
    mut phase: Local<f32>,
    mut amount: Local<f32>,
) {
    let (Ok(player), Ok((mut transform, mut visibility))) = (player.single(), arm.single_mut())
    else {
        return;
    };
    *visibility = if view.mode == 0 && hud.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    let delta = time.delta_secs().min(0.05);
    let feet = player.render_feet;
    let walked = last_feet
        .map(|previous| Vec2::new(feet.x - previous.x, feet.z - previous.z).length())
        .unwrap_or(0.0);
    *last_feet = Some(feet);
    *phase += walked * 1.9;
    let speed = walked / delta.max(0.0001);
    let target = (speed / 4.3).clamp(0.0, 1.0);
    let blend = 1.0 - (-delta * 12.0).exp();
    *amount += (target - *amount) * blend;

    let bob = phase.sin() * 0.012 * *amount;
    let swing = if player.swinging {
        (player.swing.clamp(0.0, 1.0) * std::f32::consts::PI).sin()
    } else {
        0.0
    };
    transform.translation = Vec3::new(
        0.56,
        -0.64 + bob - swing * 0.1,
        -0.58 + swing * 0.16,
    );
    transform.rotation = Quat::from_rotation_x(1.45 - swing * 0.9)
        * Quat::from_rotation_y(-0.35)
        * Quat::from_rotation_z(0.12);
    transform.scale = Vec3::splat(MODEL_SCALE * 0.95);
}

pub fn animate_player_model(
    time: Res<Time>,
    _view: Res<ViewMode>,
    player: Query<&Player, With<Camera3d>>,
    mut swing: Local<f32>,
    mut debug_frame: Local<u32>,
    mut root: Query<
        (&mut Transform, &mut Visibility),
        (With<PlayerModelRoot>, Without<PlayerPart>),
    >,
    mut parts: Query<(&PlayerPart, &PartPivot, &mut Transform), Without<PlayerModelRoot>>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let Ok((mut root_transform, mut visibility)) = root.single_mut() else {
        return;
    };
    *visibility = Visibility::Inherited;
    root_transform.translation = player.render_feet;
    root_transform.rotation = Quat::from_rotation_y(player.body_yaw);
    root_transform.scale = Vec3::splat(MODEL_SCALE);
    if player.pose == PlayerPose::Swimming {
        root_transform.translation.y += 0.4;
        root_transform.rotation *= Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    }

    let delta = time.delta_secs().min(0.05);
    let speed = player.speed_mps();
    let pose_test = std::env::var("ELOSIUM_POSE_TEST").is_ok();
    if pose_test {
        *swing = 1.0;
    } else {
        *swing += speed * delta * 1.9;
    }
    let amount = if pose_test {
        1.0
    } else {
        (speed / 4.3).clamp(0.0, 1.0)
    };

    let leg = swing.cos() * 1.4 * amount;
    let arm = swing.cos() * 1.4 * amount * 0.9;
    *debug_frame = debug_frame.wrapping_add(1);
    if std::env::var("ELOSIUM_ANIM_DEBUG").is_ok() && *debug_frame % 60 == 0 {
        println!(
            "anim speed={:.2} amount={:.2} swing={:.2} leg={:.2} arm={:.2}",
            speed, amount, *swing, leg, arm
        );
    }
    let sprint = ((speed - 5.0) / 1.4).clamp(0.0, 1.0);
    let crouch = if player.sneaking || std::env::var("ELOSIUM_SNEAK_TEST").is_ok() {
        1.0
    } else {
        0.0
    };
    let head_pitch = player.pitch;
    let mut relative = player.yaw - player.body_yaw;
    relative = (relative + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    relative = relative.clamp(-1.31, 1.31);
    let idle = (time.elapsed_secs() * 1.6).sin() * 0.02;
    let attack = if player.swinging {
        (player.swing.clamp(0.0, 1.0).sqrt() * std::f32::consts::PI).sin() * 1.2
    } else {
        0.0
    };
    let bob = (*swing * 2.0).cos().abs() * 0.02 * amount;
    root_transform.translation.y += bob;

    for (kind, pivot, mut transform) in &mut parts {
        let (rotation, offset) = match kind {
            PlayerPart::Head => (
                Quat::from_rotation_y(relative) * Quat::from_rotation_x(head_pitch),
                Vec3::new(0.0, -4.2 * crouch, 0.0),
            ),
            PlayerPart::Body => (
                Quat::from_rotation_x(-0.5 * crouch - sprint * 0.08),
                Vec3::new(0.0, -3.2 * crouch, 0.0),
            ),
            PlayerPart::ArmLeft => (
                Quat::from_rotation_x(arm - 0.4 * crouch + idle + attack * 0.3),
                Vec3::new(0.0, -3.2 * crouch, 0.0),
            ),
            PlayerPart::ArmRight => (
                Quat::from_rotation_x(-arm - 0.4 * crouch - idle + attack),
                Vec3::new(0.0, -3.2 * crouch, 0.0),
            ),
            PlayerPart::LegLeft => (
                Quat::from_rotation_x(-leg),
                Vec3::new(0.0, -0.2 * crouch, 4.0 * crouch),
            ),
            PlayerPart::LegRight => (
                Quat::from_rotation_x(leg),
                Vec3::new(0.0, -0.2 * crouch, 4.0 * crouch),
            ),
        };
        transform.rotation = rotation;
        transform.translation = pivot.0 + offset;
        transform.scale = Vec3::ONE;
    }
}

fn part_mesh(min: Vec3, max: Vec3, texture: Vec2) -> Mesh {
    let width = max.x - min.x;
    let height = max.y - min.y;
    let depth = max.z - min.z;
    let (u, v) = (texture.x, texture.y);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let front = [
        [min.x, max.y, min.z],
        [max.x, max.y, min.z],
        [max.x, min.y, min.z],
        [min.x, min.y, min.z],
    ];
    push_face(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        front,
        [0.0, 0.0, -1.0],
        [u + depth, v + depth, width, height],
    );

    let back = [
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
        [min.x, min.y, max.z],
        [max.x, min.y, max.z],
    ];
    push_face(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        back,
        [0.0, 0.0, 1.0],
        [u + 2.0 * depth + width, v + depth, width, height],
    );

    let right = [
        [max.x, max.y, max.z],
        [max.x, max.y, min.z],
        [max.x, min.y, min.z],
        [max.x, min.y, max.z],
    ];
    push_face(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        right,
        [1.0, 0.0, 0.0],
        [u, v + depth, depth, height],
    );

    let left = [
        [min.x, max.y, min.z],
        [min.x, max.y, max.z],
        [min.x, min.y, max.z],
        [min.x, min.y, min.z],
    ];
    push_face(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        left,
        [-1.0, 0.0, 0.0],
        [u + depth + width, v + depth, depth, height],
    );

    let top = [
        [min.x, max.y, max.z],
        [max.x, max.y, max.z],
        [max.x, max.y, min.z],
        [min.x, max.y, min.z],
    ];
    push_face(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        top,
        [0.0, 1.0, 0.0],
        [u + depth, v, width, depth],
    );

    let bottom = [
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [max.x, min.y, max.z],
        [min.x, min.y, max.z],
    ];
    push_face(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        bottom,
        [0.0, -1.0, 0.0],
        [u + depth + width, v, width, depth],
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    rect: [f32; 4],
) {
    let base = positions.len() as u32;
    let [x, y, w, h] = rect;
    let u0 = x / SKIN_SIZE;
    let v0 = y / SKIN_SIZE;
    let u1 = (x + w) / SKIN_SIZE;
    let v1 = (y + h) / SKIN_SIZE;
    let face_uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
    for (index, corner) in corners.iter().enumerate() {
        positions.push(*corner);
        normals.push(normal);
        uvs.push(face_uvs[index]);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
