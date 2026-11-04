use bevy::{
    input::{
        keyboard::{Key, KeyCode, KeyboardInput},
        ButtonState,
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    daynight::DayCycle,
    frontend::{Screen, ScreenState},
    menu::{bound_key, MenuState, Settings, ACTION_CHAT},
    player::Player,
};

const LINES: usize = 8;
const BASE_FONT: f32 = 15.0;

#[derive(Clone)]
pub struct ChatLine {
    text: String,
    age: f32,
}

#[derive(Resource)]
pub struct ChatState {
    pub open: bool,
    pub closed_esc: bool,
    input: String,
    lines: Vec<ChatLine>,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            open: false,
            closed_esc: false,
            input: String::new(),
            lines: Vec::new(),
        }
    }
}

#[derive(Component)]
struct ChatRoot;

#[derive(Component)]
struct ChatLineText(usize);

#[derive(Component)]
struct ChatLineBox(usize);

#[derive(Component)]
struct ChatInput;

#[derive(Component)]
struct ChatInputBox;

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatState>()
            .add_systems(Startup, spawn_chat_ui)
            .add_systems(Update, (chat_input, chat_render));
    }
}

fn spawn_chat_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/mono.ttf");
    if std::env::var("ELOSIUM_CHAT_TEST").is_ok() {
        let mut state = ChatState::default();
        state.push("<Player> Xin chào, đây là chat của Elosium.".to_string());
        state.push("Gõ T để mở chat, Enter để gửi, /help để xem lệnh.".to_string());
        if std::env::var("ELOSIUM_CHAT_TEST").ok().as_deref() == Some("2") {
            state.input = "awdkgfkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk".to_string();
        }
        state.open = true;
        commands.insert_resource(state);
    }
    commands
        .spawn((
            ChatRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                bottom: px(110),
                width: px(600),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            for index in 0..LINES {
                root.spawn((
                    ChatLineBox(index),
                    Node {
                        width: percent(100),
                        padding: UiRect::axes(px(8), px(3)),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                    Visibility::Hidden,
                ))
                .with_children(|line| {
                    line.spawn((
                        ChatLineText(index),
                        Node {
                            width: percent(100),
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(BASE_FONT),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout::new(Justify::Left, LineBreak::AnyCharacter),
                    ));
                });
            }
            root.spawn((
                ChatInputBox,
                Node {
                    width: percent(100),
                    padding: UiRect::axes(px(8), px(5)),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
                Visibility::Hidden,
            ))
            .with_children(|line| {
                line.spawn((
                    ChatInput,
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    Text::new(""),
                    TextFont {
                        font: font.into(),
                        font_size: FontSize::Px(BASE_FONT),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::new(Justify::Left, LineBreak::AnyCharacter),
                ));
            });
        });
}

fn chat_input(
    mut state: ResMut<ChatState>,
    settings: Res<Settings>,
    screen: Res<ScreenState>,
    menu: Res<MenuState>,
    creative: Res<crate::creative::CreativeState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut cursor: Single<&mut CursorOptions>,
    players: Query<&Player, With<Camera3d>>,
    mut day: ResMut<DayCycle>,
    mut clones: MessageWriter<crate::player_model::SpawnClone>,
) {
    let playing = screen.screen == Screen::Playing;
    state.closed_esc = false;
    if !playing {
        if state.open {
            state.open = false;
            if std::env::var("ELOSIUM_CHAT_TEST").is_err() {
                state.input.clear();
            }
        }
        return;
    }

    if !state.open
        && !state.input.is_empty()
        && std::env::var("ELOSIUM_CHAT_TEST").ok().as_deref() == Some("2")
    {
        state.open = true;
    }

    if !state.open {
        if !menu.open
            && !creative.open
            && keys.just_pressed(bound_key(&settings.keys, ACTION_CHAT))
        {
            state.open = true;
            cursor.visible = true;
            cursor.grab_mode = CursorGrabMode::None;
        }
        for _ in keyboard.read() {}
        return;
    }

    let (name, uuid) = players
        .single()
        .map(|player| (player.name.clone(), player.uuid))
        .unwrap_or_else(|_| ("Player".to_string(), 0));
    let mut close = false;
    for event in keyboard.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Character(text) => {
                for ch in text.chars() {
                    if !ch.is_control() && state.input.chars().count() < 200 {
                        state.input.push(ch);
                    }
                }
            }
            Key::Space => state.input.push(' '),
            Key::Backspace => {
                state.input.pop();
            }
            Key::Enter => {
                let message = state.input.trim().to_string();
                state.input.clear();
                if !message.is_empty() {
                    if message == "/clone" || message.starts_with("/clone ") {
                        let clone = message
                            .strip_prefix("/clone")
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        let clone = if clone.is_empty() {
                            "Player2".to_string()
                        } else {
                            clone
                        };
                        clones.write(crate::player_model::SpawnClone { name: clone.clone() });
                        state.push(format!("Đã tạo bản sao người chơi: {clone}"));
                    } else if message == "/whoami" {
                        state.push(format!("{name} · {uuid:032x}"));
                    } else if message.starts_with('/') {
                        state.push(command_reply(&message, &mut day));
                    } else {
                        state.push(format!("<{name}> {message}"));
                    }
                }
                close = true;
            }
            Key::Escape => {
                state.input.clear();
                state.closed_esc = true;
                close = true;
            }
            _ => {}
        }
    }
    if close {
        state.open = false;
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    }
}

fn command_reply(message: &str, day: &mut DayCycle) -> String {
    let mut parts = message.split_whitespace();
    match (parts.next(), parts.next(), parts.next()) {
        (Some("/help"), _, _) => {
            "Lệnh: /help · /time set day|night · /time <số> · /clone [tên] · /whoami".to_string()
        }
        (Some("/time"), Some("set"), Some(value)) => match value {
            "day" => {
                day.time = 0.18;
                "Đã đặt thời gian ban ngày".to_string()
            }
            "night" => {
                day.time = 0.72;
                "Đã đặt thời gian ban đêm".to_string()
            }
            _ => "Dùng: /time set day|night".to_string(),
        },
        (Some("/time"), Some(value), None) => match value.parse::<f32>() {
            Ok(time) => {
                day.time = time.rem_euclid(1.0);
                format!("Đã đặt thời gian {:.2}", day.time)
            }
            Err(_) => "Dùng: /time <0..1>".to_string(),
        },
        _ => format!("Không rõ lệnh: {message} (thử /help)"),
    }
}

impl ChatState {
    fn push(&mut self, text: String) {
        self.lines.push(ChatLine { text, age: 0.0 });
        if self.lines.len() > 40 {
            let drain = self.lines.len() - 40;
            self.lines.drain(0..drain);
        }
    }
}

fn chat_render(
    time: Res<Time>,
    mut state: ResMut<ChatState>,
    settings: Res<Settings>,
    screen: Res<ScreenState>,
    menu: Res<MenuState>,
    mut root: Query<&mut Visibility, With<ChatRoot>>,
    mut boxes: Query<
        (&ChatLineBox, &mut Visibility, &mut BackgroundColor),
        (Without<ChatRoot>, Without<ChatInputBox>),
    >,
    mut texts: Query<
        (&ChatLineText, &mut Text, &mut TextFont, &mut TextColor),
        (Without<ChatInput>, Without<ChatRoot>),
    >,
    mut input_text: Query<
        (&mut Text, &mut TextFont),
        (With<ChatInput>, Without<ChatLineText>, Without<ChatRoot>),
    >,
    mut input_box: Query<
        (&mut Visibility, &mut BackgroundColor),
        (With<ChatInputBox>, Without<ChatLineBox>, Without<ChatRoot>),
    >,
) {
    let visible = screen.screen == Screen::Playing && !menu.open;
    for mut visibility in &mut root {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }

    let fade = settings.chat_fade.max(1.0);
    let opacity = settings.chat_opacity.clamp(0.0, 1.0);
    let scale = settings.chat_scale.clamp(0.7, 1.8);
    for line in &mut state.lines {
        line.age += time.delta_secs();
    }
    if !state.open {
        state.lines.retain(|line| line.age < fade + 1.0);
    }

    let count = state.lines.len();
    let offset = count.saturating_sub(LINES);
    for (index, mut visibility, mut background) in &mut boxes {
        let slot = offset + index.0;
        if slot < count {
            let line = &state.lines[slot];
            let alpha = if state.open {
                1.0
            } else {
                (fade + 1.0 - line.age).clamp(0.0, 1.0) * line.age.min(1.0)
            };
            *visibility = if alpha > 0.01 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            background.0 = Color::srgba(0.0, 0.0, 0.0, opacity * alpha);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    for (label, mut text, mut font, mut color) in &mut texts {
        let slot = offset + label.0;
        if slot < count {
            let line = &state.lines[slot];
            text.0 = line.text.clone();
            font.font_size = FontSize::Px(BASE_FONT * scale);
            let alpha = if state.open {
                1.0
            } else {
                (fade + 1.0 - line.age).clamp(0.0, 1.0) * line.age.min(1.0)
            };
            color.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
        } else {
            text.0.clear();
        }
    }
    for (mut text, mut font) in &mut input_text {
        text.0 = if state.open {
            format!("{}_", state.input)
        } else {
            String::new()
        };
        font.font_size = FontSize::Px(BASE_FONT * scale);
    }
    for (mut visibility, mut background) in &mut input_box {
        *visibility = if state.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        background.0 = Color::srgba(0.0, 0.0, 0.0, (opacity + 0.2).min(0.95));
    }
}
