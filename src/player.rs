use bevy::{
    input::{keyboard::KeyCode, mouse::AccumulatedMouseMotion},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, Window},
};

use crate::{
    chat::ChatState,
    hud::Vitals,
    menu::{
        bound_key, MenuState, Settings, ACTION_BACK, ACTION_FORWARD, ACTION_JUMP, ACTION_LEFT,
        ACTION_RIGHT, ACTION_SNEAK, ACTION_SPRINT,
    },
    reflection::ReflectionCamera,
    world::{VoxelWorld, MIN_Y},
};

fn direction_zero() -> Vec3 {
    Vec3::ZERO
}

pub const EYE_HEIGHT: f32 = 1.62;
pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
const EYE_CROUCHING: f32 = 1.27;
const EYE_SWIMMING: f32 = 0.4;
const HEIGHT_CROUCHING: f32 = 1.5;
const HEIGHT_SWIMMING: f32 = 0.6;

const TICK_SECONDS: f32 = 0.05;
const WALK_SPEED: f32 = 0.1;
const SPRINT_BOOST: f32 = 1.3;
const JUMP_POWER: f32 = 0.42;
const GRAVITY: f32 = 0.08;
const WATER_INPUT: f32 = 0.02;
const WATER_DRAG: f32 = 0.8;
const SPRINT_WATER_DRAG: f32 = 0.9;
const WATER_SINK: f32 = GRAVITY / 16.0;
const BLOCK_FRICTION: f32 = 0.6;
const SNEAK_MULTIPLIER: f32 = 0.3;
const FLY_SPEED: f32 = 0.05;
const FLY_VERTICAL: f32 = FLY_SPEED * 3.0;
const MAX_UP_STEP: f32 = 0.6;
const SKIN: f32 = 0.001;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PlayerPose {
    #[default]
    Standing,
    Crouching,
    Swimming,
}

impl PlayerPose {
    pub fn eye_height(self) -> f32 {
        match self {
            PlayerPose::Standing => EYE_HEIGHT,
            PlayerPose::Crouching => EYE_CROUCHING,
            PlayerPose::Swimming => EYE_SWIMMING,
        }
    }

    pub fn height(self) -> f32 {
        match self {
            PlayerPose::Standing => PLAYER_HEIGHT,
            PlayerPose::Crouching => HEIGHT_CROUCHING,
            PlayerPose::Swimming => HEIGHT_SWIMMING,
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub uuid: u128,
    pub name: String,
    pub body: Vec3,
    pub feet: Vec3,
    pub prev_feet: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    pub swinging: bool,
    pub swing: f32,
    pub body_yaw: f32,
    pub sneaking: bool,
    pub sprinting: bool,
    pub swimming: bool,
    pub pose: PlayerPose,
    pub tick_accumulator: f32,
    pub no_jump_delay: i32,
    pub sprint_trigger: i32,
    pub prev_forward: f32,
    pub tick_speed: f32,
    pub flying: bool,
    pub jump_trigger_time: i32,
    pub prev_jump: bool,
    pub eye_height_current: f32,
    pub eye_height_prev: f32,
    pub render_feet: Vec3,
    pub fall_distance: f32,
}

impl Player {
    pub fn eye_height(&self) -> f32 {
        self.pose.eye_height()
    }

    pub fn speed_mps(&self) -> f32 {
        self.tick_speed * 20.0
    }
}

#[derive(Clone, Copy, Default)]
struct MoveInput {
    forward: f32,
    strafe: f32,
    jump: bool,
    sneak: bool,
    sprint: bool,
}

#[derive(Clone, Copy, Default)]
struct FluidState {
    height: f32,
    in_water: bool,
}

#[derive(Clone, Copy, Default)]
struct MoveResult {
    on_ground: bool,
    horizontal_collision: bool,
    vertical_collision: bool,
}

pub fn update_body_yaw(
    time: Res<Time>,
    view: Res<ViewMode>,
    mut player: Query<&mut Player, With<Camera3d>>,
) {
    let Ok(mut player) = player.single_mut() else {
        return;
    };
    let speed = player.speed_mps();
    let target = if view.mode == 2 || speed <= 0.7 {
        player.yaw
    } else {
        (-player.velocity.x).atan2(-player.velocity.z)
    };
    let mut difference = (target - player.body_yaw + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let step = (time.delta_secs() * 9.0).min(1.0);
    difference *= step;
    player.body_yaw = (player.body_yaw + difference + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
}

pub fn advance_swing(
    time: Res<Time>,
    mut player: Query<&mut Player, With<Camera3d>>,
) {
    let Ok(mut player) = player.single_mut() else {
        return;
    };
    if !player.swinging {
        return;
    }
    player.swing += time.delta_secs() / 0.32;
    if player.swing >= 1.0 {
        player.swing = 0.0;
        player.swinging = false;
    }
}

#[derive(Resource, Default)]
pub struct ViewMode {
    pub mode: u8,
    pub offset: Vec3,
    pub blend: f32,
    pub distance: f32,
    pub orbit_yaw: f32,
    pub orbit_pitch: f32,
}

pub fn toggle_view(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatState>,
    mut view: ResMut<ViewMode>,
    mut player: Query<&mut Player, With<Camera3d>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if chat.open {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyV) {
        return;
    }
    view.mode = (view.mode + 1) % 3;
    match view.mode {
        2 => {
            cursor.visible = true;
            cursor.grab_mode = CursorGrabMode::None;
            if let Ok(mut player) = player.single_mut() {
                view.orbit_yaw = player.yaw;
                view.orbit_pitch = -0.95;
                player.pitch = -0.2;
            }
        }
        0 => {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
        }
        _ => {}
    }
}

pub fn orbit_drag_control(
    menu: Res<MenuState>,
    chat: Res<ChatState>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut view: ResMut<ViewMode>,
) {
    if menu.open || chat.open || view.mode != 2 {
        return;
    }
    if mouse.pressed(MouseButton::Middle) {
        view.orbit_yaw -= motion.delta.x * 0.005;
        view.orbit_pitch =
            (view.orbit_pitch - motion.delta.y * 0.005).clamp(-1.35, -0.35);
    }
}

pub fn update_head_aim(
    menu: Res<MenuState>,
    chat: Res<ChatState>,
    view: Res<ViewMode>,
    window: Single<&Window>,
    world: Res<VoxelWorld>,
    camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<ReflectionCamera>)>,
    mut players: Query<&mut Player, With<Camera3d>>,
) {
    if menu.open || chat.open || view.mode != 2 {
        return;
    }
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_global)) = camera.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_global, cursor) else {
        return;
    };
    let target = crate::interaction::raycast_point(&world, ray.origin, *ray.direction, 80.0)
        .unwrap_or(ray.origin + *ray.direction * 60.0);
    let delta = target - player.body;
    let length = delta.length();
    if length < 0.05 {
        return;
    }
    player.yaw = (-delta.x).atan2(-delta.z);
    player.pitch = (delta.y / length).clamp(-0.999, 0.999).asin();
}

pub fn enforce_mode2_cursor(
    menu: Res<MenuState>,
    view: Res<ViewMode>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if !menu.open && view.mode == 2 {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

pub fn apply_camera_view(
    time: Res<Time>,
    mut view: ResMut<ViewMode>,
    world: Res<VoxelWorld>,
    mut camera: Query<(&mut Transform, &Player), With<Camera3d>>,
) {
    let Ok((mut transform, player)) = camera.single_mut() else {
        return;
    };
    let rotation =
        Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch);
    let center = if view.mode == 2 {
        player.render_feet + Vec3::Y * (player.pose.height() * 0.5)
    } else {
        player.body
    };
    let orbit_rotation = Quat::from_rotation_y(view.orbit_yaw)
        * Quat::from_rotation_x(view.orbit_pitch);
    let desired = match view.mode {
        1 => rotation * Vec3::new(0.0, 0.55, 2.8),
        2 => -(orbit_rotation * Vec3::NEG_Z) * 13.0,
        _ => Vec3::ZERO,
    };
    let ray_distance = |target: Vec3| {
        let distance = target.length();
        let direction = if distance > 0.0001 {
            target / distance
        } else {
            return (direction_zero(), 0.0);
        };
        let mut travelled = 0.0;
        while travelled < distance {
            travelled = (travelled + 0.15).min(distance);
            let point = center + direction * travelled;
            if world.is_solid(
                point.x.floor() as i32,
                point.y.floor() as i32,
                point.z.floor() as i32,
            ) {
                return (direction, (travelled - 0.3).max(0.0));
            }
        }
        (direction, distance)
    };
    let goal = if view.mode == 2 {
        desired
    } else {
        let (direction, allowed) = ray_distance(desired);
        direction * allowed
    };
    let step = (time.delta_secs() * 7.0).min(1.0);
    view.offset = view.offset.lerp(goal, step);
    view.distance = view.offset.length();
    view.blend = (view.distance / 0.5).min(1.0);
    transform.translation = center + view.offset;
    if view.mode == 2 {
        transform.look_at(center, Vec3::Y);
    } else {
        transform.rotation = rotation;
    }
}

#[derive(Resource, Clone, Copy)]
pub struct SpawnPoint(pub Vec3);

pub fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    menu: Res<MenuState>,
    chat: Res<ChatState>,
    view: Res<ViewMode>,
    mut cursor: Single<&mut CursorOptions>,
    mut player: Query<&mut Player, With<Camera3d>>,
) {
    if menu.open || chat.open || view.mode == 2 {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
        if let Ok(mut player) = player.single_mut() {
            player.swinging = true;
            player.swing = 0.0;
        }
    }
}

pub fn player_controller(
    time: Res<Time>,
    menu: Res<MenuState>,
    chat: Res<ChatState>,
    settings: Res<Settings>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions>,
    world: Res<VoxelWorld>,
    spawn: Res<SpawnPoint>,
    window: Single<&Window>,
    rules: Res<crate::frontend::GameRules>,
    mut vitals: ResMut<crate::hud::Vitals>,
    mut query: Query<&mut Player, With<Camera3d>>,
) {
    let Ok(mut player) = query.single_mut() else {
        return;
    };
    let delta_seconds = time.delta_secs().min(0.25);

    let frozen = std::env::var("ELOSIUM_FREEZE").is_ok();
    if frozen {
        player.velocity = Vec3::ZERO;
        player.feet = spawn.0;
        player.prev_feet = spawn.0;
        player.render_feet = spawn.0;
        player.pose = PlayerPose::Standing;
        player.flying = false;
        player.jump_trigger_time = 0;
        player.prev_jump = false;
        player.eye_height_current = EYE_HEIGHT;
        player.eye_height_prev = EYE_HEIGHT;
        player.body = spawn.0 + Vec3::Y * EYE_HEIGHT;
        return;
    }

    if cursor.grab_mode != CursorGrabMode::None && !menu.open && !chat.open {
        let sensitivity = 0.0022;
        player.yaw -= mouse_motion.delta.x * sensitivity;
        player.pitch =
            (player.pitch - mouse_motion.delta.y * sensitivity).clamp(-1.54, 1.54);
    }

    let raw = |vk: i32| window.focused && raw_key_down(vk);

    let mut forward_input = 0.0;
    let mut strafe_input = 0.0;
    if std::env::var("ELOSIUM_WALK_TEST").is_ok() {
        forward_input += 1.0;
    }
    if keys.pressed(bound_key(&settings.keys, ACTION_FORWARD)) || raw(VK_W) {
        forward_input += 1.0;
    }
    if keys.pressed(bound_key(&settings.keys, ACTION_BACK)) || raw(VK_S) {
        forward_input -= 1.0;
    }
    if keys.pressed(bound_key(&settings.keys, ACTION_RIGHT)) || raw(VK_D) {
        strafe_input += 1.0;
    }
    if keys.pressed(bound_key(&settings.keys, ACTION_LEFT)) || raw(VK_A) {
        strafe_input -= 1.0;
    }

    let sprint_key = keys.pressed(bound_key(&settings.keys, ACTION_SPRINT))
        || raw(VK_SHIFT)
        || std::env::var("ELOSIUM_RUN_TEST").is_ok();
    let sneaking = keys.pressed(bound_key(&settings.keys, ACTION_SNEAK))
        || keys.pressed(KeyCode::ControlRight)
        || raw(VK_CONTROL)
        || std::env::var("ELOSIUM_SNEAK_INPUT").is_ok();
    player.sneaking = sneaking;
    let jump = !menu.open
        && !chat.open
        && (keys.pressed(bound_key(&settings.keys, ACTION_JUMP))
            || raw(VK_SPACE)
            || std::env::var("ELOSIUM_JUMP_INPUT").is_ok());
    if menu.open || chat.open {
        forward_input = 0.0;
        strafe_input = 0.0;
    }

    let scale = if sneaking { SNEAK_MULTIPLIER } else { 1.0 };
    let input = MoveInput {
        forward: forward_input * scale,
        strafe: strafe_input * scale,
        jump,
        sneak: sneaking,
        sprint: sprint_key,
    };

    player.tick_accumulator = (player.tick_accumulator + delta_seconds).min(0.25);
    while player.tick_accumulator >= TICK_SECONDS {
        player.tick_accumulator -= TICK_SECONDS;
        player.prev_feet = player.feet;
        simulate_tick(
            &mut player,
            &world,
            &input,
            spawn.0,
            rules.gamemode,
            &mut vitals,
        );
    }

    let alpha = (player.tick_accumulator / TICK_SECONDS).clamp(0.0, 1.0);
    let feet = player.prev_feet.lerp(player.feet, alpha);
    let eye = player.eye_height_prev + (player.eye_height_current - player.eye_height_prev) * alpha;
    player.render_feet = feet;
    player.body = feet + Vec3::Y * eye;
}

fn simulate_tick(
    player: &mut Player,
    world: &VoxelWorld,
    input: &MoveInput,
    spawn: Vec3,
    gamemode: u8,
    vitals: &mut Vitals,
) {
    let mut feet = player.feet;
    let start = feet;
    let was_on_ground = player.on_ground;
    let mut velocity = player.velocity;

    if player.no_jump_delay > 0 {
        player.no_jump_delay -= 1;
    }
    if player.jump_trigger_time > 0 {
        player.jump_trigger_time -= 1;
    }
    if player.sprint_trigger > 0 {
        player.sprint_trigger -= 1;
    }

    let fluid = fluid_state(world, feet, player.pose);
    let eye_block = world.get_block(
        feet.x.floor() as i32,
        (feet.y + player.eye_height()).floor() as i32,
        feet.z.floor() as i32,
    );
    let under_water = fluid.in_water && eye_block.is_water();

    let has_enough = if under_water {
        input.forward > 1.0e-5
    } else {
        input.forward >= 0.8
    };
    let prev_enough = if under_water {
        player.prev_forward > 1.0e-5
    } else {
        player.prev_forward >= 0.8
    };
    if input.sneak {
        player.sprint_trigger = 0;
    }
    let can_start = !player.sprinting && has_enough;
    if (player.on_ground || under_water) && !input.sneak && !prev_enough && can_start {
        if player.sprint_trigger <= 0 && !input.sprint {
            player.sprint_trigger = 7;
        } else {
            player.sprinting = true;
        }
    }
    if (!fluid.in_water || under_water) && can_start && input.sprint {
        player.sprinting = true;
    }
    if player.sprinting {
        let no_forward = input.forward <= 1.0e-5;
        let collide_stop = no_forward
            || player.horizontal_collision
            || (fluid.in_water && !under_water);
        if player.swimming {
            if (!player.on_ground && !input.sneak && no_forward) || !fluid.in_water {
                player.sprinting = false;
            }
        } else if collide_stop {
            player.sprinting = false;
        }
    }
    player.prev_forward = input.forward;

    let can_fly = gamemode == 1 || gamemode == 3;
    if !can_fly {
        player.flying = false;
    }
    let fresh_jump = input.jump && !player.prev_jump;
    if fresh_jump && !player.swimming && can_fly {
        if player.jump_trigger_time == 0 {
            player.jump_trigger_time = 7;
        } else {
            player.flying = !player.flying;
            player.jump_trigger_time = 0;
        }
    }
    if gamemode == 3 {
        player.flying = true;
    }
    player.prev_jump = input.jump;

    if fluid.in_water && input.sneak {
        velocity.y -= 0.04;
    }

    let foot_block_water = world
        .get_block(
            feet.x.floor() as i32,
            feet.y.floor() as i32,
            feet.z.floor() as i32,
        )
        .is_water();
    if player.flying {
        player.swimming = false;
    } else if player.swimming {
        player.swimming = player.sprinting && fluid.in_water;
    } else {
        player.swimming = player.sprinting && under_water && foot_block_water;
    }

    if input.jump {
        let in_water = fluid.in_water && fluid.height > 0.0;
        let threshold = if player.eye_height() < 0.4 { 0.0 } else { 0.4 };
        if !in_water || (player.on_ground && fluid.height <= threshold) {
            if (player.on_ground || (in_water && fluid.height <= threshold))
                && player.no_jump_delay == 0
            {
                velocity.y = JUMP_POWER;
                if player.sprinting {
                    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
                    velocity.x += forward.x * 0.2;
                    velocity.z += forward.z * 0.2;
                }
                player.no_jump_delay = 10;
            }
        } else {
            velocity.y += 0.04;
        }
    } else {
        player.no_jump_delay = 0;
    }

    let input_dir = input_direction(
        player.yaw,
        input.forward * 0.98,
        input.strafe * 0.98,
    );

    if player.swimming {
        let look_y = player.pitch.sin();
        let boost = if look_y < -0.2 { 0.085 } else { 0.06 };
        let head_water = world
            .get_block(
                feet.x.floor() as i32,
                (feet.y + 0.9).floor() as i32,
                feet.z.floor() as i32,
            )
            .is_water();
        if look_y <= 0.0 || input.jump || head_water {
            velocity.y += (look_y - velocity.y) * boost;
        }
    }

    if player.flying {
        let vertical = (input.jump as i32 - input.sneak as i32) as f32;
        if vertical != 0.0 {
            velocity.y += vertical * FLY_VERTICAL;
        }
    }

    let water = fluid_state(world, feet, player.pose);
    if player.flying {
        let saved_y = velocity.y;
        let speed = if player.sprinting {
            FLY_SPEED * 2.0
        } else {
            FLY_SPEED
        };
        velocity += input_dir * speed;
        let delta = velocity;
        if gamemode == 3 {
            feet += delta;
            velocity.x *= 0.91;
            velocity.z *= 0.91;
            velocity.y = saved_y * 0.6;
            player.on_ground = false;
            player.horizontal_collision = false;
        } else {
            let (result, moved) = move_entity(world, &mut feet, delta, player.pose);
            if moved.x != delta.x {
                velocity.x = 0.0;
            }
            if moved.z != delta.z {
                velocity.z = 0.0;
            }
            let keep = if player.on_ground {
                BLOCK_FRICTION * 0.91
            } else {
                0.91
            };
            velocity.x *= keep;
            velocity.z *= keep;
            velocity.y = saved_y * 0.6;
            player.on_ground = result.on_ground;
            player.horizontal_collision = result.horizontal_collision;
            if player.on_ground {
                player.flying = false;
            }
        }
    } else if water.in_water {
        let drag = if player.sprinting {
            SPRINT_WATER_DRAG
        } else {
            WATER_DRAG
        };
        velocity += input_dir * WATER_INPUT;
        let delta = velocity;
        let (result, moved) = move_entity(world, &mut feet, delta, player.pose);
        if moved.x != delta.x {
            velocity.x = 0.0;
        }
        if moved.z != delta.z {
            velocity.z = 0.0;
        }
        if result.vertical_collision {
            velocity.y = 0.0;
        }
        velocity.x *= drag;
        velocity.z *= drag;
        velocity.y *= WATER_DRAG;
        if !player.sprinting {
            velocity.y -= WATER_SINK;
        }
        if result.horizontal_collision
            && is_free(
                world,
                feet + Vec3::new(velocity.x, MAX_UP_STEP, velocity.z),
                player.pose,
            )
        {
            velocity.y = 0.3;
        }
        player.on_ground = result.on_ground;
        player.horizontal_collision = result.horizontal_collision;
    } else {
        let speed = if player.sprinting {
            WALK_SPEED * SPRINT_BOOST
        } else {
            WALK_SPEED
        };
        let friction = BLOCK_FRICTION;
        let acceleration = if player.on_ground {
            speed * (0.216_000_02 / (friction * friction * friction))
        } else {
            0.02
        };
        velocity += input_dir * acceleration;
        let delta = if player.sneaking && player.on_ground && velocity.y <= 0.0 {
            back_off_edge(world, feet, player.pose, velocity)
        } else {
            velocity
        };
        let (result, moved) = move_entity(world, &mut feet, delta, player.pose);
        if moved.x != delta.x {
            velocity.x = 0.0;
        }
        if moved.z != delta.z {
            velocity.z = 0.0;
        }
        if result.vertical_collision {
            velocity.y = 0.0;
        }
        let keep = if player.on_ground {
            friction * 0.91
        } else {
            0.91
        };
        velocity.x *= keep;
        velocity.z *= keep;
        velocity.y = (velocity.y - GRAVITY) * 0.98;
        player.on_ground = result.on_ground;
        player.horizontal_collision = result.horizontal_collision;
    }

    update_pose(world, feet, player);

    player.eye_height_prev = player.eye_height_current;
    player.eye_height_current +=
        (player.pose.eye_height() - player.eye_height_current) * 0.5;

    let moved = feet - start;
    player.tick_speed = Vec2::new(moved.x, moved.z).length();
    player.feet = feet;
    player.velocity = velocity;

    if gamemode == 0 {
        if fluid.in_water {
            player.fall_distance = 0.0;
        } else if player.on_ground {
            if !was_on_ground && player.fall_distance > 3.0 {
                let damage = (player.fall_distance - 3.0).ceil();
                vitals.health = (vitals.health - damage).max(0.0);
            }
            player.fall_distance = 0.0;
        } else if !player.flying && player.velocity.y < 0.0 {
            player.fall_distance += (start.y - feet.y).max(0.0);
        }
        if vitals.health <= 0.0 {
            player.feet = spawn;
            player.prev_feet = spawn;
            player.render_feet = spawn;
            player.velocity = Vec3::ZERO;
            player.on_ground = false;
            player.pose = PlayerPose::Standing;
            player.tick_speed = 0.0;
            player.flying = false;
            player.fall_distance = 0.0;
            player.jump_trigger_time = 0;
            player.prev_jump = false;
            player.eye_height_current = EYE_HEIGHT;
            player.eye_height_prev = EYE_HEIGHT;
            vitals.health = 20.0;
        }
    } else {
        player.fall_distance = 0.0;
    }

    if feet.y < MIN_Y as f32 + 1.0 {
        player.feet = spawn;
        player.prev_feet = spawn;
        player.velocity = Vec3::ZERO;
        player.on_ground = false;
        player.pose = PlayerPose::Standing;
        player.tick_speed = 0.0;
        player.flying = false;
        player.jump_trigger_time = 0;
        player.prev_jump = false;
        player.eye_height_current = EYE_HEIGHT;
        player.eye_height_prev = EYE_HEIGHT;
    }
}

fn input_direction(yaw: f32, forward: f32, strafe: f32) -> Vec3 {
    let forward_dir = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(-forward_dir.z, 0.0, forward_dir.x);
    let mut wish = forward_dir * forward + right * strafe;
    let length_squared = wish.length_squared();
    if length_squared > 1.0 {
        wish /= length_squared.sqrt();
    }
    wish
}

fn fluid_state(world: &VoxelWorld, feet: Vec3, pose: PlayerPose) -> FluidState {
    let half = PLAYER_WIDTH * 0.5;
    let min = Vec3::new(feet.x - half, feet.y, feet.z - half) + Vec3::splat(SKIN);
    let max = Vec3::new(feet.x + half, feet.y + pose.height(), feet.z + half)
        - Vec3::splat(SKIN);
    let mut state = FluidState::default();
    for x in min.x.floor() as i32..=max.x.floor() as i32 {
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                if !world.get_block(x, y, z).is_water() {
                    continue;
                }
                let level = world.water_level(x, y, z).unwrap_or(0) as f32;
                let surface = y as f32 + 0.875 - level * 0.1;
                if surface >= min.y {
                    state.in_water = true;
                    state.height = state.height.max(surface - min.y);
                }
            }
        }
    }
    state
}

fn pose_collides(world: &VoxelWorld, feet: Vec3, pose: PlayerPose) -> bool {
    let half = PLAYER_WIDTH * 0.5;
    let min = Vec3::new(feet.x - half, feet.y, feet.z - half);
    let max = Vec3::new(feet.x + half, feet.y + pose.height(), feet.z + half);
    box_collides(world, min, max)
}

fn box_collides(world: &VoxelWorld, min: Vec3, max: Vec3) -> bool {
    for x in (min.x + SKIN).floor() as i32..=(max.x - SKIN).floor() as i32 {
        for y in (min.y + SKIN).floor() as i32..=(max.y - SKIN).floor() as i32 {
            for z in (min.z + SKIN).floor() as i32..=(max.z - SKIN).floor() as i32 {
                if world.is_solid(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

fn can_enter_pose(world: &VoxelWorld, feet: Vec3, pose: PlayerPose) -> bool {
    !pose_collides(world, feet, pose)
}

fn is_free(world: &VoxelWorld, feet: Vec3, pose: PlayerPose) -> bool {
    if pose_collides(world, feet, pose) {
        return false;
    }
    let half = PLAYER_WIDTH * 0.5;
    let min = Vec3::new(feet.x - half, feet.y, feet.z - half) + Vec3::splat(SKIN);
    let max = Vec3::new(feet.x + half, feet.y + pose.height(), feet.z + half)
        - Vec3::splat(SKIN);
    for x in min.x.floor() as i32..=max.x.floor() as i32 {
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                if world.get_block(x, y, z).is_water() {
                    return false;
                }
            }
        }
    }
    true
}

fn move_entity(
    world: &VoxelWorld,
    feet: &mut Vec3,
    delta: Vec3,
    pose: PlayerPose,
) -> (MoveResult, Vec3) {
    let mut moved = Vec3::ZERO;
    moved.y = move_axis(world, feet, pose, delta.y, 1);
    let vertical_collision = moved.y != delta.y;
    let x_first = delta.x.abs() >= delta.z.abs();
    let first = if x_first { 0 } else { 2 };
    let second = if x_first { 2 } else { 0 };
    let first_amount = axis_value(delta, first);
    let second_amount = axis_value(delta, second);
    let first_moved = move_axis(world, feet, pose, first_amount, first);
    let second_moved = move_axis(world, feet, pose, second_amount, second);
    set_axis(&mut moved, first, first_moved);
    set_axis(&mut moved, second, second_moved);

    let result = MoveResult {
        on_ground: vertical_collision && delta.y < 0.0,
        horizontal_collision: first_moved != first_amount || second_moved != second_amount,
        vertical_collision,
    };
    (result, moved)
}

fn move_axis(
    world: &VoxelWorld,
    feet: &mut Vec3,
    pose: PlayerPose,
    amount: f32,
    axis: usize,
) -> f32 {
    if amount == 0.0 {
        return 0.0;
    }
    let start = *feet;
    let mut probe = start;
    set_axis(&mut probe, axis, axis_value(start, axis) + amount);
    if !pose_collides(world, probe, pose) {
        *feet = probe;
        return amount;
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..24 {
        let mid = (low + high) * 0.5;
        let mut point = start;
        set_axis(&mut point, axis, axis_value(start, axis) + amount * mid);
        if pose_collides(world, point, pose) {
            high = mid;
        } else {
            low = mid;
        }
    }
    let clipped = amount * low;
    let mut end = start;
    set_axis(&mut end, axis, axis_value(start, axis) + clipped);
    *feet = end;
    clipped
}

fn back_off_edge(world: &VoxelWorld, feet: Vec3, pose: PlayerPose, delta: Vec3) -> Vec3 {
    let mut dx = delta.x;
    let mut dz = delta.z;
    let step = 0.05;
    while dx != 0.0
        && !pose_collides(world, feet + Vec3::new(dx, -MAX_UP_STEP, 0.0), pose)
    {
        dx = shrink(dx, step);
    }
    while dz != 0.0
        && !pose_collides(world, feet + Vec3::new(0.0, -MAX_UP_STEP, dz), pose)
    {
        dz = shrink(dz, step);
    }
    while dx != 0.0
        && dz != 0.0
        && !pose_collides(world, feet + Vec3::new(dx, -MAX_UP_STEP, dz), pose)
    {
        dx = shrink(dx, step);
        dz = shrink(dz, step);
    }
    Vec3::new(dx, delta.y, dz)
}

fn shrink(value: f32, step: f32) -> f32 {
    if value < step && value >= -step {
        0.0
    } else if value > 0.0 {
        value - step
    } else {
        value + step
    }
}

fn update_pose(world: &VoxelWorld, feet: Vec3, player: &mut Player) {
    if !can_enter_pose(world, feet, PlayerPose::Swimming) {
        return;
    }
    let mut pose = if player.swimming {
        PlayerPose::Swimming
    } else if player.sneaking && !player.flying {
        PlayerPose::Crouching
    } else {
        PlayerPose::Standing
    };
    if !can_enter_pose(world, feet, pose) {
        pose = if can_enter_pose(world, feet, PlayerPose::Crouching) {
            PlayerPose::Crouching
        } else {
            PlayerPose::Swimming
        };
    }
    player.pose = pose;
}

fn axis_value(value: Vec3, axis: usize) -> f32 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

fn set_axis(value: &mut Vec3, axis: usize, amount: f32) {
    match axis {
        0 => value.x = amount,
        1 => value.y = amount,
        _ => value.z = amount,
    }
}

const VK_A: i32 = 0x41;
const VK_D: i32 = 0x44;
const VK_S: i32 = 0x53;
const VK_W: i32 = 0x57;
const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_SPACE: i32 = 0x20;

#[cfg(windows)]
fn raw_key_down(vk: i32) -> bool {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk) < 0 }
}

#[cfg(not(windows))]
fn raw_key_down(_vk: i32) -> bool {
    false
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PlayerIdentity {
    uuid: String,
    name: String,
}

pub fn load_identity() -> (u128, String) {
    if let Ok(text) = std::fs::read_to_string("config/player.json") {
        if let Ok(identity) = serde_json::from_str::<PlayerIdentity>(&text) {
            if let Ok(uuid) = u128::from_str_radix(&identity.uuid, 16) {
                if !identity.name.trim().is_empty() {
                    return (uuid, identity.name);
                }
            }
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(1);
    let uuid = nanos ^ (nanos << 23) ^ 0x9E37_79B9_7F4A_7C15;
    let identity = PlayerIdentity {
        uuid: format!("{uuid:032x}"),
        name: "Player".to_string(),
    };
    if let Ok(text) = serde_json::to_string_pretty(&identity) {
        std::fs::create_dir_all("config").ok();
        std::fs::write("config/player.json", text).ok();
    }
    (uuid, identity.name)
}
