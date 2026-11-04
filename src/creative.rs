use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    ui::Pressed,
    ui_widgets::Button,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    blocks::{Block, CREATIVE_BLOCKS},
    frontend::{GameRules, Screen, ScreenState},
    hud::{Inventory, ItemAtlases, ItemStack},
    menu::MenuState,
};

const PAGE_SIZE: usize = 45;
const TABS: [&str; 4] = ["Tất cả", "Khối", "Tự nhiên", "Thực vật"];

const PANEL_BG: Color = Color::srgba(0.055, 0.065, 0.085, 0.97);
const PANEL_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);
const SLOT_BG: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);
const SLOT_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);
const TAB_IDLE: Color = Color::srgba(1.0, 1.0, 1.0, 0.05);
const TAB_ACTIVE: Color = Color::srgba(0.55, 0.70, 0.95, 0.22);
const TAB_ACTIVE_BORDER: Color = Color::srgba(0.70, 0.82, 1.0, 0.65);

#[derive(Resource, Default)]
pub struct CreativeState {
    pub open: bool,
    pub tab: usize,
    pub page: usize,
    pub query: String,
}

#[derive(Component)]
struct CreativeRoot;

#[derive(Component)]
struct CreativeTabButton(usize);

#[derive(Component)]
struct CreativeSlot(usize);

#[derive(Component)]
struct CreativeIcon;

#[derive(Component)]
struct CreativeSearchText;

#[derive(Component)]
struct CreativePageText;

pub struct CreativePlugin;

impl Plugin for CreativePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreativeState>()
            .add_systems(Startup, spawn_creative)
            .add_systems(
                Update,
                (
                    toggle_creative,
                    creative_typing,
                    scroll_creative,
                    refresh_creative,
                    click_creative,
                ),
            );
    }
}

fn spawn_creative(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    theme: Res<crate::ui_theme::UiAssets>,
) {
    let font: Handle<Font> = asset_server.load("fonts/mono.ttf");
    let muted = Color::srgba(0.72, 0.78, 0.88, 0.75);

    commands
        .spawn((
            CreativeRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: px(0.0),
                bottom: px(0.0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.015, 0.025, 0.45)),
            Visibility::Hidden,
            GlobalZIndex(40),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(520.0),
                    padding: UiRect::all(px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(9.0),
                    border: UiRect::all(px(1.0)),
                    border_radius: BorderRadius::all(px(12.0)),
                    ..default()
                },
                crate::ui_theme::sliced(theme.panel_plain.clone(), 14.0),
            ))
            .with_children(|panel| {
                panel
                    .spawn(Node {
                        width: percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: px(8.0),
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn((
                            Text::new("Kho sáng tạo"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(17.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.93, 0.95, 1.0)),
                        ));
                        for (index, label) in TABS.iter().enumerate() {
                            header
                                .spawn((
                                    CreativeTabButton(index),
                                    Button,
                                    Node {
                                        padding: UiRect::axes(px(9.0), px(4.0)),
                                        border: UiRect::all(px(1.0)),
                                        border_radius: BorderRadius::all(px(5.0)),
                                        ..default()
                                    },
                                    BackgroundColor(if index == 0 { TAB_ACTIVE } else { TAB_IDLE }),
                                    BorderColor::all(if index == 0 {
                                        TAB_ACTIVE_BORDER
                                    } else {
                                        SLOT_BORDER
                                    }),
                                ))
                                .with_children(|tab| {
                                    tab.spawn((
                                        Text::new(*label),
                                        TextFont {
                                            font: font.clone().into(),
                                            font_size: FontSize::Px(12.0),
                                            ..default()
                                        },
                                        TextColor(muted),
                                    ));
                                });
                        }
                    });

                panel.spawn((
                    crate::ui_theme::sliced(theme.divider.clone(), 8.0),
                    Node {
                        width: percent(100.0),
                        height: px(10.0),
                        ..default()
                    },
                ));

                panel.spawn((
                    CreativeSearchText,
                    Text::new("Tìm: "),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(muted),
                ));

                panel
                    .spawn(Node {
                        width: percent(100.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(4.0),
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|grid| {
                        for index in 0..PAGE_SIZE {
                            grid.spawn((
                                CreativeSlot(index),
                                Button,
                                crate::ui_theme::ThemeSkip,
                                Node {
                                    width: px(48.0),
                                    height: px(48.0),
                                    border: UiRect::all(px(1.0)),
                                    border_radius: BorderRadius::all(px(4.0)),
                                    ..default()
                                },
                                crate::ui_theme::sliced(theme.slot_empty.clone(), 14.0),
                                BorderColor::all(SLOT_BORDER),
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    CreativeIcon,
                                    ImageNode::default(),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(7.0),
                                        top: px(7.0),
                                        width: px(32.0),
                                        height: px(32.0),
                                        ..default()
                                    },
                                    Visibility::Hidden,
                                ));
                            });
                        }
                    });

                panel
                    .spawn(Node {
                        width: percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|footer| {
                        footer.spawn((
                            CreativePageText,
                            Text::new("Trang 1"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(muted),
                        ));
                        footer.spawn((
                            Text::new("E/Esc đóng · cuộn đổi trang · click lấy block · gõ để tìm"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(muted),
                        ));
                    });
            });
        });
}

fn toggle_creative(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    screen: Res<ScreenState>,
    rules: Res<GameRules>,
    menu: Res<MenuState>,
    chat: Res<crate::chat::ChatState>,
    mut state: ResMut<CreativeState>,
    mut cursor: Single<&mut CursorOptions>,
    mut root: Query<&mut Visibility, With<CreativeRoot>>,
) {
    if state.open && std::env::var("ELOSIUM_CREATIVE_TEST").is_err() {
        if keys.just_pressed(KeyCode::Escape) {
            state.open = false;
            keys.clear_just_pressed(KeyCode::Escape);
        }
    }
    if !state.open
        && screen.screen == Screen::Playing
        && rules.gamemode == 1
        && !menu.open
        && !chat.open
        && keys.just_pressed(KeyCode::KeyE)
    {
        state.open = true;
        state.page = 0;
    }
    if std::env::var("ELOSIUM_CREATIVE_TEST").is_ok()
        && screen.screen == Screen::Playing
        && rules.gamemode == 1
    {
        state.open = true;
    }
    if state.is_changed() {
        if state.open {
            cursor.visible = true;
            cursor.grab_mode = CursorGrabMode::None;
        } else if screen.screen == Screen::Playing && !menu.open {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
        }
    }
    let visible = if state.open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut current in &mut root {
        if *current != visible {
            *current = visible;
        }
    }
}

fn creative_typing(
    mut state: ResMut<CreativeState>,
    mut keyboard: MessageReader<KeyboardInput>,
) {
    if !state.open {
        return;
    }
    let mut changed = false;
    for event in keyboard.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(text) => {
                for character in text.chars() {
                    if character.is_ascii_graphic() {
                        state.query.push(character.to_ascii_lowercase());
                        changed = true;
                    }
                }
            }
            Key::Space => {
                state.query.push(' ');
                changed = true;
            }
            Key::Backspace => {
                if state.query.pop().is_some() {
                    changed = true;
                }
            }
            _ => {}
        }
    }
    if changed {
        state.page = 0;
    }
}

fn scroll_creative(
    mut wheel: MessageReader<MouseWheel>,
    mut state: ResMut<CreativeState>,
) {
    if !state.open {
        return;
    }
    let mut steps = 0;
    for event in wheel.read() {
        let amount = match event.unit {
            MouseScrollUnit::Line => event.y.signum(),
            MouseScrollUnit::Pixel => (event.y / 50.0).signum(),
        };
        steps += amount as i32;
    }
    if steps != 0 {
        let pages = filtered_blocks(&state).chunks(PAGE_SIZE).count().max(1);
        let page = state.page as i32 - steps;
        state.page = page.clamp(0, pages as i32 - 1) as usize;
    }
}

fn filtered_blocks(state: &CreativeState) -> Vec<Block> {
    CREATIVE_BLOCKS
        .iter()
        .copied()
        .filter(|block| {
            let name = format!("{:?}", block).to_lowercase();
            if !state.query.is_empty() && !name.contains(&state.query) {
                return false;
            }
            match state.tab {
                0 => true,
                1 => block.is_solid() && !block.is_plant() && !block.is_leaves(),
                2 => {
                    block.is_soil()
                        || block.is_stone_like()
                        || block.is_plant()
                        || block.is_leaves()
                        || matches!(
                            block,
                            Block::Sand
                                | Block::RedSand
                                | Block::Gravel
                                | Block::Clay
                                | Block::SnowBlock
                                | Block::Ice
                                | Block::PackedIce
                                | Block::Water
                                | Block::Lava
                        )
                }
                _ => block.is_plant() || block.is_leaves(),
            }
        })
        .collect()
}

fn refresh_creative(
    state: Res<CreativeState>,
    atlases: Option<Res<ItemAtlases>>,
    mut icons: Query<(&mut ImageNode, &mut Visibility), (With<CreativeIcon>, Without<CreativeRoot>)>,
    slots: Query<(&CreativeSlot, &Children), With<CreativeSlot>>,
    mut search_text: Query<&mut Text, (With<CreativeSearchText>, Without<CreativePageText>)>,
    mut page_text: Query<&mut Text, (With<CreativePageText>, Without<CreativeSearchText>)>,
    mut tabs: Query<(&CreativeTabButton, &mut BackgroundColor, &mut BorderColor)>,
    mut root: Query<&mut Visibility, With<CreativeRoot>>,
) {
    if !state.is_changed() {
        return;
    }
    let Some(atlases) = atlases else {
        return;
    };
    for mut visibility in &mut root {
        *visibility = if state.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !state.open {
        return;
    }
    let blocks = filtered_blocks(&state);
    let pages = blocks.chunks(PAGE_SIZE).count().max(1);
    let page = state.page.min(pages - 1);
    let visible = &blocks[(page * PAGE_SIZE).min(blocks.len())..];
    for (slot, children) in &slots {
        let block = visible.get(slot.0).copied();
        for child in children.iter() {
            if let Ok((mut image, mut visibility)) = icons.get_mut(child) {
                match block {
                    Some(block) => {
                        let (atlas, layout, index) = atlases.icon(block);
                        *image = ImageNode::from_atlas_image(atlas, TextureAtlas { layout, index });
                        *visibility = Visibility::Inherited;
                    }
                    None => {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
        }
    }
    for mut text in &mut search_text {
        text.0 = if state.query.is_empty() {
            "Tìm: ".to_string()
        } else {
            format!("Tìm: {}", state.query)
        };
    }
    for mut text in &mut page_text {
        text.0 = format!("Trang {}/{} · {} block", page + 1, pages, blocks.len());
    }
    for (tab, mut background, mut border) in &mut tabs {
        let active = tab.0 == state.tab;
        background.0 = if active { TAB_ACTIVE } else { TAB_IDLE };
        *border = BorderColor::all(if active { TAB_ACTIVE_BORDER } else { SLOT_BORDER });
    }
}

fn click_creative(
    mut state: ResMut<CreativeState>,
    mut inventory: ResMut<Inventory>,
    buttons: Query<(Entity, &CreativeTabButton, Has<Pressed>), With<Button>>,
    slots: Query<(Entity, &CreativeSlot, Has<Pressed>), With<Button>>,
    mut held: Local<std::collections::HashSet<Entity>>,
) {
    if !state.open {
        held.clear();
        return;
    }
    for (entity, tab, pressed) in &buttons {
        if pressed {
            if held.insert(entity) {
                state.tab = tab.0;
                state.page = 0;
            }
        } else {
            held.remove(&entity);
        }
    }
    let blocks = filtered_blocks(&state);
    let page = state
        .page
        .min(blocks.chunks(PAGE_SIZE).count().max(1) - 1);
    for (entity, slot, pressed) in &slots {
        if pressed {
            if held.insert(entity) {
                let index = page * PAGE_SIZE + slot.0;
                if let Some(block) = blocks.get(index).copied() {
                    let selected = inventory.selected;
                    inventory.slots[selected] = Some(ItemStack { block, count: 64 });
                }
            }
        } else {
            held.remove(&entity);
        }
    }
}
