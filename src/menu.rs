use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use bevy::{
    anti_alias::fxaa::Fxaa,
    app::AppExit,
    ecs::hierarchy::ChildSpawnerCommands,
    input::{
        keyboard::KeyCode,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, VolumetricFog},
    picking::hover::Hovered,
    post_process::bloom::Bloom,
    prelude::*,
    ui::{ComputedNode, Overflow, Pressed, RelativeCursorPosition, ScrollPosition, UiTransform},
    ui_widgets::Button,
    window::{
        CursorGrabMode, CursorOptions, MonitorSelection, PresentMode, PrimaryWindow, VideoModeSelection, Window, WindowMode,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    clouds::CloudLayer,
    daynight::SunLight,
    reflection::ReflectionCamera,
    streaming::RenderSettings,
};

pub const FPS_VALUES: [u32; 7] = [30, 60, 90, 120, 144, 240, 0];
const FPS_LABELS: [&str; 7] = ["30", "60", "90", "120", "144", "240", "Tối đa"];
const VIEW_MIN: i32 = 2;
const VIEW_MAX: i32 = 32;
const SHADOW_DISTANCE_MIN: f32 = 64.0;
const SHADOW_DISTANCE_MAX: f32 = 320.0;
const FOG_MIN: f32 = 0.5;
const FOG_MAX: f32 = 2.0;
const GODRAY_MIN: f32 = 0.2;
const GODRAY_MAX: f32 = 3.0;
const CHAT_OPACITY_MIN: f32 = 0.0;
const CHAT_OPACITY_MAX: f32 = 0.9;
const CHAT_SCALE_MIN: f32 = 0.7;
const CHAT_SCALE_MAX: f32 = 1.8;
const CHAT_FADE_MIN: f32 = 3.0;
const CHAT_FADE_MAX: f32 = 30.0;
pub const SHADOW_RESOLUTIONS: [usize; 3] = [1024, 2048, 4096];
pub const REFLECTION_SCALES: [f32; 4] = [0.25, 0.5, 0.75, 1.0];
const QUALITY_LABELS: [&str; 4] = ["Thấp", "Trung bình", "Cao", "Cực cao"];
const SHADOW_RES_LABELS: [&str; 3] = ["1024", "2048", "4096"];
const REFLECTION_RES_LABELS: [&str; 4] = ["25%", "50%", "75%", "100%"];
const CLOUD_LAYER_LABELS: [&str; 3] = ["1 lớp", "2 lớp", "3 lớp"];
const WINDOW_LABELS: [&str; 3] = ["Cửa sổ", "Không viền", "Toàn màn hình"];

const ACCENT: Color = Color::srgb(0.26, 0.72, 0.54);
const BUTTON_BASE: Color = Color::srgba(0.16, 0.19, 0.24, 1.0);
const BUTTON_HOVER: Color = Color::srgba(0.22, 0.27, 0.33, 1.0);
const PANEL_BG: Color = Color::srgba(0.09, 0.10, 0.13, 0.97);
const TRACK_BG: Color = Color::srgba(0.05, 0.06, 0.08, 1.0);
const SCROLL_BG: Color = Color::srgba(0.05, 0.06, 0.08, 0.8);

pub const ACTION_FORWARD: usize = 0;
pub const ACTION_BACK: usize = 1;
pub const ACTION_LEFT: usize = 2;
pub const ACTION_RIGHT: usize = 3;
pub const ACTION_JUMP: usize = 4;
pub const ACTION_SPRINT: usize = 5;
pub const ACTION_SNEAK: usize = 6;
pub const ACTION_CHAT: usize = 7;

pub const KEY_ACTIONS: [&str; 8] = [
    "Tiến", "Lùi", "Sang trái", "Sang phải", "Nhảy", "Chạy", "Ngồi", "Chat",
];

pub const KEYS: [(KeyCode, &str); 22] = [
    (KeyCode::KeyW, "W"),
    (KeyCode::KeyA, "A"),
    (KeyCode::KeyS, "S"),
    (KeyCode::KeyD, "D"),
    (KeyCode::Space, "Space"),
    (KeyCode::ShiftLeft, "Shift"),
    (KeyCode::ControlLeft, "Ctrl"),
    (KeyCode::KeyT, "T"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::KeyE, "E"),
    (KeyCode::KeyQ, "Q"),
    (KeyCode::KeyF, "F"),
    (KeyCode::KeyG, "G"),
    (KeyCode::KeyV, "V"),
    (KeyCode::KeyC, "C"),
    (KeyCode::KeyX, "X"),
    (KeyCode::KeyZ, "Z"),
    (KeyCode::KeyR, "R"),
    (KeyCode::KeyB, "B"),
    (KeyCode::Slash, "/"),
    (KeyCode::Enter, "Enter"),
    (KeyCode::Backspace, "Backspace"),
];

pub fn bound_key(bind: &[u8; 8], action: usize) -> KeyCode {
    KEYS[(bind[action] as usize).min(KEYS.len() - 1)].0
}

pub fn bound_label(bind: &[u8; 8], action: usize) -> &'static str {
    KEYS[(bind[action] as usize).min(KEYS.len() - 1)].1
}

#[derive(Resource, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub vsync: bool,
    pub fps_index: usize,
    pub view_distance: i32,
    pub window_mode: usize,
    pub quality: usize,
    pub shadows: bool,
    pub shadow_resolution: usize,
    pub shadow_distance: f32,
    pub fxaa: bool,
    pub msaa: bool,
    pub reflections: bool,
    pub reflection_resolution: usize,
    pub clouds: bool,
    pub cloud_layers: usize,
    pub godrays: bool,
    pub godray_intensity: f32,
    pub bloom: bool,
    pub fog_radius: f32,
    pub keys: [u8; 8],
    pub chat_opacity: f32,
    pub chat_scale: f32,
    pub chat_fade: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vsync: true,
            fps_index: 6,
            view_distance: 7,
            window_mode: 0,
            quality: 2,
            shadows: true,
            shadow_resolution: 1,
            shadow_distance: 240.0,
            fxaa: false,
            msaa: true,
            reflections: true,
            reflection_resolution: 1,
            clouds: true,
            cloud_layers: 3,
            godrays: true,
            godray_intensity: 1.0,
            bloom: true,
            fog_radius: 1.0,
            keys: [0, 2, 1, 3, 4, 5, 6, 7],
            chat_opacity: 0.55,
            chat_scale: 1.0,
            chat_fade: 10.0,
        }
    }
}

const SETTINGS_FILE: &str = "config/settings.json";

impl Settings {
    pub fn load() -> Self {
        fs::read_to_string(SETTINGS_FILE)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if let Some(parent) = Path::new(SETTINGS_FILE).parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(SETTINGS_FILE, text).ok();
        }
    }
}

impl Settings {
    fn apply_quality(&mut self, index: usize) {
        self.quality = index.min(QUALITY_LABELS.len() - 1);
        match self.quality {
            0 => {
                self.shadows = false;
                self.shadow_resolution = 0;
                self.shadow_distance = 96.0;
                self.fxaa = true;
                self.msaa = false;
                self.reflections = false;
                self.reflection_resolution = 0;
                self.clouds = true;
                self.cloud_layers = 2;
                self.godrays = false;
                self.fog_radius = 1.0;
            }
            1 => {
                self.shadows = true;
                self.shadow_resolution = 0;
                self.shadow_distance = 144.0;
                self.fxaa = true;
                self.msaa = false;
                self.reflections = true;
                self.reflection_resolution = 0;
                self.clouds = true;
                self.cloud_layers = 3;
                self.godrays = false;
                self.fog_radius = 1.0;
            }
            2 => {
                self.shadows = true;
                self.shadow_resolution = 1;
                self.shadow_distance = 224.0;
                self.fxaa = false;
                self.msaa = true;
                self.reflections = true;
                self.reflection_resolution = 1;
                self.clouds = true;
                self.cloud_layers = 3;
                self.godrays = true;
                self.fog_radius = 1.0;
            }
            _ => {
                self.shadows = true;
                self.shadow_resolution = 2;
                self.shadow_distance = 320.0;
                self.fxaa = true;
                self.msaa = true;
                self.reflections = true;
                self.reflection_resolution = 3;
                self.clouds = true;
                self.cloud_layers = 3;
                self.godrays = true;
                self.fog_radius = 1.0;
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct MenuState {
    pub open: bool,
    pub page: u8,
    pub tab: usize,
    pub anim: f32,
    pub rebind: Option<usize>,
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct MenuPanel;

#[derive(Component)]
struct PageNode(u8);

#[derive(Component)]
struct TabNode(usize);

#[derive(Component)]
struct TabButton(usize);

#[derive(Component)]
struct MenuAction(Action);

#[derive(Component)]
struct HoverVisual {
    base: Color,
    hover: Color,
    press: Color,
}

#[derive(Component)]
struct SliderTrack(SliderKind);

#[derive(Component)]
struct SliderFill(SliderKind);

#[derive(Component)]
struct SliderHandle(SliderKind);

#[derive(Component)]
struct SliderLabel(SliderKind);

#[derive(Component)]
struct ToggleLabel(ToggleKind);

#[derive(Component)]
struct KeyRow(usize);

#[derive(Component)]
struct KeyLabel(usize);

#[derive(Component)]
struct ChoiceLabel(ChoiceKind);

#[derive(Component)]
struct ScrollView;

#[derive(Component)]
struct ScrollTrack;

#[derive(Component)]
struct ScrollThumb;

#[derive(Clone, Copy)]
enum SliderKind {
    Fps,
    View,
    ShadowDistance,
    FogRadius,
    Godray,
    ChatOpacity,
    ChatScale,
    ChatFade,
}

#[derive(Clone, Copy)]
enum ToggleKind {
    Vsync,
    Msaa,
    Fxaa,
    Shadows,
    Reflections,
    Clouds,
    Godrays,
    Bloom,
}

#[derive(Clone, Copy)]
enum ChoiceKind {
    Quality,
    ShadowResolution,
    ReflectionResolution,
    CloudLayers,
    WindowMode,
}

#[derive(Clone, Copy)]
enum Action {
    Resume,
    OpenSettings,
    Back,
    ToTitle,
    Quit,
    Tab(usize),
    Toggle(ToggleKind),
    Cycle(ChoiceKind),
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        let state = if std::env::var("ELOSIUM_MENU").is_ok() {
            MenuState {
                open: true,
                page: std::env::var("ELOSIUM_MENU_PAGE")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                tab: std::env::var("ELOSIUM_MENU_TAB")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                anim: 1.0,
                rebind: None,
            }
        } else {
            MenuState::default()
        };
        app.insert_resource(Settings::load())
            .insert_resource(state)
            .add_systems(
                Update,
                (
                    toggle_menu,
                    menu_actions,
                    clear_pressed,
                    handle_sliders,
                    handle_rebind,
                    handle_scrollbar,
                    menu_scroll,
                    apply_settings,
                    update_menu,
                    update_scrollbar,
                    animate_buttons,
                    persist_settings,
                )
                    .chain(),
            )
            .add_systems(Last, limit_fps);
    }
}

pub fn spawn_menu(commands: &mut Commands, asset_server: &AssetServer, theme: &crate::ui_theme::UiAssets) {
    let font: Handle<Font> = asset_server.load("fonts/mono.ttf");
    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            Visibility::Hidden,
            GlobalZIndex(50),
        ))
        .with_children(|root| {
            root.spawn((
                MenuPanel,
                Node {
                    width: percent(94),
                    max_width: px(1000),
                    height: percent(90),
                    max_height: px(680),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(14),
                    padding: UiRect::all(px(22)),
                    border_radius: BorderRadius::all(px(16)),
                    ..default()
                },
                crate::ui_theme::sliced(theme.panel_ornate.clone(), 20.0),
                UiTransform::default(),
            ))
            .with_children(|panel| {
                panel
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn((
                            Text::new("Elosium"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(26.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        header.spawn(plain(&font, "ESC để đóng", 13.0));
                    });

                panel
                    .spawn((
                        PageNode(0),
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: px(12),
                            ..default()
                        },
                    ))
                    .with_children(|page| {
                        button(page, &font, "Tiếp tục", Action::Resume, 340.0, 44.0);
                        button(page, &font, "Cài đặt", Action::OpenSettings, 340.0, 44.0);
                        button(page, &font, "Lưu & thoát ra tiêu đề", Action::ToTitle, 340.0, 44.0);
                        button(page, &font, "Thoát game", Action::Quit, 340.0, 44.0);
                    });

                panel
                    .spawn((
                        PageNode(1),
                        Node {
                            display: Display::None,
                            flex_grow: 1.0,
                            column_gap: px(18),
                            min_height: px(0),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            width: px(172),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(6),
                            ..default()
                        })
                        .with_children(|sidebar| {
                            tab_button(sidebar, &font, "Video", 0);
                            tab_button(sidebar, &font, "Đồ hoạ", 1);
                            tab_button(sidebar, &font, "Điều khiển", 2);
                            tab_button(sidebar, &font, "Chat", 3);
                            tab_button(sidebar, &font, "Khác", 4);
                            sidebar.spawn(Node {
                                flex_grow: 1.0,
                                ..default()
                            });
                            button(sidebar, &font, "Quay lại", Action::Back, 0.0, 36.0);
                        });

                        row.spawn(Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            min_height: px(0),
                            position_type: PositionType::Relative,
                            ..default()
                        })
                        .with_children(|wrap| {
                            wrap.spawn((
                                ScrollView,
                                ScrollPosition::default(),
                                Node {
                                    width: percent(100),
                                    height: percent(100),
                                    overflow: Overflow::scroll_y(),
                                    padding: UiRect::right(px(20)),
                                    flex_direction: FlexDirection::Column,
                                    ..default()
                                },
                            ))
                            .with_children(|view| {
                                view.spawn(Node {
                                    width: percent(100),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(14),
                                    ..default()
                                })
                                .with_children(|content| {
                                    content
                                        .spawn((
                                            TabNode(0),
                                            Node {
                                                width: percent(100),
                                                flex_direction: FlexDirection::Column,
                                                row_gap: px(14),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|video| {
                                            toggle_row(video, &font, "VSync", ToggleKind::Vsync);
                                            slider_row(video, &font, "Giới hạn FPS", SliderKind::Fps);
                                            slider_row(video, &font, "Tầm nhìn", SliderKind::View);
                                            choice_row(video, &font, "Kiểu cửa sổ", ChoiceKind::WindowMode);
                                        });
                                    content
                                        .spawn((
                                            TabNode(1),
                                            Node {
                                                display: Display::None,
                                                width: percent(100),
                                                flex_direction: FlexDirection::Column,
                                                row_gap: px(14),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|graphics| {
                                            choice_row(graphics, &font, "Chất lượng đồ hoạ", ChoiceKind::Quality);
                                            toggle_row(graphics, &font, "Đổ bóng (Shadow)", ToggleKind::Shadows);
                                            choice_row(graphics, &font, "Độ phân giải shadow", ChoiceKind::ShadowResolution);
                                            slider_row(graphics, &font, "Khoảng cách shadow", SliderKind::ShadowDistance);
                                            toggle_row(graphics, &font, "FXAA", ToggleKind::Fxaa);
                                            toggle_row(graphics, &font, "MSAA", ToggleKind::Msaa);
                                            toggle_row(graphics, &font, "Phản chiếu nước", ToggleKind::Reflections);
                                            choice_row(graphics, &font, "Độ phân giải phản chiếu", ChoiceKind::ReflectionResolution);
                                            toggle_row(graphics, &font, "Mây", ToggleKind::Clouds);
                                            choice_row(graphics, &font, "Lớp mây", ChoiceKind::CloudLayers);
                                            toggle_row(graphics, &font, "God rays", ToggleKind::Godrays);
                                            slider_row(graphics, &font, "Cường độ god ray", SliderKind::Godray);
                                            toggle_row(graphics, &font, "Bloom", ToggleKind::Bloom);
                                            slider_row(graphics, &font, "Bán kính sương", SliderKind::FogRadius);
                                        });
                                    content
                                        .spawn((
                                            TabNode(2),
                                            Node {
                                                display: Display::None,
                                                width: percent(100),
                                                flex_direction: FlexDirection::Column,
                                                row_gap: px(8),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|text| {
                                            for action in 0..8 {
                                                key_row(text, &font, KEY_ACTIONS[action], action);
                                            }
                                            text.spawn(plain(&font, "Space ×2: bật/tắt bay · Space/Ctrl: lên/xuống", 14.0));
                                            text.spawn(plain(&font, "V: đổi góc nhìn · F3: thông tin · F1: ẩn/hiện GUI", 14.0));
                                        });
                                    content
                                        .spawn((
                                            TabNode(3),
                                            Node {
                                                display: Display::None,
                                                width: percent(100),
                                                flex_direction: FlexDirection::Column,
                                                row_gap: px(14),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|chat| {
                                            slider_row(chat, &font, "Độ mờ nền chat", SliderKind::ChatOpacity);
                                            slider_row(chat, &font, "Cỡ chữ chat", SliderKind::ChatScale);
                                            slider_row(chat, &font, "Thời gian hiện chat", SliderKind::ChatFade);
                                            chat.spawn(plain(&font, "T: mở chat · Enter: gửi · Esc: đóng", 14.0));
                                            chat.spawn(plain(&font, "/help: danh sách lệnh", 14.0));
                                        });
                                    content
                                        .spawn((
                                            TabNode(4),
                                            Node {
                                                display: Display::None,
                                                width: percent(100),
                                                flex_direction: FlexDirection::Column,
                                                row_gap: px(8),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|text| {
                                            text.spawn(plain(&font, "Elosium 0.1", 15.0));
                                            text.spawn(plain(&font, "Voxel engine · Bevy 0.20", 15.0));
                                        });
                                });
                            });

                            wrap.spawn((
                                ScrollTrack,
                                Button,
                                RelativeCursorPosition::default(),
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: px(2),
                                    top: px(0),
                                    width: px(8),
                                    height: percent(100),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(SCROLL_BG),
                            ))
                            .with_children(|track| {
                                track.spawn((
                                    ScrollThumb,
                                    Button,
                                    Hovered::default(),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(0),
                                        top: percent(0),
                                        width: percent(100),
                                        height: percent(20),
                                        border_radius: BorderRadius::all(px(4)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.42, 0.48, 0.56, 0.95)),
                                ));
                            });
                        });
                    });
            });
        });
}

fn plain(font: &Handle<Font>, value: &str, size: f32) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.88, 0.92)),
    )
}

fn button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    action: Action,
    width: f32,
    height: f32,
) {
    let mut node = Node {
        height: px(height),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        border_radius: BorderRadius::all(px(9)),
        ..default()
    };
    if width > 0.0 {
        node.width = px(width);
    }
    parent
        .spawn((
            Button,
            Hovered::default(),
            MenuAction(action),
            HoverVisual {
                base: BUTTON_BASE,
                hover: BUTTON_HOVER,
                press: ACCENT,
            },
            UiTransform::default(),
            node,
            BackgroundColor(BUTTON_BASE),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn key_row(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str, action: usize) {
    parent
        .spawn(Node {
            width: percent(100),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(px(8)),
            ..default()
        })
        .with_children(|row| {
            row.spawn(plain(font, label, 15.0));
            row.spawn((
                Button,
                Hovered::default(),
                KeyRow(action),
                Node {
                    width: px(112),
                    height: px(30),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(7)),
                    ..default()
                },
                BackgroundColor(BUTTON_BASE),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    KeyLabel(action),
                ));
            });
        });
}

fn tab_button(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str, index: usize) {
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabButton(index),
            MenuAction(Action::Tab(index)),
            Node {
                height: px(36),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(12)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(BUTTON_BASE),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn toggle_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    kind: ToggleKind,
) {
    parent
        .spawn(Node {
            width: percent(100),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn(plain(font, label, 15.0));
            row.spawn((
                Button,
                Hovered::default(),
                MenuAction(Action::Toggle(kind)),
                HoverVisual {
                    base: BUTTON_BASE,
                    hover: BUTTON_HOVER,
                    press: ACCENT,
                },
                UiTransform::default(),
                Node {
                    width: px(90),
                    height: px(32),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(BUTTON_BASE),
            ))
            .with_children(|b| {
                b.spawn((
                    ToggleLabel(kind),
                    Text::new("Bật"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn choice_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    kind: ChoiceKind,
) {
    parent
        .spawn(Node {
            width: percent(100),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn(plain(font, label, 15.0));
            row.spawn((
                Button,
                Hovered::default(),
                MenuAction(Action::Cycle(kind)),
                HoverVisual {
                    base: BUTTON_BASE,
                    hover: BUTTON_HOVER,
                    press: ACCENT,
                },
                UiTransform::default(),
                Node {
                    width: px(150),
                    height: px(32),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(BUTTON_BASE),
            ))
            .with_children(|b| {
                b.spawn((
                    ChoiceLabel(kind),
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn slider_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    kind: SliderKind,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        })
        .with_children(|column| {
            column
                .spawn(Node {
                    width: percent(100),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(plain(font, label, 15.0));
                    row.spawn((
                        SliderLabel(kind),
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(ACCENT),
                    ));
                });
            column
                .spawn((
                    Button,
                    SliderTrack(kind),
                    RelativeCursorPosition::default(),
                    Node {
                        width: percent(100),
                        height: px(12),
                        border_radius: BorderRadius::all(px(6)),
                        ..default()
                    },
                    BackgroundColor(TRACK_BG),
                ))
                .with_children(|track| {
                    track.spawn((
                        SliderFill(kind),
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(0),
                            top: px(0),
                            width: percent(50),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(ACCENT),
                    ));
                    track.spawn((
                        SliderHandle(kind),
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(50),
                            top: px(-3),
                            width: px(18),
                            height: px(18),
                            border_radius: BorderRadius::all(px(9)),
                            ..default()
                        },
                        UiTransform {
                            translation: Val2::px(-9.0, 0.0),
                            ..default()
                        },
                        BackgroundColor(Color::WHITE),
                    ));
                });
        });
}

fn toggle_menu(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MenuState>,
    screen: Res<crate::frontend::ScreenState>,
    chat: Res<crate::chat::ChatState>,
    creative: Res<crate::creative::CreativeState>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if std::env::var("ELOSIUM_BAKE_PANORAMA").is_ok()
        || std::env::var("ELOSIUM_CAPTURE").is_ok()
    {
        return;
    }
    if chat.open || chat.closed_esc || creative.open {
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if !state.open && screen.screen != crate::frontend::Screen::Playing {
        return;
    }
    state.open = !state.open;
    if state.open {
        state.page = 0;
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    } else if screen.screen == crate::frontend::Screen::Playing {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    } else {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

fn menu_actions(
    query: Query<&MenuAction, Added<Pressed>>,
    mut state: ResMut<MenuState>,
    mut settings: ResMut<Settings>,
    mut screen: ResMut<crate::frontend::ScreenState>,
    mut exit: MessageWriter<AppExit>,
    mut cursor: Single<&mut CursorOptions>,
    mut views: Query<&mut ScrollPosition, With<ScrollView>>,
) {
    for action in &query {
        match action.0 {
            Action::Resume => {
                state.open = false;
                cursor.visible = false;
                cursor.grab_mode = CursorGrabMode::Locked;
            }
            Action::ToTitle => {
                state.open = false;
                screen.screen = crate::frontend::Screen::Title;
                cursor.visible = true;
                cursor.grab_mode = CursorGrabMode::None;
            }
            Action::OpenSettings => {
                state.page = 1;
                for mut view in &mut views {
                    view.y = 0.0;
                }
            }
            Action::Back => {
                state.page = 0;
                for mut view in &mut views {
                    view.y = 0.0;
                }
            }
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            Action::Tab(index) => {
                state.tab = index;
                for mut view in &mut views {
                    view.y = 0.0;
                }
            }
            Action::Toggle(kind) => match kind {
                ToggleKind::Vsync => settings.vsync = !settings.vsync,
                ToggleKind::Msaa => settings.msaa = !settings.msaa,
                ToggleKind::Fxaa => settings.fxaa = !settings.fxaa,
                ToggleKind::Shadows => settings.shadows = !settings.shadows,
                ToggleKind::Reflections => settings.reflections = !settings.reflections,
                ToggleKind::Clouds => settings.clouds = !settings.clouds,
                ToggleKind::Godrays => settings.godrays = !settings.godrays,
                ToggleKind::Bloom => settings.bloom = !settings.bloom,
            },
            Action::Cycle(kind) => match kind {
                ChoiceKind::Quality => {
                    settings.quality = (settings.quality + 1) % QUALITY_LABELS.len();
                    let quality = settings.quality;
                    settings.apply_quality(quality);
                }
                ChoiceKind::ShadowResolution => {
                    settings.shadow_resolution =
                        (settings.shadow_resolution + 1) % SHADOW_RESOLUTIONS.len();
                }
                ChoiceKind::ReflectionResolution => {
                    settings.reflection_resolution =
                        (settings.reflection_resolution + 1) % REFLECTION_SCALES.len();
                }
                ChoiceKind::CloudLayers => {
                    settings.cloud_layers =
                        (settings.cloud_layers % CLOUD_LAYER_LABELS.len()) + 1;
                }
                ChoiceKind::WindowMode => {
                    settings.window_mode = (settings.window_mode + 1) % WINDOW_LABELS.len();
                }
            },
        }
    }
}

fn clear_pressed(
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    pressed: Query<Entity, With<Pressed>>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    for entity in &pressed {
        commands.entity(entity).remove::<Pressed>();
    }
}

fn handle_sliders(
    mouse: Res<ButtonInput<MouseButton>>,
    mut settings: ResMut<Settings>,
    tracks: Query<(&RelativeCursorPosition, &SliderTrack, Has<Pressed>)>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    for (cursor, kind, pressed) in &tracks {
        if !pressed {
            continue;
        }
        let Some(normalized) = cursor.normalized else {
            continue;
        };
        let fraction = (normalized.x + 0.5).clamp(0.0, 1.0);
        match kind.0 {
            SliderKind::Fps => {
                settings.fps_index = (fraction * 6.0).round() as usize;
            }
            SliderKind::View => {
                let span = (VIEW_MAX - VIEW_MIN) as f32;
                settings.view_distance = VIEW_MIN + (fraction * span).round() as i32;
            }
            SliderKind::ShadowDistance => {
                let span = SHADOW_DISTANCE_MAX - SHADOW_DISTANCE_MIN;
                settings.shadow_distance =
                    ((SHADOW_DISTANCE_MIN + fraction * span) / 8.0).round() * 8.0;
            }
            SliderKind::FogRadius => {
                let span = FOG_MAX - FOG_MIN;
                settings.fog_radius =
                    ((FOG_MIN + fraction * span) * 10.0).round() / 10.0;
            }
            SliderKind::Godray => {
                let span = GODRAY_MAX - GODRAY_MIN;
                settings.godray_intensity =
                    ((GODRAY_MIN + fraction * span) * 10.0).round() / 10.0;
            }
            SliderKind::ChatOpacity => {
                let span = CHAT_OPACITY_MAX - CHAT_OPACITY_MIN;
                settings.chat_opacity =
                    ((CHAT_OPACITY_MIN + fraction * span) * 100.0).round() / 100.0;
            }
            SliderKind::ChatScale => {
                let span = CHAT_SCALE_MAX - CHAT_SCALE_MIN;
                settings.chat_scale =
                    ((CHAT_SCALE_MIN + fraction * span) * 10.0).round() / 10.0;
            }
            SliderKind::ChatFade => {
                let span = CHAT_FADE_MAX - CHAT_FADE_MIN;
                settings.chat_fade = ((CHAT_FADE_MIN + fraction * span) * 1.0).round();
            }
        }
    }
}

fn handle_scrollbar(
    mouse: Res<ButtonInput<MouseButton>>,
    thumbs: Query<Has<Pressed>, With<ScrollThumb>>,
    tracks: Query<&RelativeCursorPosition, With<ScrollTrack>>,
    mut views: Query<(&ComputedNode, &mut ScrollPosition), Without<ScrollThumb>>,
) {
    if !mouse.pressed(MouseButton::Left) || !thumbs.iter().any(|pressed| pressed) {
        return;
    }
    let Some(normalized) = tracks.iter().next().and_then(|cursor| cursor.normalized) else {
        return;
    };
    let fraction = (normalized.y + 0.5).clamp(0.0, 1.0);
    for (computed, mut scroll) in &mut views {
        let max = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        scroll.y = fraction * max;
    }
}

fn menu_scroll(
    mut wheel: MessageReader<MouseWheel>,
    state: Res<MenuState>,
    mut views: Query<(&ComputedNode, &mut ScrollPosition), With<ScrollView>>,
) {
    if !state.open || state.page != 1 {
        return;
    }
    let mut delta = 0.0;
    for event in wheel.read() {
        delta += match event.unit {
            MouseScrollUnit::Line => -event.y * 40.0,
            MouseScrollUnit::Pixel => -event.y,
        };
    }
    if delta == 0.0 {
        return;
    }
    for (computed, mut scroll) in &mut views {
        let max = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        scroll.y = (scroll.y + delta).clamp(0.0, max);
    }
}

fn apply_settings(
    mut commands: Commands,
    settings: Res<Settings>,
    mut render: ResMut<RenderSettings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut cameras: Query<(Entity, &mut Msaa), (With<Camera3d>, Without<ReflectionCamera>)>,
    mut suns: Query<(&mut DirectionalLight, Entity), With<SunLight>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut clouds: Query<(&CloudLayer, &mut Visibility)>,
) {
    if !settings.is_changed() {
        return;
    }

    let present = if settings.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::Immediate
    };
    if window.present_mode != present {
        window.present_mode = present;
    }

    let mode = match settings.window_mode {
        1 => WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        2 => WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current),
        _ => WindowMode::Windowed,
    };
    if window.mode != mode {
        window.mode = mode;
    }

    render.distance = settings.view_distance.clamp(VIEW_MIN, VIEW_MAX);

    for (entity, mut msaa) in &mut cameras {
        let target = if settings.msaa { Msaa::Sample4 } else { Msaa::Off };
        if *msaa != target {
            *msaa = target;
        }
        if settings.fxaa {
            commands.entity(entity).insert(Fxaa::default());
        } else {
            commands.entity(entity).remove::<Fxaa>();
        }
        if settings.godrays {
            commands.entity(entity).insert(VolumetricFog {
                ambient_intensity: 0.0,
                step_count: 64,
                jitter: 0.0,
                ..default()
            });
        } else {
            commands.entity(entity).remove::<VolumetricFog>();
        }
        let bloom_on = settings.bloom && std::env::var("ELOSIUM_NO_BLOOM").is_err();
        if bloom_on {
            commands.entity(entity).insert(Bloom {
                intensity: 0.22,
                ..Bloom::NATURAL
            });
        } else {
            commands.entity(entity).remove::<Bloom>();
        }
    }

    for (mut light, entity) in &mut suns {
        light.shadow_maps_enabled = settings.shadows;
        commands.entity(entity).insert(
            CascadeShadowConfigBuilder {
                num_cascades: 3,
                maximum_distance: settings.shadow_distance,
                first_cascade_far_bound: 48.0,
                ..default()
            }
            .build(),
        );
    }

    shadow_map.size = SHADOW_RESOLUTIONS
        [settings.shadow_resolution.min(SHADOW_RESOLUTIONS.len() - 1)];

    for (layer, mut visibility) in &mut clouds {
        *visibility = if settings.clouds && layer.index < settings.cloud_layers {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn slider_fraction(kind: SliderKind, settings: &Settings) -> f32 {
    match kind {
        SliderKind::Fps => settings.fps_index.min(6) as f32 / 6.0,
        SliderKind::View => {
            (settings.view_distance - VIEW_MIN) as f32 / (VIEW_MAX - VIEW_MIN) as f32
        }
        SliderKind::ShadowDistance => {
            (settings.shadow_distance - SHADOW_DISTANCE_MIN)
                / (SHADOW_DISTANCE_MAX - SHADOW_DISTANCE_MIN)
        }
        SliderKind::FogRadius => (settings.fog_radius - FOG_MIN) / (FOG_MAX - FOG_MIN),
        SliderKind::Godray => {
            (settings.godray_intensity - GODRAY_MIN) / (GODRAY_MAX - GODRAY_MIN)
        }
        SliderKind::ChatOpacity => {
            (settings.chat_opacity - CHAT_OPACITY_MIN) / (CHAT_OPACITY_MAX - CHAT_OPACITY_MIN)
        }
        SliderKind::ChatScale => {
            (settings.chat_scale - CHAT_SCALE_MIN) / (CHAT_SCALE_MAX - CHAT_SCALE_MIN)
        }
        SliderKind::ChatFade => {
            (settings.chat_fade - CHAT_FADE_MIN) / (CHAT_FADE_MAX - CHAT_FADE_MIN)
        }
    }
}

fn update_menu(
    time: Res<Time>,
    mut state: ResMut<MenuState>,
    settings: Res<Settings>,
    mut root: Query<
        (&mut Visibility, &mut BackgroundColor),
        (With<MenuRoot>, Without<TabButton>),
    >,
    mut panel: Query<&mut UiTransform, (With<MenuPanel>, Without<HoverVisual>)>,
    mut pages: Query<
        (&PageNode, &mut Node),
        (Without<TabNode>, Without<SliderFill>, Without<SliderHandle>),
    >,
    mut tabs: Query<
        (&TabNode, &mut Node),
        (Without<PageNode>, Without<SliderFill>, Without<SliderHandle>),
    >,
    mut tab_buttons: Query<(&TabButton, &mut BackgroundColor), Without<MenuRoot>>,
    mut toggles: Query<
        (&ToggleLabel, &mut Text),
        (Without<SliderLabel>, Without<KeyLabel>, Without<ChoiceLabel>),
    >,
    mut choices: Query<
        (&ChoiceLabel, &mut Text),
        (Without<SliderLabel>, Without<ToggleLabel>, Without<KeyLabel>),
    >,
    mut slider_labels: Query<
        (&SliderLabel, &mut Text),
        (Without<ToggleLabel>, Without<ChoiceLabel>, Without<KeyLabel>),
    >,
    mut fills: Query<
        (&SliderFill, &mut Node),
        (Without<SliderHandle>, Without<PageNode>, Without<TabNode>),
    >,
    mut handles: Query<
        (&SliderHandle, &mut Node),
        (Without<SliderFill>, Without<PageNode>, Without<TabNode>),
    >,
    mut key_labels: Query<
        (&KeyLabel, &mut Text),
        (Without<SliderLabel>, Without<ToggleLabel>, Without<ChoiceLabel>),
    >,
) {
    let step = (time.delta_secs() * 9.0).min(1.0);
    let target = if state.open { 1.0 } else { 0.0 };
    state.anim += (target - state.anim) * step;
    if (state.anim - target).abs() < 0.002 {
        state.anim = target;
    }

    for (mut visibility, mut background) in &mut root {
        if state.open || state.anim > 0.01 {
            *visibility = Visibility::Inherited;
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.55 * state.anim);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    for mut transform in &mut panel {
        let scale = 0.94 + 0.06 * state.anim;
        transform.scale = Vec2::splat(scale);
    }
    for (page, mut node) in &mut pages {
        node.display = if page.0 == state.page {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (tab, mut node) in &mut tabs {
        node.display = if tab.0 == state.tab {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (tab, mut background) in &mut tab_buttons {
        background.0 = if tab.0 == state.tab {
            ACCENT
        } else {
            BUTTON_BASE
        };
    }
    for (label, mut text) in &mut toggles {
        let on = match label.0 {
            ToggleKind::Vsync => settings.vsync,
            ToggleKind::Msaa => settings.msaa,
            ToggleKind::Fxaa => settings.fxaa,
            ToggleKind::Shadows => settings.shadows,
            ToggleKind::Reflections => settings.reflections,
            ToggleKind::Clouds => settings.clouds,
            ToggleKind::Godrays => settings.godrays,
            ToggleKind::Bloom => settings.bloom,
        };
        text.0 = match (label.0, on) {
            (ToggleKind::Msaa, true) => "4x".to_string(),
            (_, true) => "Bật".to_string(),
            (_, false) => "Tắt".to_string(),
        };
    }
    for (label, mut text) in &mut choices {
        text.0 = match label.0 {
            ChoiceKind::Quality => QUALITY_LABELS[settings.quality.min(3)].to_string(),
            ChoiceKind::ShadowResolution => {
                SHADOW_RES_LABELS[settings.shadow_resolution.min(2)].to_string()
            }
            ChoiceKind::ReflectionResolution => {
                REFLECTION_RES_LABELS[settings.reflection_resolution.min(3)].to_string()
            }
            ChoiceKind::CloudLayers => {
                CLOUD_LAYER_LABELS[(settings.cloud_layers - 1).min(2)].to_string()
            }
            ChoiceKind::WindowMode => WINDOW_LABELS[settings.window_mode.min(2)].to_string(),
        };
    }
    for (label, mut text) in &mut slider_labels {
        text.0 = match label.0 {
            SliderKind::Fps => {
                let index = settings.fps_index.min(6);
                if FPS_VALUES[index] == 0 {
                    "Tối đa".to_string()
                } else {
                    format!("{} FPS", FPS_LABELS[index])
                }
            }
            SliderKind::View => format!(
                "{} chunk · {} block",
                settings.view_distance,
                settings.view_distance * 16
            ),
            SliderKind::ShadowDistance => format!("{:.0} block", settings.shadow_distance),
            SliderKind::FogRadius => format!("{:.1}×", settings.fog_radius),
            SliderKind::Godray => format!("{:.1}×", settings.godray_intensity),
            SliderKind::ChatOpacity => format!("{:.0}%", settings.chat_opacity * 100.0),
            SliderKind::ChatScale => format!("{:.1}×", settings.chat_scale),
            SliderKind::ChatFade => format!("{:.0}s", settings.chat_fade),
        };
    }
    for (fill, mut node) in &mut fills {
        let fraction = slider_fraction(fill.0, &settings);
        node.width = percent(fraction.clamp(0.0, 1.0) * 100.0);
    }
    for (handle, mut node) in &mut handles {
        let fraction = slider_fraction(handle.0, &settings);
        node.left = percent(fraction.clamp(0.0, 1.0) * 100.0);
    }
    for (label, mut text) in &mut key_labels {
        text.0 = if state.rebind == Some(label.0) {
            "Nhấn phím...".to_string()
        } else {
            bound_label(&settings.keys, label.0).to_string()
        };
    }
}

fn handle_rebind(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MenuState>,
    mut settings: ResMut<Settings>,
    rows: Query<&KeyRow, Added<Pressed>>,
) {
    for row in &rows {
        state.rebind = Some(row.0);
    }
    let Some(action) = state.rebind else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        state.rebind = None;
        return;
    }
    for (index, (code, _)) in KEYS.iter().enumerate() {
        if keys.just_pressed(*code) {
            settings.keys[action] = index as u8;
            state.rebind = None;
            settings.save();
            return;
        }
    }
}

fn persist_settings(settings: Res<Settings>) {
    if settings.is_changed() {
        settings.save();
    }
}

fn update_scrollbar(
    views: Query<(&ComputedNode, &ScrollPosition), (With<ScrollView>, Without<ScrollThumb>)>,
    mut thumbs: Query<&mut Node, With<ScrollThumb>>,
) {
    let Ok((computed, scroll)) = views.single() else {
        return;
    };
    let scale = computed.inverse_scale_factor();
    let view_height = (computed.size().y * scale).max(1.0);
    let content_height = (computed.content_size().y * scale).max(view_height);
    let max = (content_height - view_height).max(0.0);
    let fraction = if max > 0.0 {
        (scroll.y / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let thumb_fraction = (view_height / content_height).clamp(0.12, 1.0);
    for mut node in &mut thumbs {
        node.height = percent(thumb_fraction * 100.0);
        node.top = percent(fraction * (1.0 - thumb_fraction) * 100.0);
    }
}

fn animate_buttons(
    time: Res<Time>,
    mut query: Query<(
        &HoverVisual,
        &Hovered,
        Has<Pressed>,
        &mut BackgroundColor,
        &mut UiTransform,
    )>,
) {
    let factor = (time.delta_secs() * 16.0).min(1.0);
    for (visual, hovered, pressed, mut background, mut transform) in &mut query {
        let (target, scale) = if pressed {
            (visual.press, 0.97)
        } else if hovered.get() {
            (visual.hover, 1.02)
        } else {
            (visual.base, 1.0)
        };
        background.0 = mix_color(background.0, target, factor);
        let current = transform.scale.x;
        transform.scale = Vec2::splat(current + (scale - current) * factor);
    }
}

fn mix_color(from: Color, to: Color, factor: f32) -> Color {
    let a = from.to_srgba();
    let b = to.to_srgba();
    let mix = |x: f32, y: f32| x + (y - x) * factor;
    Color::Srgba(bevy::color::Srgba::new(
        mix(a.red, b.red),
        mix(a.green, b.green),
        mix(a.blue, b.blue),
        mix(a.alpha, b.alpha),
    ))
}

fn limit_fps(settings: Res<Settings>, mut last: Local<Option<Instant>>) {
    let fps = FPS_VALUES[settings.fps_index.min(FPS_VALUES.len() - 1)];
    let now = Instant::now();
    if fps > 0 {
        if let Some(previous) = *last {
            let target = 1.0 / fps as f64;
            let elapsed = now.duration_since(previous).as_secs_f64();
            if elapsed < target {
                std::thread::sleep(Duration::from_secs_f64(target - elapsed));
            }
        }
    }
    *last = Some(Instant::now());
}
