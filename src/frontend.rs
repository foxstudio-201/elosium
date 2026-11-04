use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::AppExit,
    asset::{LoadState, RenderAssetUsages},
    ecs::hierarchy::ChildSpawnerCommands,
    image::{
        CompressedImageFormats, ImageFilterMode, ImageSampler, ImageSamplerDescriptor, ImageType,
    },
    input::keyboard::{KeyCode, KeyboardInput},
    input::mouse::{MouseScrollUnit, MouseWheel},
    camera::visibility::RenderLayers,
    input::ButtonState,
    light::FogVolume,
    light::NotShadowCaster,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
    ui::{ComputedNode, Overflow, Pressed, ScrollPosition},
    ui_widgets::Button,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    daynight::DayCycle,
    ui_theme::{sliced, ThemeSkip, UiAssets},
    debug::HudVisible,
    menu::MenuState,
    player::{Player, PlayerPose, SpawnPoint, EYE_HEIGHT},
    reflection::ReflectionCamera,
    streaming::{ChunkRenderer, LoadWarmup, RenderSettings},
    world::{Block, ChunkPos, VoxelWorld},
    worldgen,
};

const ACCENT: Color = Color::srgb(0.26, 0.72, 0.54);
const BUTTON_BASE: Color = Color::srgba(0.16, 0.19, 0.24, 1.0);
const BUTTON_ACTIVE: Color = Color::srgba(0.18, 0.45, 0.36, 1.0);
const PANEL_BG: Color = Color::srgba(0.09, 0.10, 0.13, 0.97);
const TRACK_BG: Color = Color::srgba(0.05, 0.06, 0.08, 1.0);
const MUTED: Color = Color::srgb(0.62, 0.67, 0.74);
const WHITE: Color = Color::WHITE;

const GAMEMODES: [&str; 4] = ["Sinh tồn", "Sáng tạo", "Phiêu lưu", "Khán giả"];
const WORLD_TYPES: [&str; 4] = ["Mặc định", "Siêu phẳng", "Biome lớn", "Khuếch đại"];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Screen {
    #[default]
    Loading,
    Title,
    Worlds,
    Create,
    Playing,
}

#[derive(Resource, Default)]
pub struct ScreenState {
    pub screen: Screen,
    pub world_load: bool,
}

pub fn streaming_active(screen: Res<ScreenState>) -> bool {
    screen.screen == Screen::Playing || screen.world_load
}

#[derive(Resource, Default)]
pub struct GameRules {
    pub gamemode: u8,
    pub cheats: bool,
    pub share: bool,
}

pub fn in_game(screen: Res<ScreenState>) -> bool {
    screen.screen == Screen::Playing
}

#[derive(Clone)]
enum LoadMode {
    Startup,
    World { icon: bool, slug: String },
}

#[derive(Resource)]
struct Loading {
    mode: LoadMode,
    elapsed: f32,
    min: f32,
    label: String,
}

impl Default for Loading {
    fn default() -> Self {
        Self {
            mode: LoadMode::Startup,
            elapsed: 0.0,
            min: 2.0,
            label: "Đang nạp tài nguyên...".into(),
        }
    }
}

#[derive(Resource)]
struct PostLoading {
    target: Screen,
}

impl Default for PostLoading {
    fn default() -> Self {
        Self {
            target: Screen::Title,
        }
    }
}

#[derive(Resource, Default)]
struct ActiveWorld(Option<WorldMeta>);

#[derive(Resource)]
struct AutoSaveTimer(Timer);

impl Default for AutoSaveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(30.0, TimerMode::Repeating))
    }
}

#[derive(Resource, Default)]
struct BakePanorama {
    stage: u8,
    timer: f32,
    face: usize,
    frames: u32,
    wait: f32,
}

#[derive(Resource, Default)]
struct WorldsUi {
    dirty: bool,
    pending_delete: Option<String>,
}

#[derive(Resource)]
struct CreateForm {
    name: String,
    seed: String,
    gamemode: u8,
    world_type: u8,
    cheats: bool,
    share: bool,
    focus: u8,
    submit: bool,
}

impl Default for CreateForm {
    fn default() -> Self {
        Self {
            name: "Thế giới mới".into(),
            seed: String::new(),
            gamemode: 0,
            world_type: 0,
            cheats: false,
            share: false,
            focus: 0,
            submit: false,
        }
    }
}

#[derive(Resource, Default)]
struct StartRequest(Option<WorldMeta>);

#[derive(Resource, Default)]
struct FrontIcons(HashMap<String, Handle<Image>>);

#[derive(Resource)]
struct IconCapture {
    path: PathBuf,
    timer: f32,
    phase: u8,
}

#[derive(Resource)]
struct FrontAssets {
    font: Handle<Font>,
    title: Handle<Image>,
    panorama: Handle<Image>,
}

#[derive(Clone, Default)]
struct WorldMeta {
    slug: String,
    name: String,
    seed: u32,
    seed_text: String,
    gamemode: u8,
    cheats: bool,
    share: bool,
    world_type: u8,
    created: u64,
    played: u64,
    px: f32,
    py: f32,
    pz: f32,
    yaw: f32,
    pitch: f32,
    time: f32,
}

#[derive(Component)]
struct FrontRoot(RootKind);

#[derive(Clone, Copy, PartialEq, Eq)]
enum RootKind {
    Loading,
    Title,
    Worlds,
    Create,
    Confirm,
}

#[derive(Component)]
struct PanoramaRig;

#[derive(Component)]
struct LoadingFill;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct WorldsScroll;

#[derive(Component)]
struct WorldsGrid;

#[derive(Component)]
struct ConfirmText;

#[derive(Component)]
struct FieldText(u8);

#[derive(Component)]
struct GamemodeButton(u8);

#[derive(Component)]
struct WorldTypeLabel;

#[derive(Component)]
struct CheatLabel;

#[derive(Component)]
struct ShareLabel;

#[derive(Component)]
struct FrontAction(Action);

#[derive(Clone)]
enum Action {
    Singleplayer,
    Settings,
    Quit,
    BackToTitle,
    OpenCreate,
    CreateConfirm,
    CreateCancel,
    PlayWorld(String),
    AskDelete(String),
    ConfirmDelete,
    CancelDelete,
    Gamemode(u8),
    CycleWorldType,
    ToggleCheats,
    ToggleShare,
    FocusField(u8),
}

pub struct FrontendPlugin;

impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        let ui_env = std::env::var("ELOSIUM_UI").ok();
        let skip = std::env::var("ELOSIUM_SKIP_LOADING").is_ok();
        let baking = std::env::var("ELOSIUM_BAKE_PANORAMA").is_ok();
        let capture_only = std::env::var("ELOSIUM_CAPTURE").is_ok() && ui_env.is_none();
        let target = match ui_env.as_deref() {
            Some("title") => Some(Screen::Title),
            Some("worlds") => Some(Screen::Worlds),
            Some("create") => Some(Screen::Create),
            Some("loading") => Some(Screen::Loading),
            Some("playing") => Some(Screen::Playing),
            _ => None,
        };
        let initial = if baking
            || (capture_only && std::env::var("ELOSIUM_PLAY").is_err())
        {
            Screen::Playing
        } else if skip {
            target.unwrap_or(Screen::Loading)
        } else {
            Screen::Loading
        };
        let post = target.unwrap_or(Screen::Title);

        app.insert_resource(ScreenState {
            screen: initial,
            world_load: false,
        })
            .insert_resource(GameRules::default())
            .insert_resource(Loading::default())
            .insert_resource(PostLoading { target: post })
            .insert_resource(ActiveWorld::default())
            .insert_resource(BakePanorama::default())
            .insert_resource(AutoSaveTimer::default())
            .insert_resource(WorldsUi {
                dirty: true,
                pending_delete: None,
            })
            .insert_resource(CreateForm::default())
            .insert_resource(StartRequest::default())
            .insert_resource(FrontIcons::default())
            .add_systems(
                Update,
                (
                    auto_test_hooks,
                    front_actions,
                    process_start_request,
                    create_text_input,
                    front_escape,
                    advance_loading,
                    sync_roots,
                    sync_camera_layers,
                    on_screen_change,
                    update_loading_ui,
                    sync_create_labels,
                    update_confirm_text,
                    rebuild_world_list,
                    world_list_scroll,
                    animate_panorama_rig,
                    capture_icon,
                    autosave_world,
                    save_world_state,
                    menu_camera_orientation,
                    bake_panorama,
                )
                    .chain(),
            );
    }
}

pub fn spawn_screens(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    theme: Res<UiAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    fs::create_dir_all("assets/panorama").ok();
    fs::create_dir_all("saves").ok();
    let font: Handle<Font> = asset_server.load("fonts/mono.ttf");
    let title: Handle<Image> = asset_server.load("textures/elosium_title.png");
    let panorama: Handle<Image> = asset_server.load("panorama/face_front.png");
    commands.insert_resource(FrontAssets {
        font: font.clone(),
        title: title.clone(),
        panorama,
    });

    let plane = meshes.add(Rectangle::new(120.0, 120.0));
    let faces: [(&str, Vec3, Quat); 6] = [
        ("front", Vec3::new(0.0, 0.0, -60.0), Quat::IDENTITY),
        ("right", Vec3::new(60.0, 0.0, 0.0), Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        ("back", Vec3::new(0.0, 0.0, 60.0), Quat::from_rotation_y(std::f32::consts::PI)),
        ("left", Vec3::new(-60.0, 0.0, 0.0), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ("up", Vec3::new(0.0, 60.0, 0.0), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ("down", Vec3::new(0.0, -60.0, 0.0), Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ];
    let rig = commands
        .spawn((
            PanoramaRig,
            Transform::default(),
            Visibility::Inherited,
            RenderLayers::layer(5),
        ))
        .id();
    for (name, position, rotation) in faces {
        let texture: Handle<Image> = asset_server.load(format!("panorama/face_{name}.png"));
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            unlit: true,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        });
        let face = commands
            .spawn((
                Mesh3d(plane.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(position).with_rotation(rotation),
                RenderLayers::layer(5),
                NotShadowCaster,
            ))
            .id();
        commands.entity(rig).add_child(face);
    }

    commands
        .spawn((
            FrontRoot(RootKind::Loading),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(40),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.045, 0.055, 0.075)),
            ));
            root.spawn((
                ImageNode::new(title.clone()),
                Node {
                    width: percent(52),
                    max_width: px(760),
                    aspect_ratio: Some(3.0),
                    ..default()
                },
            ));
            root.spawn(Node {
                width: px(440),
                max_width: percent(80),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                margin: UiRect::top(px(42)),
                ..default()
            })
            .with_children(|bar| {
                bar.spawn((
                    LoadingText,
                    Text::new("Đang tải... 0%"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(MUTED),
                ));
                bar.spawn((
                    crate::ui_theme::sliced(theme.bar_frame.clone(), 10.0),
                    Node {
                        width: percent(100),
                        height: px(18),
                        padding: UiRect::all(px(4)),
                        ..default()
                    },
                ))
                .with_children(|track| {
                    track.spawn((
                        LoadingFill,
                        crate::ui_theme::sliced(theme.bar_fill_mp.clone(), 4.0),
                        Node {
                            width: percent(0),
                            height: percent(100),
                            ..default()
                        },
                    ));
                });
            });
        });

    commands
        .spawn((
            FrontRoot(RootKind::Title),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(12),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(30),
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode::new(title),
                Node {
                    width: percent(48),
                    max_width: px(760),
                    aspect_ratio: Some(3.0),
                    margin: UiRect::bottom(px(26)),
                    ..default()
                },
            ));
            front_button(root, &font, "Chơi đơn", Action::Singleplayer, 360.0, 46.0);
            front_button(root, &font, "Cài đặt", Action::Settings, 360.0, 46.0);
            front_button(root, &font, "Thoát game", Action::Quit, 360.0, 46.0);
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(16),
                    bottom: px(12),
                    ..default()
                },
                text(&font, "Elosium 0.1 · Bevy 0.20", 12.0, MUTED),
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(16),
                    bottom: px(12),
                    ..default()
                },
                text(&font, "Seed, chế độ chơi và thế giới được lưu trong thư mục saves", 12.0, MUTED),
            ));
        });

    commands
        .spawn((
            FrontRoot(RootKind::Worlds),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(28)),
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.55)),
            Visibility::Hidden,
            GlobalZIndex(30),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn(text(&font, "Thế giới", 28.0, WHITE));
                header.spawn(text(&font, "Chọn một thế giới để chơi", 13.0, MUTED));
            });
            root.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                ..default()
            })
            .with_children(|wrap| {
                wrap.spawn((
                    WorldsScroll,
                    ScrollPosition::default(),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|scroll| {
                    scroll.spawn((
                        WorldsGrid,
                        Node {
                            width: percent(100),
                            flex_wrap: FlexWrap::Wrap,
                            align_content: AlignContent::FlexStart,
                            column_gap: px(16),
                            row_gap: px(16),
                            ..default()
                        },
                    ));
                });
            });
            root.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|footer| {
                front_button(footer, &font, "Tạo thế giới", Action::OpenCreate, 220.0, 44.0);
                front_button(footer, &font, "Quay lại", Action::BackToTitle, 160.0, 44.0);
            });
        });

    commands
        .spawn((
            FrontRoot(RootKind::Create),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.6)),
            Visibility::Hidden,
            GlobalZIndex(30),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(760),
                    max_width: percent(94),
                    max_height: percent(94),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(14),
                    padding: UiRect::all(px(24)),
                    border_radius: BorderRadius::all(px(16)),
                    ..default()
                },
                sliced(theme.panel_ornate.clone(), 20.0),
            ))
            .with_children(|panel| {
                panel.spawn(text(&font, "Tạo thế giới mới", 24.0, WHITE));
                field_row(panel, &font, "Tên thế giới", 0, &CreateForm::default().name);
                field_row(panel, &font, "Seed", 1, &CreateForm::default().seed);
                panel.spawn(text(&font, "Chế độ chơi", 14.0, MUTED));
                panel
                    .spawn(Node {
                        width: percent(100),
                        column_gap: px(8),
                        ..default()
                    })
                    .with_children(|row| {
                        for (index, label) in GAMEMODES.iter().enumerate() {
                            let mut node = Node {
                                flex_grow: 1.0,
                                height: px(38),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            };
                            node.width = px(0);
                            row.spawn((
                                Button,
                                ThemeSkip,
                                FrontAction(Action::Gamemode(index as u8)),
                                GamemodeButton(index as u8),
                                node,
                                BackgroundColor(BUTTON_BASE),
                            ))
                            .with_children(|button| {
                                button.spawn(text(&font, label, 14.0, WHITE));
                            });
                        }
                    });
                create_choice_row(
                    panel,
                    &font,
                    "Loại thế giới",
                    Action::CycleWorldType,
                    WorldTypeLabel,
                );
                create_choice_row(
                    panel,
                    &font,
                    "Cho phép cheat",
                    Action::ToggleCheats,
                    CheatLabel,
                );
                create_choice_row(
                    panel,
                    &font,
                    "Chia sẻ thế giới (LAN)",
                    Action::ToggleShare,
                    ShareLabel,
                );
                panel.spawn(Node {
                    flex_grow: 1.0,
                    min_height: px(4),
                    ..default()
                });
                panel
                    .spawn(Node {
                        width: percent(100),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|footer| {
                        front_button(
                            footer,
                            &font,
                            "Tạo thế giới",
                            Action::CreateConfirm,
                            220.0,
                            44.0,
                        );
                        front_button(footer, &font, "Huỷ", Action::CreateCancel, 160.0, 44.0);
                    });
            });
        });

    commands
        .spawn((
            FrontRoot(RootKind::Confirm),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
            Visibility::Hidden,
            GlobalZIndex(45),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(520),
                    max_width: percent(88),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(16),
                    padding: UiRect::all(px(24)),
                    border_radius: BorderRadius::all(px(14)),
                    ..default()
                },
                sliced(theme.panel_plain.clone(), 14.0),
            ))
            .with_children(|panel| {
                panel.spawn((
                    ConfirmText,
                    Text::new(""),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(WHITE),
                ));
                panel.spawn(text(
                    &font,
                    "Thế giới sẽ được chuyển vào thùng rác saves/.trash.",
                    13.0,
                    MUTED,
                ));
                panel
                    .spawn(Node {
                        width: percent(100),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|row| {
                        front_button(row, &font, "Xoá thế giới", Action::ConfirmDelete, 200.0, 40.0);
                        front_button(row, &font, "Huỷ", Action::CancelDelete, 160.0, 40.0);
                    });
            });
        });
}

fn text(font: &Handle<Font>, value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn front_button(
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
            FrontAction(action),
            node,
            BackgroundColor(BUTTON_BASE),
        ))
        .with_children(|button| {
            button.spawn(text(font, label, 15.0, WHITE));
        });
}

fn field_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    index: u8,
    value: &str,
) {
    parent
        .spawn(Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|row| {
            row.spawn(text(font, label, 14.0, MUTED));
            row.spawn((
                Button,
                FrontAction(Action::FocusField(index)),
                Node {
                    width: px(470),
                    height: px(38),
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(px(12)),
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(TRACK_BG),
            ))
            .with_children(|field| {
                field.spawn((
                    FieldText(index),
                    Text::new(value),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(WHITE),
                ));
            });
        });
}

fn create_choice_row<M: Bundle>(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    action: Action,
    marker: M,
) {
    parent
        .spawn(Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|row| {
            row.spawn(text(font, label, 14.0, MUTED));
            row.spawn((
                Button,
                FrontAction(action),
                Node {
                    width: px(220),
                    height: px(38),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(BUTTON_BASE),
            ))
            .with_children(|button| {
                button.spawn((marker, text(font, "", 14.0, WHITE)));
            });
        });
}

fn front_actions(
    clicks: Query<&FrontAction, Added<Pressed>>,
    mut screen: ResMut<ScreenState>,
    mut form: ResMut<CreateForm>,
    mut menu: ResMut<MenuState>,
    mut worlds: ResMut<WorldsUi>,
    mut start: ResMut<StartRequest>,
    mut exit: MessageWriter<AppExit>,
    mut cursor: Single<&mut CursorOptions>,
    mut icons: ResMut<FrontIcons>,
) {
    let mut actions: Vec<Action> = clicks.iter().map(|action| action.0.clone()).collect();
    if form.submit {
        form.submit = false;
        actions.push(Action::CreateConfirm);
    }

    for action in actions {
        match action {
            Action::Singleplayer => {
                screen.screen = Screen::Worlds;
                worlds.dirty = true;
            }
            Action::Settings => {
                menu.open = true;
                menu.page = 1;
                menu.tab = 0;
                cursor.visible = true;
                cursor.grab_mode = CursorGrabMode::None;
            }
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            Action::BackToTitle => {
                screen.screen = Screen::Title;
            }
            Action::OpenCreate => {
                form.name = "Thế giới mới".into();
                form.seed.clear();
                form.gamemode = 0;
                form.world_type = 0;
                form.cheats = false;
                form.share = false;
                form.focus = 0;
                screen.screen = Screen::Create;
            }
            Action::CreateCancel => {
                screen.screen = Screen::Worlds;
                worlds.dirty = true;
            }
            Action::CreateConfirm => {
                if let Some(meta) = form_to_meta(&form) {
                    save_world(&meta);
                    start.0 = Some(meta);
                }
            }
            Action::PlayWorld(slug) => {
                if let Some(mut meta) = read_world(&slug) {
                    meta.played = now_secs();
                    save_world(&meta);
                    start.0 = Some(meta);
                }
            }
            Action::AskDelete(slug) => {
                worlds.pending_delete = Some(slug);
            }
            Action::ConfirmDelete => {
                if let Some(slug) = worlds.pending_delete.take() {
                    move_world_to_trash(&slug);
                    icons.0.remove(&slug);
                    worlds.dirty = true;
                }
            }
            Action::CancelDelete => {
                worlds.pending_delete = None;
            }
            Action::Gamemode(index) => {
                form.gamemode = index.min(3);
            }
            Action::CycleWorldType => {
                form.world_type = (form.world_type + 1) % WORLD_TYPES.len() as u8;
            }
            Action::ToggleCheats => {
                form.cheats = !form.cheats;
            }
            Action::ToggleShare => {
                form.share = !form.share;
            }
            Action::FocusField(index) => {
                form.focus = index.min(1);
            }
        }
    }
}

fn process_start_request(
    mut commands: Commands,
    mut screen: ResMut<ScreenState>,
    mut start: ResMut<StartRequest>,
    mut loading: ResMut<Loading>,
    mut rules: ResMut<GameRules>,
    mut menu: ResMut<MenuState>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
    render: Res<RenderSettings>,
    mut players: Query<&mut Player, With<Camera3d>>,
    mut spawn_point: ResMut<SpawnPoint>,
    mut cursor: Single<&mut CursorOptions>,
    mut active: ResMut<ActiveWorld>,
    mut day: ResMut<DayCycle>,
    mut warmup: ResMut<LoadWarmup>,
) {
    let Some(meta) = start.0.take() else {
        return;
    };
    if matches!(screen.screen, Screen::Loading | Screen::Playing) {
        return;
    }
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    worldgen::set_world_type(meta.world_type);
    let edits = load_edits(&meta.slug);
    let player_state = read_player(&meta.slug);
    let saved = player_state
        .map(|(feet, yaw, pitch, _)| (feet, yaw, pitch))
        .or_else(|| {
            if meta.py != 0.0 {
                Some((Vec3::new(meta.px, meta.py, meta.pz), meta.yaw, meta.pitch))
            } else {
                None
            }
        });
    let center = saved
        .map(|(feet, _, _)| {
            ChunkPos::from_block(feet.x.floor() as i32, feet.z.floor() as i32)
        })
        .unwrap_or(ChunkPos::new(0, 0));
    reset_world(
        &mut commands,
        &mut world,
        &mut renderer,
        &render,
        &mut player,
        &mut spawn_point,
        center,
        meta.seed,
        meta.gamemode,
        edits,
        saved,
        Some(&meta.slug),
    );
    warmup.0 = 15.0;
    day.time = player_state
        .map(|(_, _, _, time)| time)
        .filter(|time| *time > 0.0)
        .or_else(|| (meta.time > 0.0).then_some(meta.time))
        .unwrap_or_else(crate::daynight::configured_start_time);
    rules.gamemode = meta.gamemode;
    rules.cheats = meta.cheats;
    rules.share = meta.share;
    loading.mode = LoadMode::World {
        icon: !icon_path(&meta.slug).exists(),
        slug: meta.slug.clone(),
    };
    loading.label = format!("Đang vào {}...", meta.name);
    loading.elapsed = 0.0;
    loading.min = 2.0;
    menu.open = false;
    screen.screen = Screen::Loading;
    screen.world_load = true;
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
    active.0 = Some(meta);
}

fn reset_world(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    renderer: &mut ChunkRenderer,
    render: &RenderSettings,
    player: &mut Player,
    spawn_point: &mut SpawnPoint,
    center: ChunkPos,
    seed: u32,
    gamemode: u8,
    edits: HashMap<(i32, i32, i32), Block>,
    saved: Option<(Vec3, f32, f32)>,
    storage: Option<&str>,
) {
    renderer.reset(commands);
    *world = VoxelWorld::new(seed);
    if let Some(slug) = storage {
        world.set_storage(Some(crate::storage::WorldStorage::new(&world_dir(slug))));
    }
    world.load_edits(edits);
    for dx in -render.distance - 1..=render.distance + 1 {
        for dz in -render.distance - 1..=render.distance + 1 {
            world.load_chunk(ChunkPos::new(center.x + dx, center.z + dz));
        }
    }
    let (feet, yaw, pitch, spawn) = match saved {
        Some((feet, yaw, pitch)) => (feet, yaw, pitch, feet),
        None => {
            let spawn = world.find_land_spawn(center);
            (spawn, 0.35, -0.15, spawn)
        }
    };
    *spawn_point = SpawnPoint(spawn);
    player.feet = feet;
    player.prev_feet = feet;
    player.render_feet = feet;
    player.body = feet + Vec3::Y * EYE_HEIGHT;
    player.velocity = Vec3::ZERO;
    player.yaw = yaw;
    player.pitch = pitch;
    player.body_yaw = player.yaw;
    player.on_ground = false;
    player.horizontal_collision = false;
    player.swinging = false;
    player.swing = 0.0;
    player.sneaking = false;
    player.sprinting = false;
    player.swimming = false;
    player.pose = PlayerPose::Standing;
    player.tick_accumulator = 0.0;
    player.no_jump_delay = 0;
    player.sprint_trigger = 0;
    player.prev_forward = 0.0;
    player.tick_speed = 0.0;
    player.flying = gamemode == 1 || gamemode == 3;
    player.jump_trigger_time = 0;
    player.prev_jump = false;
    player.eye_height_current = EYE_HEIGHT;
    player.eye_height_prev = EYE_HEIGHT;
}


fn create_text_input(
    mut input: MessageReader<KeyboardInput>,
    screen: Res<ScreenState>,
    mut form: ResMut<CreateForm>,
) {
    if screen.screen != Screen::Create {
        return;
    }
    for event in input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Backspace => {
                let field = if form.focus == 0 { &mut form.name } else { &mut form.seed };
                field.pop();
            }
            KeyCode::Tab => {
                form.focus = (form.focus + 1) % 2;
            }
            KeyCode::Enter => {
                form.submit = true;
            }
            _ => {
                let Some(value) = event.text.as_ref() else {
                    continue;
                };
                let focused_name = form.focus == 0;
                let limit = if focused_name { 32 } else { 40 };
                let field = if focused_name { &mut form.name } else { &mut form.seed };
                for ch in value.chars() {
                    if ch.is_control() || field.chars().count() >= limit {
                        continue;
                    }
                    field.push(ch);
                }
            }
        }
    }
}

fn front_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut screen: ResMut<ScreenState>,
    mut worlds: ResMut<WorldsUi>,
    menu: Res<MenuState>,
) {
    if !keys.just_pressed(KeyCode::Escape) || menu.open {
        return;
    }
    if worlds.pending_delete.is_some() {
        worlds.pending_delete = None;
        return;
    }
    match screen.screen {
        Screen::Worlds | Screen::Create => {
            screen.screen = Screen::Title;
        }
        _ => {}
    }
}

fn advance_loading(
    time: Res<Time>,
    mut commands: Commands,
    mut screen: ResMut<ScreenState>,
    mut loading: ResMut<Loading>,
    post: Res<PostLoading>,
    renderer: Res<ChunkRenderer>,
    assets: Res<FrontAssets>,
    asset_server: Res<AssetServer>,
) {
    if screen.screen != Screen::Loading {
        return;
    }
    loading.elapsed += time.delta_secs();
    let meshed = renderer.view_count();
    let loaded = |state: Option<LoadState>| matches!(state, Some(LoadState::Loaded));
    let assets_ready = loaded(asset_server.get_load_state(&assets.font))
        && loaded(asset_server.get_load_state(&assets.title))
        && loaded(asset_server.get_load_state(&assets.panorama));
    let mesh_ok = match &loading.mode {
        LoadMode::Startup => assets_ready || loading.elapsed >= 5.0,
        LoadMode::World { .. } => meshed >= 90 || loading.elapsed >= loading.min + 4.0,
    };
    if loading.elapsed < loading.min || !mesh_ok {
        return;
    }
    let mode = loading.mode.clone();
    match mode {
        LoadMode::Startup => {
            screen.screen = post.target;
        }
        LoadMode::World { icon, slug } => {
            screen.screen = Screen::Playing;
            screen.world_load = false;
            if icon {
                commands.insert_resource(IconCapture {
                    path: icon_path(&slug),
                    timer: 0.0,
                    phase: 0,
                });
            }
        }
    }
}

fn sync_roots(
    screen: Res<ScreenState>,
    worlds: Res<WorldsUi>,
    mut roots: Query<(&FrontRoot, &mut Visibility)>,
) {
    for (root, mut visibility) in &mut roots {
        let visible = match root.0 {
            RootKind::Loading => screen.screen == Screen::Loading,
            RootKind::Title => screen.screen == Screen::Title,
            RootKind::Worlds => screen.screen == Screen::Worlds,
            RootKind::Create => screen.screen == Screen::Create,
            RootKind::Confirm => {
                screen.screen == Screen::Worlds && worlds.pending_delete.is_some()
            }
        };
        let target = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != target {
            *visibility = target;
        }
    }
}

fn on_screen_change(
    screen: Res<ScreenState>,
    mut last: Local<Option<Screen>>,
    mut hud: ResMut<HudVisible>,
    mut menu: ResMut<MenuState>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if *last == Some(screen.screen) {
        return;
    }
    *last = Some(screen.screen);
    if screen.screen == Screen::Playing {
        hud.0 = std::env::var("ELOSIUM_HUD_OFF").is_err();
        menu.open = false;
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    } else {
        hud.0 = false;
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

fn update_loading_ui(
    loading: Res<Loading>,
    renderer: Res<ChunkRenderer>,
    mut fills: Query<&mut Node, With<LoadingFill>>,
    mut texts: Query<&mut Text, With<LoadingText>>,
) {
    let mut fraction = (loading.elapsed / loading.min.max(0.1)).clamp(0.0, 1.0);
    if matches!(loading.mode, LoadMode::World { .. }) {
        fraction = fraction.max((renderer.view_count() as f32 / 120.0).min(1.0));
    }
    for mut node in &mut fills {
        node.width = percent(fraction * 100.0);
    }
    for mut text in &mut texts {
        text.0 = format!("{} {:.0}%", loading.label, fraction * 100.0);
    }
}

fn sync_create_labels(
    form: Res<CreateForm>,
    mut texts: Query<(
        &mut Text,
        Option<&FieldText>,
        Has<WorldTypeLabel>,
        Has<CheatLabel>,
        Has<ShareLabel>,
    )>,
    mut modes: Query<(&GamemodeButton, &mut BackgroundColor)>,
    mut first: Local<bool>,
) {
    if *first && !form.is_changed() {
        return;
    }
    *first = true;
    for (mut text, field, world_type, cheat, share) in &mut texts {
        if let Some(field) = field {
            let value = if field.0 == 0 { &form.name } else { &form.seed };
            let caret = if form.focus == field.0 { "|" } else { "" };
            text.0 = format!("{value}{caret}");
        } else if world_type {
            text.0 = WORLD_TYPES[form.world_type as usize % WORLD_TYPES.len()].into();
        } else if cheat {
            text.0 = if form.cheats { "Bật" } else { "Tắt" }.into();
        } else if share {
            text.0 = if form.share { "Bật" } else { "Tắt" }.into();
        }
    }
    for (mode, mut color) in &mut modes {
        *color = BackgroundColor(if mode.0 == form.gamemode {
            BUTTON_ACTIVE
        } else {
            BUTTON_BASE
        });
    }
}


fn update_confirm_text(
    worlds: Res<WorldsUi>,
    mut texts: Query<&mut Text, With<ConfirmText>>,
) {
    if !worlds.is_changed() {
        return;
    }
    let message = worlds.pending_delete.as_ref().map_or_else(String::new, |slug| {
        let name = read_world(slug)
            .map(|meta| meta.name)
            .unwrap_or_else(|| slug.clone());
        format!("Xoá thế giới \"{name}\"?")
    });
    for mut text in &mut texts {
        text.0 = message.clone();
    }
}

fn rebuild_world_list(
    mut commands: Commands,
    screen: Res<ScreenState>,
    mut worlds: ResMut<WorldsUi>,
    grid: Query<Entity, With<WorldsGrid>>,
    assets: Res<FrontAssets>,
    theme: Res<UiAssets>,
    mut images: ResMut<Assets<Image>>,
    mut icons: ResMut<FrontIcons>,
) {
    if screen.screen != Screen::Worlds || !worlds.dirty {
        return;
    }
    worlds.dirty = false;
    let Ok(grid) = grid.single() else {
        return;
    };
    commands.entity(grid).despawn_children();

    let mut list = load_worlds();
    if list.is_empty() {
        commands.entity(grid).with_children(|parent| {
            parent.spawn(text(
                &assets.font,
                "Chưa có thế giới nào. Bấm \"Tạo thế giới\" để bắt đầu.",
                15.0,
                MUTED,
            ));
        });
        return;
    }
    list.sort_by(|a, b| b.played.cmp(&a.played));

    let font = assets.font.clone();
    for meta in list {
        let icon = if let Some(handle) = icons.0.get(&meta.slug) {
            Some(handle.clone())
        } else if let Some(image) = load_world_icon(&meta.slug) {
            let handle = images.add(image);
            icons.0.insert(meta.slug.clone(), handle.clone());
            Some(handle)
        } else {
            None
        };
        let name = meta.name.clone();
        let gamemode = GAMEMODES[meta.gamemode as usize % GAMEMODES.len()];
        let world_type = WORLD_TYPES[meta.world_type as usize % WORLD_TYPES.len()];
        let info = format!("Seed {} · {} · {}", meta.seed, gamemode, world_type);
        let slug = meta.slug.clone();
        commands.entity(grid).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(300),
                        height: px(246),
                        flex_direction: FlexDirection::Column,
                        border_radius: BorderRadius::all(px(12)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    sliced(theme.panel_plain.clone(), 12.0),
                ))
                .with_children(|card| {
                    if let Some(icon) = icon {
                        card.spawn((
                            ImageNode {
                                image_mode: NodeImageMode::Stretch,
                                ..ImageNode::new(icon)
                            },
                            Node {
                                width: percent(100),
                                height: px(140),
                                ..default()
                            },
                        ));
                    } else {
                        card.spawn((
                            Node {
                                width: percent(100),
                                height: px(140),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(TRACK_BG),
                        ))
                        .with_children(|placeholder| {
                            placeholder.spawn(text(&font, &name, 16.0, MUTED));
                        });
                    }
                    card.spawn(Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(10)),
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|body| {
                        body.spawn(text(&font, &name, 17.0, WHITE));
                        body.spawn(text(&font, &info, 12.0, MUTED));
                        body.spawn(Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            ..default()
                        });
                        body.spawn(Node {
                            width: percent(100),
                            column_gap: px(8),
                            ..default()
                        })
                        .with_children(|row| {
                            front_button(row, &font, "Chơi", Action::PlayWorld(slug.clone()), 180.0, 32.0);
                            front_button(row, &font, "Xoá", Action::AskDelete(slug.clone()), 80.0, 32.0);
                        });
                    });
                });
        });
    }
}

fn world_list_scroll(
    mut wheel: MessageReader<MouseWheel>,
    screen: Res<ScreenState>,
    mut views: Query<(&ComputedNode, &mut ScrollPosition), With<WorldsScroll>>,
) {
    if screen.screen != Screen::Worlds {
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

fn capture_icon(
    time: Res<Time>,
    mut commands: Commands,
    icon: Option<ResMut<IconCapture>>,
    mut hud: ResMut<HudVisible>,
    screen: Res<ScreenState>,
) {
    let Some(mut icon) = icon else {
        return;
    };
    if screen.screen != Screen::Playing {
        return;
    }
    icon.timer += time.delta_secs();
    match icon.phase {
        0 if icon.timer > 1.4 => {
            hud.0 = false;
            icon.phase = 1;
            icon.timer = 0.0;
        }
        1 if icon.timer > 0.3 => {
            if let Some(parent) = icon.path.parent() {
                fs::create_dir_all(parent).ok();
            }
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(icon.path.to_string_lossy().to_string()));
            icon.phase = 2;
            icon.timer = 0.0;
        }
        2 if icon.timer > 0.4 => {
            hud.0 = std::env::var("ELOSIUM_HUD_OFF").is_err();
            commands.remove_resource::<IconCapture>();
        }
        _ => {}
    }
}

fn auto_test_hooks(
    screen: Res<ScreenState>,
    mut start: ResMut<StartRequest>,
    mut ran: Local<bool>,
) {
    if *ran || screen.screen != Screen::Title {
        return;
    }
    if let Ok(slug) = std::env::var("ELOSIUM_PLAY") {
        *ran = true;
        if let Some(meta) = read_world(slug.trim()) {
            start.0 = Some(meta);
        }
        return;
    }
    let Ok(value) = std::env::var("ELOSIUM_NEWWORLD") else {
        return;
    };
    *ran = true;
    let parts: Vec<&str> = value.split(';').collect();
    let name = parts
        .first()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .unwrap_or("Thế giới mới");
    let seed_text = parts.get(1).map(|part| part.trim()).unwrap_or("");
    let gamemode = parts
        .get(2)
        .and_then(|part| part.trim().parse::<u8>().ok())
        .unwrap_or(0);
    let world_type = parts
        .get(3)
        .and_then(|part| part.trim().parse::<u8>().ok())
        .unwrap_or(0);
    let meta = WorldMeta {
        slug: unique_slug(&slugify(name)),
        name: name.to_string(),
        seed: parse_seed(seed_text),
        seed_text: seed_text.to_string(),
        gamemode: gamemode.min(3),
        cheats: false,
        share: false,
        world_type: world_type.min(3),
        created: now_secs(),
        played: now_secs(),
        ..Default::default()
    };
    save_world(&meta);
    start.0 = Some(meta);
}

fn form_to_meta(form: &CreateForm) -> Option<WorldMeta> {
    let name = form.name.trim();
    let name = if name.is_empty() {
        "Thế giới mới"
    } else {
        name
    };
    let meta = WorldMeta {
        slug: unique_slug(&slugify(name)),
        name: name.to_string(),
        seed: parse_seed(&form.seed),
        seed_text: form.seed.trim().to_string(),
        gamemode: form.gamemode.min(3),
        cheats: form.cheats,
        share: form.share,
        world_type: form.world_type.min(3),
        created: now_secs(),
        played: now_secs(),
        ..Default::default()
    };
    Some(meta)
}

const SAVE_ROOT: &str = "saves";

fn world_dir(slug: &str) -> PathBuf {
    Path::new(SAVE_ROOT).join(slug)
}

fn icon_path(slug: &str) -> PathBuf {
    world_dir(slug).join("icon.png")
}

fn world_file(slug: &str) -> PathBuf {
    world_dir(slug).join("world.cfg")
}

fn level_path(slug: &str) -> PathBuf {
    world_dir(slug).join("level.dat")
}

fn level_backup_path(slug: &str) -> PathBuf {
    world_dir(slug).join("level.dat_old")
}

fn player_path(slug: &str) -> PathBuf {
    world_dir(slug).join("playerdata").join("player.dat")
}

fn write_player(meta: &WorldMeta, time: f32) {
    let path = player_path(&meta.slug);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut data = Vec::with_capacity(10 + 24);
    data.extend_from_slice(b"ELP1");
    for value in [meta.px, meta.py, meta.pz, meta.yaw, meta.pitch, time] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    fs::write(path, data).ok();
}

fn read_player(slug: &str) -> Option<(Vec3, f32, f32, f32)> {
    let data = fs::read(player_path(slug)).ok()?;
    if data.len() < 28 || &data[..4] != b"ELP1" {
        return None;
    }
    let mut cursor = 4;
    let mut values = [0f32; 6];
    for value in &mut values {
        *value = take_f32(&data, &mut cursor)?;
    }
    Some((
        Vec3::new(values[0], values[1], values[2]),
        values[3],
        values[4],
        values[5],
    ))
}

fn push_str(data: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(bytes);
}

fn write_level(meta: &WorldMeta) {
    let mut data = Vec::with_capacity(128);
    data.extend_from_slice(b"ELS3");
    push_str(&mut data, &meta.name);
    data.extend_from_slice(&meta.seed.to_le_bytes());
    push_str(&mut data, &meta.seed_text);
    data.push(meta.gamemode);
    data.push(meta.cheats as u8);
    data.push(meta.share as u8);
    data.push(meta.world_type);
    data.extend_from_slice(&meta.created.to_le_bytes());
    data.extend_from_slice(&meta.played.to_le_bytes());
    for value in [meta.px, meta.py, meta.pz, meta.yaw, meta.pitch, meta.time] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    let path = level_path(&meta.slug);
    if path.exists() {
        fs::copy(&path, level_backup_path(&meta.slug)).ok();
    }
    fs::write(path, data).ok();
}

fn read_level(slug: &str) -> Option<WorldMeta> {
    let data = fs::read(level_path(slug)).ok()?;
    if data.len() < 8 || &data[..4] != b"ELS3" {
        return None;
    }
    let mut cursor = 4;
    let name = take_str(&data, &mut cursor)?;
    let seed = take_u32(&data, &mut cursor)?;
    let seed_text = take_str(&data, &mut cursor)?;
    let gamemode = *data.get(cursor)?;
    cursor += 1;
    let cheats = *data.get(cursor)? != 0;
    cursor += 1;
    let share = *data.get(cursor)? != 0;
    cursor += 1;
    let world_type = *data.get(cursor)?;
    cursor += 1;
    let created = take_u64(&data, &mut cursor)?;
    let played = take_u64(&data, &mut cursor)?;
    let mut floats = [0f32; 6];
    for value in &mut floats {
        *value = take_f32(&data, &mut cursor)?;
    }
    Some(WorldMeta {
        slug: slug.to_string(),
        name,
        seed,
        seed_text,
        gamemode,
        cheats,
        share,
        world_type,
        created,
        played,
        px: floats[0],
        py: floats[1],
        pz: floats[2],
        yaw: floats[3],
        pitch: floats[4],
        time: floats[5],
    })
}

fn take_str(data: &[u8], cursor: &mut usize) -> Option<String> {
    let len = take_u32(data, cursor)? as usize;
    let bytes = data.get(*cursor..*cursor + len)?;
    *cursor += len;
    Some(String::from_utf8_lossy(bytes).to_string())
}

fn take_u32(data: &[u8], cursor: &mut usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(*cursor..*cursor + 4)?.try_into().ok()?;
    *cursor += 4;
    Some(u32::from_le_bytes(bytes))
}

fn take_u64(data: &[u8], cursor: &mut usize) -> Option<u64> {
    let bytes: [u8; 8] = data.get(*cursor..*cursor + 8)?.try_into().ok()?;
    *cursor += 8;
    Some(u64::from_le_bytes(bytes))
}

fn take_f32(data: &[u8], cursor: &mut usize) -> Option<f32> {
    let bytes: [u8; 4] = data.get(*cursor..*cursor + 4)?.try_into().ok()?;
    *cursor += 4;
    Some(f32::from_le_bytes(bytes))
}

fn save_world(meta: &WorldMeta) {
    fs::create_dir_all(world_dir(&meta.slug)).ok();
    write_level(meta);
}

fn read_world(slug: &str) -> Option<WorldMeta> {
    if let Some(meta) = read_level(slug) {
        return Some(meta);
    }
    let data = fs::read_to_string(world_file(slug)).ok()?;
    let mut meta = WorldMeta {
        slug: slug.to_string(),
        ..default()
    };
    for line in data.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "name" => meta.name = value.to_string(),
            "seed" => meta.seed = value.parse().unwrap_or(0),
            "seed_text" => meta.seed_text = value.to_string(),
            "gamemode" => meta.gamemode = value.parse().unwrap_or(0),
            "cheats" => meta.cheats = value == "1",
            "share" => meta.share = value == "1",
            "type" => meta.world_type = value.parse().unwrap_or(0),
            "created" => meta.created = value.parse().unwrap_or(0),
            "played" => meta.played = value.parse().unwrap_or(0),
            "px" => meta.px = value.parse().unwrap_or(0.0),
            "py" => meta.py = value.parse().unwrap_or(0.0),
            "pz" => meta.pz = value.parse().unwrap_or(0.0),
            "yaw" => meta.yaw = value.parse().unwrap_or(0.0),
            "pitch" => meta.pitch = value.parse().unwrap_or(0.0),
            "time" => meta.time = value.parse().unwrap_or(0.0),
            _ => {}
        }
    }
    if meta.name.is_empty() {
        meta.name = slug.to_string();
    }
    write_level(&meta);
    Some(meta)
}

fn load_worlds() -> Vec<WorldMeta> {    let Ok(entries) = fs::read_dir(SAVE_ROOT) else {
        return Vec::new();
    };
    let mut worlds = Vec::new();
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        if slug.starts_with('.') {
            continue;
        }
        if let Some(meta) = read_world(&slug) {
            worlds.push(meta);
        }
    }
    worlds
}

fn bake_world() -> Option<WorldMeta> {
    let worlds = load_worlds();
    if let Some(meta) = worlds
        .iter()
        .filter(|meta| meta.py != 0.0)
        .max_by_key(|meta| meta.played)
    {
        return Some(meta.clone());
    }
    worlds.into_iter().max_by_key(|meta| meta.played)
}

fn move_world_to_trash(slug: &str) {
    let dir = world_dir(slug);
    if !dir.exists() {
        return;
    }
    let trash = Path::new(SAVE_ROOT).join(".trash");
    fs::create_dir_all(&trash).ok();
    let target = trash.join(format!("{slug}-{}", now_secs()));
    fs::rename(&dir, target).ok();
}

fn load_world_icon(slug: &str) -> Option<Image> {
    let bytes = fs::read(icon_path(slug)).ok()?;
    Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Descriptor(ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        }),
        RenderAssetUsages::default(),
    )
    .ok()
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for character in name.trim().chars() {
        if character.is_alphanumeric() {
            for lowered in character.to_lowercase() {
                slug.push(lowered);
            }
        } else if matches!(character, ' ' | '-' | '_') {
            slug.push('-');
        }
        if slug.len() >= 40 {
            break;
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "world".into()
    } else {
        trimmed
    }
}

fn unique_slug(base: &str) -> String {
    if !world_dir(base).exists() {
        return base.to_string();
    }
    for index in 2..1000 {
        let candidate = format!("{base}-{index}");
        if !world_dir(&candidate).exists() {
            return candidate;
        }
    }
    format!("{base}-{}", now_secs())
}

fn random_seed() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mut value = nanos ^ 0x2545_f491_4f6c_dd1d;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) as u32
}

fn parse_seed(text: &str) -> u32 {
    let text = text.trim();
    if text.is_empty() {
        return random_seed();
    }
    if let Ok(value) = text.parse::<i64>() {
        return value as u32;
    }
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn edits_path(slug: &str) -> PathBuf {
    world_dir(slug).join("data").join("edits.dat")
}

fn legacy_edits_path(slug: &str) -> PathBuf {
    world_dir(slug).join("chunks.bin")
}

fn save_edits(slug: &str, world: &VoxelWorld) {
    let path = edits_path(slug);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut data = Vec::with_capacity(16 + world.edits().len() * 13);
    data.extend_from_slice(b"ELS1");
    data.extend_from_slice(&(world.edits().len() as u32).to_le_bytes());
    for (&(x, y, z), &block) in world.edits() {
        data.extend_from_slice(&x.to_le_bytes());
        data.extend_from_slice(&y.to_le_bytes());
        data.extend_from_slice(&z.to_le_bytes());
        data.push(block as u8);
    }
    fs::write(path, data).ok();
}

fn load_edits(slug: &str) -> HashMap<(i32, i32, i32), Block> {
    let mut edits = HashMap::new();
    let data = fs::read(edits_path(slug))
        .or_else(|_| fs::read(legacy_edits_path(slug)))
        .unwrap_or_default();
    if data.len() < 8 || &data[..4] != b"ELS1" {
        return edits;
    }
    let count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut cursor = 8usize;
    for _ in 0..count {
        if cursor + 13 > data.len() {
            break;
        }
        let x = i32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap());
        let y = i32::from_le_bytes(data[cursor + 4..cursor + 8].try_into().unwrap());
        let z = i32::from_le_bytes(data[cursor + 8..cursor + 12].try_into().unwrap());
        let block = data[cursor + 12];
        cursor += 13;
        let block = block_from_id(block);
        edits.insert((x, y, z), block);
    }
    edits
}

fn block_from_id(id: u8) -> Block {
    match id {
        0 => Block::Air,
        1 => Block::Bedrock,
        2 => Block::Deepslate,
        3 => Block::Tuff,
        4 => Block::Calcite,
        5 => Block::Stone,
        6 => Block::Granite,
        7 => Block::Diorite,
        8 => Block::Andesite,
        9 => Block::Gravel,
        10 => Block::Dirt,
        11 => Block::CoarseDirt,
        12 => Block::GrassBlock,
        13 => Block::Podzol,
        14 => Block::Mycelium,
        15 => Block::Mud,
        16 => Block::Clay,
        17 => Block::Sand,
        18 => Block::RedSand,
        19 => Block::Sandstone,
        20 => Block::RedSandstone,
        21 => Block::SnowBlock,
        22 => Block::Ice,
        23 => Block::PackedIce,
        24 => Block::MossBlock,
        25 => Block::Dripstone,
        26 => Block::CoalOre,
        27 => Block::DeepslateCoalOre,
        28 => Block::IronOre,
        29 => Block::DeepslateIronOre,
        30 => Block::CopperOre,
        31 => Block::DeepslateCopperOre,
        32 => Block::GoldOre,
        33 => Block::DeepslateGoldOre,
        34 => Block::RedstoneOre,
        35 => Block::DeepslateRedstoneOre,
        36 => Block::LapisOre,
        37 => Block::DeepslateLapisOre,
        38 => Block::DiamondOre,
        39 => Block::DeepslateDiamondOre,
        40 => Block::EmeraldOre,
        41 => Block::DeepslateEmeraldOre,
        42 => Block::Water,
        43 => Block::Lava,
        44 => Block::Cactus,
        45 => Block::OakLog,
        46 => Block::OakLeaves,
        47 => Block::BirchLog,
        48 => Block::BirchLeaves,
        49 => Block::SpruceLog,
        50 => Block::SpruceLeaves,
        51 => Block::JungleLog,
        52 => Block::JungleLeaves,
        53 => Block::AcaciaLog,
        54 => Block::AcaciaLeaves,
        55 => Block::DarkOakLog,
        56 => Block::DarkOakLeaves,
        57 => Block::CherryLog,
        58 => Block::CherryLeaves,
        59 => Block::BaobabLog,
        60 => Block::BaobabLeaves,
        61 => Block::BlackwoodLog,
        62 => Block::BlackwoodLeaves,
        63 => Block::EucalyptusLog,
        64 => Block::EucalyptusLeaves,
        65 => Block::MagnoliaLog,
        66 => Block::MagnoliaLeaves,
        67 => Block::MapleLog,
        68 => Block::MapleLeaves,
        69 => Block::PineLog,
        70 => Block::PineLeaves,
        71 => Block::RedwoodLog,
        72 => Block::RedwoodLeaves,
        73 => Block::WillowLog,
        74 => Block::WillowLeaves,
        75 => Block::DeadLog,
        76 => Block::ShortGrass,
        77 => Block::Fern,
        78 => Block::DeadBush,
        79 => Block::Dandelion,
        80 => Block::Poppy,
        81 => Block::Cornflower,
        82 => Block::OxeyeDaisy,
        83 => Block::Allium,
        84 => Block::AzureBluet,
        85 => Block::OrangeTulip,
        86 => Block::RedTulip,
        87 => Block::PinkTulip,
        88 => Block::WhiteTulip,
        89 => Block::LilyOfTheValley,
        90 => Block::RedMushroom,
        91 => Block::BrownMushroom,
        92 => Block::SugarCane,
        93 => Block::Chalk,
        94 => Block::ChalkGrass,
        95 => Block::Silt,
        96 => Block::Clover,
        97 => Block::Daisy,
        98 => Block::Cattail,
        99 => Block::PalmLog,
        100 => Block::PalmLeaves,
        101 => Block::JoshuaLog,
        102 => Block::JoshuaLeaves,
        103 => Block::LarchLog,
        104 => Block::LarchLeaves,
        105 => Block::FirLog,
        106 => Block::FirLeaves,
        107 => Block::CypressLog,
        108 => Block::CypressLeaves,
        109 => Block::AspenLog,
        110 => Block::AspenLeaves,
        111 => Block::JacarandaLog,
        112 => Block::JacarandaLeaves,
        113 => Block::MahoganyLog,
        114 => Block::MahoganyLeaves,
        115 => Block::EbonyLog,
        116 => Block::EbonyLeaves,
        117 => Block::WhiteMangroveLog,
        118 => Block::WhiteMangroveLeaves,
        119 => Block::CikaLog,
        120 => Block::CikaLeaves,
        121 => Block::SkyrisLog,
        122 => Block::SkyrisLeaves,
        123 => Block::HollyLog,
        124 => Block::HollyLeaves,
        125 => Block::Lavender,
        126 => Block::Rose,
        127 => Block::PinkDaffodil,
        128 => Block::HighGrass,
        129 => Block::DesertGrass,
        130 => Block::DuneGrass,
        131 => Block::DeadGrass,
        132 => Block::Glowflower,
        133 => Block::Sprout,
        134 => Block::Amaranth,
        135 => Block::CyanAmaranth,
        136 => Block::BlackRose,
        137 => Block::Crocus,
        138 => Block::IncanLily,
        139 => Block::JapaneseOrchid,
        140 => Block::BlueSage,
        141 => Block::PrairieGrass,
        142 => Block::BeachGrass,
        143 => Block::Hydrangea,
        144 => Block::FirecrackerFlower,
        145 => Block::PinkAllium,
        _ => Block::Air,
    }
}

fn persist_world(meta: &mut WorldMeta, player: &Player, world: &VoxelWorld, time: f32) {
    meta.px = player.feet.x;
    meta.py = player.feet.y;
    meta.pz = player.feet.z;
    meta.yaw = player.yaw;
    meta.pitch = player.pitch;
    meta.time = time;
    meta.played = now_secs();
    save_world(meta);
    write_player(meta, time);
    save_edits(&meta.slug, world);
}

fn autosave_world(
    time: Res<Time>,
    mut timer: ResMut<AutoSaveTimer>,
    screen: Res<ScreenState>,
    mut active: ResMut<ActiveWorld>,
    world: Res<VoxelWorld>,
    day: Res<DayCycle>,
    players: Query<&Player, With<Camera3d>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() || screen.screen != Screen::Playing {
        return;
    }
    let Some(meta) = active.0.as_mut() else {
        return;
    };
    if let Ok(player) = players.single() {
        persist_world(meta, player, &world, day.time);
    }
}

fn save_world_state(
    screen: Res<ScreenState>,
    mut active: ResMut<ActiveWorld>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<crate::streaming::ChunkRenderer>,
    mut commands: Commands,
    day: Res<DayCycle>,
    players: Query<&Player, With<Camera3d>>,
    mut last: Local<Option<Screen>>,
    mut exits: MessageReader<AppExit>,
) {
    let mut exiting = false;
    for _ in exits.read() {
        exiting = true;
    }
    let leaving = last
        .map(|previous| previous == Screen::Playing && screen.screen != Screen::Playing)
        .unwrap_or(false);
    *last = Some(screen.screen);
    if !(leaving || exiting) {
        return;
    }
    let Some(meta) = active.0.as_mut() else {
        return;
    };
    if let Ok(player) = players.single() {
        persist_world(meta, player, &world, day.time);
    }
    if leaving {
        active.0 = None;
        renderer.reset(&mut commands);
        world.clear_chunks();
    }
}



fn animate_panorama_rig(
    camera: Query<&GlobalTransform, (With<Camera3d>, Without<ReflectionCamera>, Without<PanoramaRig>)>,
    mut rig: Query<&mut Transform, With<PanoramaRig>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    let Ok(mut transform) = rig.single_mut() else {
        return;
    };
    transform.translation = camera.translation();
}

fn sync_camera_layers(
    screen: Res<ScreenState>,
    mut cameras: Query<&mut RenderLayers, (With<Camera3d>, Without<ReflectionCamera>)>,
    mut last: Local<Option<Screen>>,
) {
    if *last == Some(screen.screen) {
        return;
    }
    *last = Some(screen.screen);
    let layers = if screen.screen == Screen::Playing {
        RenderLayers::from_layers(&[0, 1, 3])
    } else {
        RenderLayers::layer(5)
    };
    for mut camera in &mut cameras {
        *camera = layers.clone();
    }
}

fn menu_camera_orientation(
    time: Res<Time>,
    screen: Res<ScreenState>,
    mut players: Query<&mut Player, With<Camera3d>>,
) {
    if !matches!(screen.screen, Screen::Title | Screen::Worlds | Screen::Create) {
        return;
    }
    for mut player in &mut players {
        player.yaw = time.elapsed_secs() * 0.02;
        player.pitch = 0.0;
        player.body_yaw = player.yaw;
        let menu_feet = Vec3::new(0.5, 300.0, 0.5);
        player.feet = menu_feet;
        player.prev_feet = menu_feet;
        player.render_feet = menu_feet;
        player.body = menu_feet + Vec3::Y * EYE_HEIGHT;
        player.velocity = Vec3::ZERO;
    }
}

const PANORAMA_FACES: [(&str, f32, f32); 6] = [
    ("front", 0.0, 0.0),
    ("right", -std::f32::consts::FRAC_PI_2, 0.0),
    ("back", std::f32::consts::PI, 0.0),
    ("left", std::f32::consts::FRAC_PI_2, 0.0),
    ("up", 0.0, std::f32::consts::FRAC_PI_2),
    ("down", 0.0, -std::f32::consts::FRAC_PI_2),
];

fn bake_panorama(
    time: Res<Time>,
    mut bake: ResMut<BakePanorama>,
    mut players: Query<&mut Player, (With<Camera3d>, Without<ReflectionCamera>)>,
    mut cameras: Query<Entity, (With<Camera3d>, Without<ReflectionCamera>)>,
    mut projections: Query<&mut Projection, (With<Camera3d>, Without<ReflectionCamera>)>,
    mut world: ResMut<VoxelWorld>,
    mut renderer: ResMut<ChunkRenderer>,
    render: Res<RenderSettings>,
    mut spawn_point: ResMut<SpawnPoint>,
    mut day: ResMut<DayCycle>,
    mut warmup: ResMut<LoadWarmup>,
    mut volumes: Query<&mut FogVolume, With<crate::environment::GlobalFogVolume>>,
    mut hud: ResMut<HudVisible>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if std::env::var("ELOSIUM_BAKE_PANORAMA").is_err() {
        return;
    }
    match bake.stage {
        0 => {
            hud.0 = false;
            if let Ok(entity) = cameras.single_mut() {
                commands.entity(entity).remove::<DistanceFog>();
            }
            for mut volume in &mut volumes {
                volume.density_factor = 0.0;
                volume.absorption = 0.0;
                volume.scattering = 0.0;
            }
            if let Ok(mut projection) = projections.single_mut()
                && let Projection::Perspective(perspective) = &mut *projection
            {
                perspective.fov = 90f32.to_radians();
            }
            if let Ok(mut player) = players.single_mut() {
                if let Some(meta) = bake_world() {
                    worldgen::set_world_type(meta.world_type);
                    let edits = load_edits(&meta.slug);
                    let player_state = read_player(&meta.slug);
                    let saved = player_state
                        .map(|(feet, yaw, pitch, _)| (feet, yaw, pitch))
                        .or_else(|| {
                            if meta.py != 0.0 {
                                Some((
                                    Vec3::new(meta.px, meta.py, meta.pz),
                                    meta.yaw,
                                    meta.pitch,
                                ))
                            } else {
                                None
                            }
                        });
                    let center = saved
                        .map(|(feet, _, _)| {
                            ChunkPos::from_block(feet.x.floor() as i32, feet.z.floor() as i32)
                        })
                        .unwrap_or(ChunkPos::new(0, 0));
                    reset_world(
                        &mut commands,
                        &mut world,
                        &mut renderer,
                        &render,
                        &mut player,
                        &mut spawn_point,
                        center,
                        meta.seed,
                        meta.gamemode,
                        edits,
                        saved,
                        Some(&meta.slug),
                    );
                    day.time = player_state
                        .map(|(_, _, _, time)| time)
                        .filter(|time| *time > 0.0)
                        .or_else(|| (meta.time > 0.0).then_some(meta.time))
                        .unwrap_or_else(crate::daynight::configured_start_time);
                    warmup.0 = 200.0;
                }
                if let Some(pos) = bake_position() {
                    player.feet = pos;
                    player.prev_feet = pos;
                    player.render_feet = pos;
                    player.body = pos + Vec3::Y * EYE_HEIGHT;
                    player.velocity = Vec3::ZERO;
                    player.flying = true;
                    player.on_ground = false;
                }
                bake.wait = std::env::var("ELOSIUM_BAKE_WAIT")
                    .ok()
                    .and_then(|value| value.parse::<f32>().ok())
                    .unwrap_or(180.0);
                player.yaw = PANORAMA_FACES[0].1;
                player.pitch = PANORAMA_FACES[0].2;
            }
            bake.stage = 1;
            bake.timer = 0.0;
            bake.frames = 0;
        }
        1 => {
            bake.timer += time.delta_secs();
            if bake.timer < bake.wait || renderer.view_count() < 90 {
                return;
            }
            bake.stage = 2;
            bake.frames = 0;
        }
        2 => {
            bake.timer += time.delta_secs();
            if bake.timer < 0.7 {
                return;
            }
            bake.timer = 0.0;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!(
                    "assets/panorama/bake_{}.png",
                    bake.face
                )));
            bake.face += 1;
            if bake.face >= PANORAMA_FACES.len() {
                bake.stage = 3;
                bake.timer = 0.0;
            } else if let Ok(mut player) = players.single_mut() {
                player.yaw = PANORAMA_FACES[bake.face].1;
                player.pitch = PANORAMA_FACES[bake.face].2;
            }
        }
        _ => {
            bake.timer += time.delta_secs();
            if bake.timer < 1.5 {
                return;
            }
            assemble_panorama_faces();
            exit.write(AppExit::Success);
        }
    }
}

fn bake_position() -> Option<Vec3> {
    let value = std::env::var("ELOSIUM_BAKE_POS").ok()?;
    let parts: Vec<f32> = value
        .split(',')
        .filter_map(|part| part.trim().parse().ok())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    Some(Vec3::new(parts[0], parts[1], parts[2]))
}

fn assemble_panorama_faces() {
    let mut faces = Vec::new();
    for (index, (name, _, _)) in PANORAMA_FACES.iter().enumerate() {
        let Ok(image) = image::open(format!("assets/panorama/bake_{index}.png")) else {
            continue;
        };
        let image = image.to_rgb8();
        if image.width() < 1000 || image.height() < 720 {
            continue;
        }
        let face = image::imageops::crop_imm(&image, 280, 0, 720, 720).to_image();
        face.save(format!("assets/panorama/face_{name}.png")).ok();
        faces.push((*name, face));
    }
    if faces.len() != PANORAMA_FACES.len() {
        return;
    }
    let mut atlas = image::RgbImage::new(2160, 1440);
    let order = ["down", "up", "front", "right", "back", "left"];
    for (slot, name) in order.iter().enumerate() {
        if let Some((_, face)) = faces.iter().find(|(face_name, _)| face_name == name) {
            image::imageops::replace(
                &mut atlas,
                face,
                (slot % 3) as i64 * 720,
                (slot / 3) as i64 * 720,
            );
        }
    }
    atlas.save("assets/panorama/panorama.png").ok();
}

