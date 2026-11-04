use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
};

use std::collections::HashMap;

use crate::{
    blocks::Block,
    debug::HudVisible,
    hud::Inventory,
    materials::{BlockMaterial, GameMaterials},
    player::{Player, PlayerPose, ViewMode},
};

const SKIN_SIZE: f32 = 64.0;
const MODEL_SCALE: f32 = 0.9375 / 16.0;

#[derive(Component)]
pub struct PlayerModelRoot;

#[derive(Component)]
pub struct FirstPersonArm;

#[derive(Component)]
pub struct HeldBlock {
    block: Option<Block>,
}

#[derive(Component)]
pub struct PartPivot(pub Vec3);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPart {
    Head,
    Body,
    ArmLeft,
    ArmRight,
    ForearmLeft,
    ForearmRight,
    LegLeft,
    LegRight,
    ShinLeft,
    ShinRight,
}

#[derive(Component)]
pub struct PlayerLimb;

#[derive(Component)]
pub struct PlayerLayer;

#[derive(Clone, Copy, Default)]
pub struct LimbBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl LimbBox {
    pub fn overlaps(&self, min: Vec3, max: Vec3) -> bool {
        self.min.x < max.x
            && self.max.x > min.x
            && self.min.y < max.y
            && self.max.y > min.y
            && self.min.z < max.z
            && self.max.z > min.z
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}

#[derive(Resource, Default)]
pub struct LimbContacts {
    pub boxes: [LimbBox; 10],
    pub hands: [Vec3; 2],
    pub feet: [Vec3; 2],
}

const SLOT_HEAD: usize = 0;
const SLOT_BODY: usize = 1;
const SLOT_ARM_LEFT: usize = 2;
const SLOT_ARM_RIGHT: usize = 3;
const SLOT_FOREARM_LEFT: usize = 4;
const SLOT_FOREARM_RIGHT: usize = 5;
const SLOT_LEG_LEFT: usize = 6;
const SLOT_LEG_RIGHT: usize = 7;
const SLOT_SHIN_LEFT: usize = 8;
const SLOT_SHIN_RIGHT: usize = 9;

fn part_slot(part: PlayerPart) -> usize {
    match part {
        PlayerPart::Head => SLOT_HEAD,
        PlayerPart::Body => SLOT_BODY,
        PlayerPart::ArmLeft => SLOT_ARM_LEFT,
        PlayerPart::ArmRight => SLOT_ARM_RIGHT,
        PlayerPart::ForearmLeft => SLOT_FOREARM_LEFT,
        PlayerPart::ForearmRight => SLOT_FOREARM_RIGHT,
        PlayerPart::LegLeft => SLOT_LEG_LEFT,
        PlayerPart::LegRight => SLOT_LEG_RIGHT,
        PlayerPart::ShinLeft => SLOT_SHIN_LEFT,
        PlayerPart::ShinRight => SLOT_SHIN_RIGHT,
    }
}

fn part_bounds(part: PlayerPart) -> (Vec3, Vec3) {
    PARTS
        .iter()
        .find(|(kind, _, _, _, _, _)| *kind == part)
        .map(|(_, _, min, max, _, _)| (*min, *max))
        .unwrap_or((Vec3::ZERO, Vec3::ZERO))
}

fn clamp_hand_target(
    world: &crate::world::VoxelWorld,
    shoulder: Vec3,
    target: Vec3,
    root_rotation: Quat,
    root_position: Vec3,
    side: f32,
) -> Vec3 {
    let to_local = |point: Vec3| root_rotation.inverse() * (point - root_position);
    let to_world = |point: Vec3| root_position + root_rotation * point;
    let shoulder_local = to_local(shoulder);
    let mut local = to_local(target);
    local.x = side * local.x.abs().max(0.30);
    local.y = local.y.clamp(0.55, 1.65);
    local.z = local.z.clamp(-0.62, 0.35 * local.x.abs());
    let offset = local - shoulder_local;
    let reach = offset
        .length()
        .clamp(0.26, (ARM_LENGTH * 2.0 - 0.4) * MODEL_SCALE);
    local = shoulder_local + offset.normalize_or_zero() * reach;
    let mut world_target = to_world(local);
    let direction = (world_target - shoulder).normalize_or_zero();
    let distance = (world_target - shoulder).length();
    let steps = 16;
    let mut previous = shoulder;
    for step in 1..=steps {
        let sample = shoulder + direction * (distance * step as f32 / steps as f32);
        let block = world.get_block(
            sample.x.floor() as i32,
            sample.y.floor() as i32,
            sample.z.floor() as i32,
        );
        if block.is_solid() {
            world_target = previous + direction * 0.01;
            break;
        }
        previous = sample;
    }
    world_target
}

fn foot_support(
    world: &crate::world::VoxelWorld,
    foot: Vec3,
    feet: Vec3,
) -> (f32, bool) {
    let x = foot.x.floor() as i32;
    let z = foot.z.floor() as i32;
    let base = feet.y.floor() as i32;
    let mut support = feet.y;
    for y in (base - 1..=base + 1).rev() {
        let block = world.get_block(x, y, z);
        if block.is_solid() {
            let surface = y as f32 + 1.0;
            if surface <= feet.y + 0.55 {
                support = support.max(surface);
            }
            break;
        }
    }
    let plant = world.get_block(x, base, z).is_plant();
    if plant {
        support = support.max(base as f32 + 0.3);
    }
    (support, plant)
}

fn aim_bone(direction: Vec3, reference: Vec3) -> Quat {
    let y = -direction.normalize_or_zero();
    if y.length_squared() < 1.0e-6 {
        return Quat::IDENTITY;
    }
    let mut z = reference - y * reference.dot(y);
    if z.length_squared() < 1.0e-6 {
        z = if y.y.abs() < 0.9 {
            Vec3::Y.cross(y)
        } else {
            Vec3::X.cross(y)
        };
    }
    let z = z.normalize_or_zero();
    let x = y.cross(z).normalize_or_zero();
    Quat::from_mat3(&Mat3::from_cols(x, y, z))
}

fn solve_two_bone(
    shoulder: Vec3,
    target: Vec3,
    upper: f32,
    lower: f32,
    pole: Vec3,
) -> Option<(Vec3, Vec3)> {
    let delta = target - shoulder;
    let raw = delta.length();
    if raw < 1.0e-4 {
        return None;
    }
    let direction = delta / raw;
    let reach = raw.clamp((upper - lower).abs() + 0.02, upper + lower - 0.02);
    let along = (upper * upper - lower * lower + reach * reach) / (2.0 * reach);
    let height = (upper * upper - along * along).max(0.0).sqrt();
    let pole_dir = (pole - direction * pole.dot(direction)).normalize_or_zero();
    let elbow = shoulder + direction * along + pole_dir * height;
    let hand = shoulder + direction * reach;
    Some((elbow, hand))
}

fn hand_contact_target(
    world: &crate::world::VoxelWorld,
    shoulder: Vec3,
    feet: Vec3,
    side: Vec3,
    moving: bool,
) -> Option<(Vec3, f32)> {
    let base = shoulder.floor();
    let mut solid: Option<(f32, Vec3)> = None;
    let mut plant: Option<(f32, Vec3)> = None;
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let x = base.x as i32 + dx;
                let y = base.y as i32 + dy;
                let z = base.z as i32 + dz;
                let block = world.get_block(x, y, z);
                if !block.is_solid() && !(moving && block.is_plant()) {
                    continue;
                }
                let min = Vec3::new(x as f32, y as f32, z as f32);
                let max = min + Vec3::ONE;
                let nearest = shoulder.clamp(min, max);
                let offset = nearest - shoulder;
                let distance = offset.length();
                if distance < 0.01 || distance > 0.56 {
                    continue;
                }
                let lateral = offset.normalize_or_zero().dot(side);
                if lateral < 0.15 {
                    continue;
                }
                let point = nearest + offset.normalize_or_zero() * 0.02;
                if block.is_solid() {
                    let top = y as f32 + 1.0;
                    let hand_band = nearest.y >= feet.y + 0.5 && nearest.y <= feet.y + 1.75;
                    let step = top >= feet.y + 0.95 && top <= feet.y + 2.0;
                    if (hand_band || step) && solid.map_or(true, |(best, _)| distance < best) {
                        solid = Some((distance, point));
                    }
                } else if moving && plant.map_or(true, |(best, _)| distance < best) {
                    plant = Some((distance, point));
                }
            }
        }
    }
    if let Some((_, point)) = solid {
        return Some((point, 1.0));
    }
    plant.map(|(_, point)| (point, 0.55))
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
pub struct CloneOwner(pub Entity);

#[derive(Component)]
pub struct NameTag(pub Entity);

#[derive(Message)]
pub struct SpawnClone {
    pub name: String,
}

const ARM_LENGTH: f32 = 5.0;
const PARTS: [(PlayerPart, Vec3, Vec3, Vec3, Vec2, Vec2); 10] = [
    (
        PlayerPart::Head,
        Vec3::new(0.0, 24.0, 0.0),
        Vec3::new(-4.0, 0.0, -4.0),
        Vec3::new(4.0, 8.0, 4.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 0.0),
    ),
    (
        PlayerPart::Body,
        Vec3::new(0.0, 24.0, 0.0),
        Vec3::new(-4.0, -12.0, -2.0),
        Vec3::new(4.0, 0.0, 2.0),
        Vec2::new(16.0, 16.0),
        Vec2::new(16.0, 16.0),
    ),
    (
        PlayerPart::ArmRight,
        Vec3::new(6.0, 22.0, 0.0),
        Vec3::new(-2.0, -ARM_LENGTH, -2.0),
        Vec3::new(2.0, 2.0, 2.0),
        Vec2::new(40.0, 16.0),
        Vec2::new(40.0, 16.0),
    ),
    (
        PlayerPart::ArmLeft,
        Vec3::new(-6.0, 22.0, 0.0),
        Vec3::new(-2.0, -ARM_LENGTH, -2.0),
        Vec3::new(2.0, 2.0, 2.0),
        Vec2::new(32.0, 48.0),
        Vec2::new(32.0, 48.0),
    ),
    (
        PlayerPart::ForearmRight,
        Vec3::new(0.0, -ARM_LENGTH, 0.0),
        Vec3::new(-1.98, -ARM_LENGTH, -1.98),
        Vec3::new(1.98, 1.0, 1.98),
        Vec2::new(40.0, 21.0),
        Vec2::new(40.0, 24.0),
    ),
    (
        PlayerPart::ForearmLeft,
        Vec3::new(0.0, -ARM_LENGTH, 0.0),
        Vec3::new(-1.98, -ARM_LENGTH, -1.98),
        Vec3::new(1.98, 1.0, 1.98),
        Vec2::new(32.0, 53.0),
        Vec2::new(32.0, 56.0),
    ),
    (
        PlayerPart::LegRight,
        Vec3::new(2.0, 12.0, 0.0),
        Vec3::new(-2.0, -6.0, -2.0),
        Vec3::new(2.0, 0.0, 2.0),
        Vec2::new(0.0, 16.0),
        Vec2::new(0.0, 16.0),
    ),
    (
        PlayerPart::LegLeft,
        Vec3::new(-2.0, 12.0, 0.0),
        Vec3::new(-2.0, -6.0, -2.0),
        Vec3::new(2.0, 0.0, 2.0),
        Vec2::new(16.0, 48.0),
        Vec2::new(16.0, 48.0),
    ),
    (
        PlayerPart::ShinRight,
        Vec3::new(0.0, -6.0, 0.0),
        Vec3::new(-1.98, -6.0, -1.98),
        Vec3::new(1.98, 1.0, 1.98),
        Vec2::new(0.0, 21.0),
        Vec2::new(0.0, 16.0),
    ),
    (
        PlayerPart::ShinLeft,
        Vec3::new(0.0, -6.0, 0.0),
        Vec3::new(-1.98, -6.0, -1.98),
        Vec3::new(1.98, 1.0, 1.98),
        Vec2::new(16.0, 53.0),
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
    overlay: Handle<StandardMaterial>,
    root: Entity,
    layer: usize,
    clone: bool,
    owner: Option<Entity>,
) {
    let mut ids = [Entity::PLACEHOLDER; 10];
    for (kind, pivot, min, max, texture, cap) in PARTS {
        let mut entity = commands.spawn((
            kind,
            Mesh3d(meshes.add(part_mesh(min, max, texture, cap))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(pivot),
            PartPivot(pivot),
            RenderLayers::layer(layer),
        ));
        if clone {
            entity.insert(ClonePart);
            if let Some(owner) = owner {
                entity.insert(CloneOwner(owner));
            }
        } else {
            entity.insert(PlayerLimb);
            entity.insert(PlayerLayer);
        }
        let id = entity.id();
        ids[part_slot(kind)] = id;
        if let Some((texture, cap)) = overlay_texture(kind) {
            let mut overlay_entity = commands.spawn((
                Mesh3d(meshes.add(part_mesh(
                    min - Vec3::splat(0.5),
                    max + Vec3::splat(0.5),
                    texture,
                    cap,
                ))),
                MeshMaterial3d(overlay.clone()),
                Transform::IDENTITY,
                NotShadowCaster,
                RenderLayers::layer(layer),
            ));
            if !clone {
                overlay_entity.insert(PlayerLayer);
            }
            let overlay_id = overlay_entity.id();
            commands.entity(id).add_child(overlay_id);
        }
        if !matches!(
            kind,
            PlayerPart::ForearmLeft
                | PlayerPart::ForearmRight
                | PlayerPart::ShinLeft
                | PlayerPart::ShinRight
        ) {
            commands.entity(root).add_child(id);
        }
    }
    for (forearm, arm) in [
        (SLOT_FOREARM_LEFT, SLOT_ARM_LEFT),
        (SLOT_FOREARM_RIGHT, SLOT_ARM_RIGHT),
        (SLOT_SHIN_LEFT, SLOT_LEG_LEFT),
        (SLOT_SHIN_RIGHT, SLOT_LEG_RIGHT),
    ] {
        if ids[forearm] != Entity::PLACEHOLDER && ids[arm] != Entity::PLACEHOLDER {
            commands.entity(ids[arm]).add_child(ids[forearm]);
        }
    }
}

pub fn spawn_player_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> Entity {
    let material = skin_material(materials, asset_server);
    let overlay = overlay_material(materials, asset_server);
    let root = commands
        .spawn((
            PlayerModelRoot,
            Transform::from_scale(Vec3::splat(MODEL_SCALE)),
            Visibility::Inherited,
        ))
        .id();

    spawn_model_parts(
        commands,
        meshes,
        material.clone(),
        overlay,
        root,
        2,
        false,
        None,
    );

    commands
        .spawn((
            FirstPersonArm,
            Mesh3d(meshes.add(part_mesh(
                Vec3::new(-2.0, -12.0, -2.0),
                Vec3::new(2.0, 0.0, 2.0),
                Vec2::new(40.0, 16.0),
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
        let overlay = overlay_material(&mut materials, &asset_server);
        spawn_model_parts(
            &mut commands,
            &mut meshes,
            material,
            overlay,
            model,
            0,
            true,
            Some(root),
        );
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
    contacts: Res<LimbContacts>,
    mut players: Query<&mut Player, With<Camera3d>>,
    mut clones: Query<(&mut PlayerClone, &Transform), Without<Player>>,
) {
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    for (mut clone, transform) in &mut clones {
        let clone_feet = transform.translation;
        let clone_half = CLONE_WIDTH * 0.5;
        let clone_min = clone_feet - Vec3::new(clone_half, 0.0, clone_half);
        let clone_max = clone_feet + Vec3::new(clone_half, CLONE_HEIGHT, clone_half);
        for slot in 2..10 {
            let limb = contacts.boxes[slot];
            if !limb.overlaps(clone_min, clone_max) {
                continue;
            }
            let center = limb.center();
            let dx = clone_feet.x - center.x;
            let dz = clone_feet.z - center.z;
            let sign_x = if dx.abs() < 1e-4 { 1.0 } else { dx.signum() };
            let sign_z = if dz.abs() < 1e-4 { 1.0 } else { dz.signum() };
            clone.velocity.x += sign_x * 0.25;
            clone.velocity.z += sign_z * 0.25;
        }
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
    view: Res<ViewMode>,
    world: Res<crate::world::VoxelWorld>,
    player: Query<&Player, With<Camera3d>>,
    mut swing: Local<f32>,
    mut debug_frame: Local<u32>,
    mut left_reach: Local<f32>,
    mut right_reach: Local<f32>,
    mut contacts: ResMut<LimbContacts>,
    mut root: Query<
        (&mut Transform, &mut Visibility),
        (With<PlayerModelRoot>, Without<PlayerPart>),
    >,
    mut parts: Query<
        (&PlayerPart, &PartPivot, &mut Transform, Option<&PlayerLimb>),
        (Without<PlayerModelRoot>, Without<ClonePart>),
    >,
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

    let feet = player.render_feet;
    let interactive = view.mode != 0;
    let arm_offset = Vec3::new(0.0, -3.2 * crouch, 0.0);
    let right = Vec3::new(player.body_yaw.cos(), 0.0, -player.body_yaw.sin());
    let mut ik_upper = [Quat::IDENTITY; 2];
    let mut ik_fore = [Quat::IDENTITY; 2];
    let mut ik_target = [0.0f32; 2];
    if interactive {
        for (index, sign) in [-1.0f32, 1.0].into_iter().enumerate() {
            let side = right * sign;
            let shoulder_pivot =
                Vec3::new(if sign < 0.0 { -6.0 } else { 6.0 }, 22.0, 0.0) + arm_offset;
            let shoulder = root_transform.transform_point(shoulder_pivot);
            if let Some((point, weight)) =
                hand_contact_target(&world, shoulder, feet, side, speed > 0.6)
            {
                let point = clamp_hand_target(
                    &world,
                    shoulder,
                    point,
                    root_transform.rotation,
                    root_transform.translation,
                    sign,
                );
                let local_target =
                    root_transform.rotation.inverse() * (point - root_transform.translation)
                        / MODEL_SCALE;
                let pole = Vec3::new(sign * 0.35, -0.85, 0.4);
                if let Some((elbow, hand)) =
                    solve_two_bone(shoulder_pivot, local_target, ARM_LENGTH, ARM_LENGTH, pole)
                {
                    let upper = aim_bone(
                        (elbow - shoulder_pivot).normalize_or_zero(),
                        Vec3::Z,
                    );
                    let lower = aim_bone((hand - elbow).normalize_or_zero(), Vec3::Z);
                    ik_upper[index] = upper;
                    ik_fore[index] = upper.inverse() * lower;
                    ik_target[index] = weight;
                }
            }
        }
    }
    let reach_blend = 1.0 - (-delta * 9.0).exp();
    *left_reach += (ik_target[0] - *left_reach) * reach_blend;
    *right_reach += (ik_target[1] - *right_reach) * reach_blend;
    if std::env::var("ELOSIUM_ANIM_DEBUG").is_ok() && *debug_frame % 60 == 0 {
        println!(
            "limb ik=({:.2},{:.2}) feet={:?} yaw={:.2}",
            *left_reach, *right_reach, feet, player.body_yaw
        );
    }
    let leg_offset = Vec3::new(0.0, -0.2 * crouch, 4.0 * crouch);
    let mut leg_upper = [Quat::IDENTITY; 2];
    let mut leg_lower = [Quat::IDENTITY; 2];
    let mut leg_ik = [false; 2];
    if player.on_ground && player.pose == PlayerPose::Standing {
        for (index, sign) in [-1.0f32, 1.0].into_iter().enumerate() {
            let foot_world = feet + root_transform.rotation * Vec3::new(sign * 2.0, 0.0, 0.0) * MODEL_SCALE;
            let (support, plant) = foot_support(&world, foot_world, feet);
            if support <= feet.y + 0.02 && !plant {
                continue;
            }
            let target_y = (support - feet.y) / MODEL_SCALE;
            let hip = Vec3::new(sign * 2.0, 12.0, 0.0) + leg_offset;
            let target = Vec3::new(sign * 2.0, target_y, 0.0);
            let pole = Vec3::new(sign * 0.15, 0.0, -1.0);
            if let Some((knee, _)) = solve_two_bone(hip, target, 6.0, 6.0, pole) {
                let upper = aim_bone((knee - hip).normalize_or_zero(), Vec3::Z);
                let lower = aim_bone((target - knee).normalize_or_zero(), Vec3::Z);
                leg_upper[index] = upper;
                leg_lower[index] = upper.inverse() * lower;
                leg_ik[index] = true;
            }
        }
    }
    let applied_leg = [
        if leg_ik[0] {
            leg_upper[0]
        } else {
            Quat::from_rotation_x(-leg)
        },
        if leg_ik[1] {
            leg_upper[1]
        } else {
            Quat::from_rotation_x(leg)
        },
    ];
    let applied_shin = [
        Quat::IDENTITY.slerp(leg_lower[0], if leg_ik[0] { 1.0 } else { 0.0 }),
        Quat::IDENTITY.slerp(leg_lower[1], if leg_ik[1] { 1.0 } else { 0.0 }),
    ];
    let root_tf = Transform {
        translation: root_transform.translation,
        rotation: root_transform.rotation,
        scale: root_transform.scale,
    };
    let applied_upper = [
        Quat::from_rotation_x(arm - 0.4 * crouch + idle + attack * 0.3)
            .slerp(ik_upper[0], *left_reach),
        Quat::from_rotation_x(-arm - 0.4 * crouch - idle + attack)
            .slerp(ik_upper[1], *right_reach),
    ];
    let applied_fore = [
        Quat::IDENTITY.slerp(ik_fore[0], *left_reach),
        Quat::IDENTITY.slerp(ik_fore[1], *right_reach),
    ];

    for (kind, pivot, mut transform, limb) in &mut parts {
        let (rotation, offset) = match kind {
            PlayerPart::Head => (
                Quat::from_rotation_y(relative) * Quat::from_rotation_x(head_pitch),
                Vec3::new(0.0, -4.2 * crouch, 0.0),
            ),
            PlayerPart::Body => (
                Quat::from_rotation_x(-0.5 * crouch - sprint * 0.08),
                Vec3::new(0.0, -3.2 * crouch, 0.0),
            ),
            PlayerPart::ArmLeft => (applied_upper[0], arm_offset),
            PlayerPart::ArmRight => (applied_upper[1], arm_offset),
            PlayerPart::ForearmLeft => (applied_fore[0], Vec3::ZERO),
            PlayerPart::ForearmRight => (applied_fore[1], Vec3::ZERO),
            PlayerPart::LegLeft => (applied_leg[0], leg_offset),
            PlayerPart::LegRight => (applied_leg[1], leg_offset),
            PlayerPart::ShinLeft => (applied_shin[0], Vec3::ZERO),
            PlayerPart::ShinRight => (applied_shin[1], Vec3::ZERO),
        };
        transform.rotation = rotation;
        if matches!(
            kind,
            PlayerPart::ForearmLeft | PlayerPart::ForearmRight
        ) {
            transform.translation = Vec3::new(0.0, -ARM_LENGTH, 0.0);
        } else if matches!(kind, PlayerPart::ShinLeft | PlayerPart::ShinRight) {
            transform.translation = Vec3::new(0.0, -6.0, 0.0);
        } else {
            transform.translation = pivot.0 + offset;
        }
        transform.scale = Vec3::ONE;

        if limb.is_some() {
            let slot = part_slot(*kind);
            let world_part = match kind {
                PlayerPart::ForearmLeft | PlayerPart::ForearmRight => {
                    let left = *kind == PlayerPart::ForearmLeft;
                    let shoulder_pivot =
                        Vec3::new(if left { -6.0 } else { 6.0 }, 22.0, 0.0) + arm_offset;
                    let upper = applied_upper[if left { 0 } else { 1 }];
                    let upper_world = root_tf.mul_transform(Transform {
                        translation: shoulder_pivot,
                        rotation: upper,
                        scale: Vec3::ONE,
                    });
                    upper_world.mul_transform(Transform {
                        translation: Vec3::new(0.0, -ARM_LENGTH, 0.0),
                        rotation,
                        scale: Vec3::ONE,
                    })
                }
                PlayerPart::ShinLeft | PlayerPart::ShinRight => {
                    let left = *kind == PlayerPart::ShinLeft;
                    let hip_pivot = Vec3::new(if left { -2.0 } else { 2.0 }, 12.0, 0.0)
                        + leg_offset;
                    let upper = applied_leg[if left { 0 } else { 1 }];
                    let upper_world = root_tf.mul_transform(Transform {
                        translation: hip_pivot,
                        rotation: upper,
                        scale: Vec3::ONE,
                    });
                    upper_world.mul_transform(Transform {
                        translation: Vec3::new(0.0, -6.0, 0.0),
                        rotation,
                        scale: Vec3::ONE,
                    })
                }
                _ => root_tf.mul_transform(Transform {
                    translation: pivot.0 + offset,
                    rotation,
                    scale: Vec3::ONE,
                }),
            };
            let (min, max) = part_bounds(*kind);
            let mut bmin = Vec3::splat(f32::MAX);
            let mut bmax = Vec3::splat(f32::MIN);
            for corner in 0..8u8 {
                let point = world_part.transform_point(Vec3::new(
                    if corner & 1 == 0 { min.x } else { max.x },
                    if corner & 2 == 0 { min.y } else { max.y },
                    if corner & 4 == 0 { min.z } else { max.z },
                ));
                bmin = bmin.min(point);
                bmax = bmax.max(point);
            }
            contacts.boxes[slot] = LimbBox { min: bmin, max: bmax };
            match kind {
                PlayerPart::ForearmLeft => {
                    contacts.hands[0] =
                        world_part.transform_point(Vec3::new(0.0, -ARM_LENGTH, 0.0));
                }
                PlayerPart::ForearmRight => {
                    contacts.hands[1] =
                        world_part.transform_point(Vec3::new(0.0, -ARM_LENGTH, 0.0));
                }
                PlayerPart::ShinLeft => {
                    contacts.feet[0] = world_part.transform_point(Vec3::new(0.0, -6.0, 0.0));
                }
                PlayerPart::ShinRight => {
                    contacts.feet[1] = world_part.transform_point(Vec3::new(0.0, -6.0, 0.0));
                }
                _ => {}
            }
        }
    }
}

pub fn animate_clone_models(
    time: Res<Time>,
    mut phases: Local<HashMap<Entity, f32>>,
    clones: Query<(Entity, &PlayerClone)>,
    mut parts: Query<(&CloneOwner, &PlayerPart, &mut Transform), With<ClonePart>>,
) {
    if parts.is_empty() {
        return;
    }
    let mut states: HashMap<Entity, (f32, f32)> = HashMap::new();
    let delta = time.delta_secs().min(0.05);
    for (entity, clone) in &clones {
        let speed = Vec2::new(clone.velocity.x, clone.velocity.z).length();
        let phase = phases.entry(entity).or_insert(0.0);
        *phase += speed * delta * 1.9;
        states.insert(entity, (*phase, (speed / 4.3).clamp(0.0, 1.0)));
    }
    for (owner, kind, mut transform) in &mut parts {
        let Some(&(phase, amount)) = states.get(&owner.0) else {
            continue;
        };
        let swing = phase.cos() * 1.4 * amount;
        transform.rotation = match kind {
            PlayerPart::ArmLeft => Quat::from_rotation_x(swing * 0.9),
            PlayerPart::ArmRight => Quat::from_rotation_x(-swing * 0.9),
            PlayerPart::LegLeft => Quat::from_rotation_x(-swing),
            PlayerPart::LegRight => Quat::from_rotation_x(swing),
            _ => Quat::IDENTITY,
        };
    }
}

fn overlay_texture(part: PlayerPart) -> Option<(Vec2, Vec2)> {
    match part {
        PlayerPart::Head => Some((Vec2::new(32.0, 0.0), Vec2::new(32.0, 0.0))),
        PlayerPart::Body => Some((Vec2::new(16.0, 32.0), Vec2::new(16.0, 32.0))),
        PlayerPart::ArmRight => Some((Vec2::new(40.0, 32.0), Vec2::new(40.0, 32.0))),
        PlayerPart::ArmLeft => Some((Vec2::new(48.0, 32.0), Vec2::new(48.0, 32.0))),
        PlayerPart::ForearmRight => Some((Vec2::new(40.0, 38.0), Vec2::new(40.0, 32.0))),
        PlayerPart::ForearmLeft => Some((Vec2::new(48.0, 38.0), Vec2::new(48.0, 32.0))),
        PlayerPart::LegRight => Some((Vec2::new(0.0, 32.0), Vec2::new(0.0, 32.0))),
        PlayerPart::LegLeft => Some((Vec2::new(16.0, 32.0), Vec2::new(16.0, 32.0))),
        PlayerPart::ShinLeft | PlayerPart::ShinRight => None,
    }
}

fn overlay_material(
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/player_skin.png")),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        ..default()
    })
}

fn part_mesh(min: Vec3, max: Vec3, texture: Vec2, cap: Vec2) -> Mesh {
    let width = max.x - min.x;
    let height = max.y - min.y;
    let depth = max.z - min.z;
    let (u, v) = (texture.x, texture.y);
    let (cap_u, cap_v) = (cap.x, cap.y);

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
        [cap_u + depth, cap_v, width, depth],
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
        [cap_u + depth + width, cap_v, width, depth],
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

pub fn spawn_held_block(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &GameMaterials,
) -> Entity {
    commands
        .spawn((
            HeldBlock { block: None },
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.particle.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            RenderLayers::layer(1),
        ))
        .id()
}

pub fn update_held_block(
    inventory: Res<Inventory>,
    view: Res<ViewMode>,
    hud: Res<HudVisible>,
    materials: Res<GameMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: Local<HashMap<Block, Handle<Mesh>>>,
    players: Query<(&Player, &GlobalTransform), With<Camera3d>>,
    mut held: Query<(
        &mut HeldBlock,
        &mut Mesh3d,
        &mut MeshMaterial3d<BlockMaterial>,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let (Ok((player, camera)), Ok((mut state, mut mesh, mut material, mut transform, mut visibility))) =
        (players.single(), held.single_mut())
    else {
        return;
    };
    let selected = inventory.slots[inventory.selected].map(|stack| stack.block);
    if view.mode != 0 || !hud.0 || selected.is_none() {
        *visibility = Visibility::Hidden;
        state.block = None;
        return;
    }
    let block = selected.unwrap();
    if state.block != Some(block) {
        let handle = cache
            .entry(block)
            .or_insert_with(|| crate::particles::block_item_mesh(&mut meshes, block));
        mesh.0 = handle.clone();
        material.0 = crate::particles::block_particle_material(&materials, block);
        state.block = Some(block);
    }
    let rotation = camera.rotation();
    let forward = rotation * Vec3::NEG_Z;
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let swing = if player.swinging {
        (player.swing.clamp(0.0, 1.0) * std::f32::consts::PI).sin()
    } else {
        0.0
    };
    let base = camera.translation() + forward * 0.62 - up * 0.30 + right * 0.30;
    transform.translation = base - up * (swing * 0.12) + forward * (swing * 0.10);
    transform.rotation = rotation
        * Quat::from_rotation_y(0.55)
        * Quat::from_rotation_x(-0.30 + swing * 0.7);
    transform.scale = Vec3::splat(0.26);
    *visibility = Visibility::Inherited;
}
