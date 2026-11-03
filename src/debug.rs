use bevy::prelude::*;

use crate::{
    daynight::DayCycle,
    player::{Player, ViewMode},
    world::{section_index, ChunkPos, VoxelWorld},
};

#[derive(Resource)]
pub struct HudVisible(pub bool);

impl Default for HudVisible {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource, Default)]
pub struct DebugVisible(pub bool);

#[derive(Component)]
pub struct DebugOverlay;

#[derive(Component)]
pub struct DebugText;

#[derive(Resource, Default)]
pub struct FpsCounter {
    elapsed: f32,
    frames: u32,
    fps: f32,
}

pub fn spawn_debug_overlay(commands: &mut Commands, asset_server: &AssetServer) {
    let font: Handle<Font> = asset_server.load("fonts/mono.ttf");
    commands.spawn((
        DebugOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(8.0),
            top: px(8.0),
            padding: UiRect::all(px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.55)),
        Visibility::Hidden,
        GlobalZIndex(30),
        children![(
            DebugText,
            Text::new(""),
            TextFont {
                font: font.into(),
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}

pub fn toggle_hud(
    keys: Res<ButtonInput<KeyCode>>,
    mut hud: ResMut<HudVisible>,
    debug: Res<DebugVisible>,
    mut overlay: Query<&mut Visibility, With<DebugOverlay>>,
) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }
    hud.0 = !hud.0;
    let visible = hud.0 && debug.0;
    for mut visibility in &mut overlay {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub fn sync_hud_visibility(
    hud: Res<HudVisible>,
    debug: Res<DebugVisible>,
    mut overlay: Query<&mut Visibility, With<DebugOverlay>>,
) {
    if !hud.is_changed() && !debug.is_changed() {
        return;
    }
    let visible = hud.0 && debug.0;
    for mut visibility in &mut overlay {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub fn toggle_debug(
    keys: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<DebugVisible>,
    hud: Res<HudVisible>,
    mut overlay: Query<&mut Visibility, With<DebugOverlay>>,
) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    debug.0 = !debug.0;
    let visible = debug.0 && hud.0;
    for mut visibility in &mut overlay {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub fn update_debug(
    time: Res<Time>,
    mut fps: ResMut<FpsCounter>,
    cycle: Res<DayCycle>,
    world: Res<VoxelWorld>,
    view: Res<ViewMode>,
    player: Query<&Player, With<Camera3d>>,
    overlay: Query<&Visibility, With<DebugOverlay>>,
    mut text: Query<&mut Text, With<DebugText>>,
) {
    if overlay
        .single()
        .map(|visibility| *visibility == Visibility::Hidden)
        .unwrap_or(true)
    {
        return;
    }
    let Ok(player) = player.single() else {
        return;
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };

    let eye = player.body;
    let feet = player.feet.y;
    let block_x = eye.x.floor() as i32;
    let block_z = eye.z.floor() as i32;
    let chunk = ChunkPos::from_block(block_x, block_z);
    let section = section_index(feet.floor() as i32)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());

    let forward = Vec2::new(-player.yaw.sin(), -player.yaw.cos());
    let facing = if forward.x.abs() > forward.y.abs() {
        if forward.x > 0.0 { "Đông" } else { "Tây" }
    } else if forward.y < 0.0 {
        "Bắc"
    } else {
        "Nam"
    };

    let hours = (cycle.time * 24.0 + 6.0).rem_euclid(24.0);
    let clock = format!("{:02.0}:{:02.0}", hours, (hours.fract() * 60.0));
    fps.elapsed += time.delta_secs();
    fps.frames += 1;
    if fps.elapsed >= 0.5 {
        fps.fps = fps.frames as f32 / fps.elapsed.max(0.0001);
        fps.elapsed = 0.0;
        fps.frames = 0;
    }
    let view_mode = match view.mode {
        1 => "góc nhìn thứ 3",
        2 => "góc nhìn Dungeons",
        _ => "góc nhìn thứ 1",
    };
    let biome = world
        .column_at(block_x, block_z)
        .map(|column| crate::blocks::info(column.biome).name)
        .unwrap_or("-");

    text.0 = format!(
        "Elosium\n\
         XYZ: {:.2} / {:.2} / {:.2}\n\
         Mắt: {:.2}  Chân: {:.2}\n\
         Chunk: {} {}  Section: {}\n\
         Hướng: {}  yaw {:.1}°  pitch {:.1}°\n\
         Biome: {}\n\
         FPS: {:.0}  Giờ: {}\n\
         Chunk đã tải: {}\n\
         {}",
        eye.x,
        eye.y,
        eye.z,
        eye.y,
        feet,
        chunk.x,
        chunk.z,
        section,
        facing,
        player.yaw.to_degrees(),
        player.pitch.to_degrees(),
        biome,
        fps.fps,
        clock,
        world.loaded_chunks().count(),
        view_mode,
    );
}
