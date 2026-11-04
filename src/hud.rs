use bevy::{
    asset::RenderAssetUsages,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    blocks::Block,
    chat::ChatState,
    creative::CreativeState,
    debug::HudVisible,
    environment::EnvironmentState,
    frontend::{GameRules, Screen, ScreenState},
    menu::MenuState,
    pack_atlas::{BLOCK_UV_RECTS, FLORA_UV_RECTS, LEAF_UV_RECTS},
    particles::BlockDrop,
    player::Player,
    reflection::ReflectionCamera,
};

pub const HOTBAR_SLOTS: usize = 9;

#[derive(Clone, Copy, Debug)]
pub struct ItemStack {
    pub block: Block,
    pub count: u32,
}

#[derive(Resource)]
pub struct Inventory {
    pub slots: [Option<ItemStack>; HOTBAR_SLOTS],
    pub selected: usize,
}

impl Inventory {
    pub fn add_block(&mut self, block: Block) -> bool {
        for slot in &mut self.slots {
            if let Some(stack) = slot {
                if stack.block == block && stack.count < 64 {
                    stack.count += 1;
                    return true;
                }
            }
        }
        for slot in &mut self.slots {
            if slot.is_none() {
                *slot = Some(ItemStack { block, count: 1 });
                return true;
            }
        }
        false
    }

    pub fn add_blocks(&mut self, block: Block, mut count: u32) -> u32 {
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            if let Some(stack) = slot {
                if stack.block == block && stack.count < 64 {
                    let room = 64 - stack.count;
                    let moved = room.min(count);
                    stack.count += moved;
                    count -= moved;
                }
            }
        }
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            if slot.is_none() {
                let moved = count.min(64);
                *slot = Some(ItemStack { block, count: moved });
                count -= moved;
            }
        }
        count
    }

    pub fn take_selected(&mut self) -> Option<Block> {
        let slot = self.slots[self.selected].as_mut()?;
        let block = slot.block;
        slot.count -= 1;
        if slot.count == 0 {
            self.slots[self.selected] = None;
        }
        Some(block)
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [None; HOTBAR_SLOTS],
            selected: 0,
        }
    }
}

#[derive(Resource)]
pub struct Vitals {
    pub health: f32,
    pub armor: f32,
    pub mana: f32,
    pub stamina: f32,
    pub hunger: f32,
    pub thirst: f32,
    pub xp: f32,
    pub level: u32,
    pub exhaustion: f32,
    pub regen_timer: f32,
    pub starve_timer: f32,
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            health: 20.0,
            armor: 20.0,
            mana: 100.0,
            stamina: 100.0,
            hunger: 20.0,
            thirst: 20.0,
            xp: 0.0,
            level: 0,
            exhaustion: 0.0,
            regen_timer: 0.0,
            starve_timer: 0.0,
        }
    }
}

#[derive(Component)]
pub struct GameHud;

#[derive(Component, Clone, Copy)]
pub struct SlotButton(pub usize);

#[derive(Component)]
pub struct ItemIcon(pub usize);

#[derive(Component)]
pub struct ItemLabel;

#[derive(Component)]
pub struct ItemLabelText;

#[derive(Resource)]
pub struct ItemAtlases {
    block: AtlasSet,
    flora: AtlasSet,
    leaves: AtlasSet,
}

struct AtlasSet {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    columns: u32,
    rows: u32,
}

impl ItemAtlases {
    pub fn icon(&self, block: Block) -> (Handle<Image>, Handle<TextureAtlasLayout>, usize) {
        let (set, rects, face) = if block.is_plant() {
            (&self.flora, FLORA_UV_RECTS.as_slice(), 0)
        } else if block.is_leaves() {
            (&self.leaves, LEAF_UV_RECTS.as_slice(), 0)
        } else {
            (&self.block, BLOCK_UV_RECTS.as_slice(), 1)
        };
        let tile = (block.texture(face) as usize).min(rects.len() - 1);
        let rect = rects[tile];
        let x = (rect[0] * set.columns as f32).round() as usize;
        let y = (rect[1] * set.rows as f32).round() as usize;
        let index = y * set.columns as usize + x;
        (set.image.clone(), set.layout.clone(), index)
    }
}

#[derive(Component, Clone, Copy)]
pub struct BarFill(BarKind);

#[derive(Component)]
pub struct LevelText;

#[derive(Clone, Copy)]
enum BarKind {
    Health,
    Armor,
    Mana,
    Stamina,
    Hunger,
    Thirst,
    Xp,
}

impl BarKind {
    fn color(self) -> Color {
        match self {
            Self::Health => Color::srgb(0.86, 0.22, 0.28),
            Self::Armor => Color::srgb(0.70, 0.78, 0.86),
            Self::Mana => Color::srgb(0.42, 0.60, 1.0),
            Self::Stamina => Color::srgb(0.45, 0.86, 0.55),
            Self::Hunger => Color::srgb(0.92, 0.66, 0.34),
            Self::Thirst => Color::srgb(0.35, 0.76, 0.92),
            Self::Xp => Color::srgb(0.51, 0.90, 0.36),
        }
    }

    fn fraction(self, vitals: &Vitals) -> f32 {
        let value = match self {
            Self::Health => vitals.health / 20.0,
            Self::Armor => vitals.armor / 20.0,
            Self::Mana => vitals.mana / 100.0,
            Self::Stamina => vitals.stamina / 100.0,
            Self::Hunger => vitals.hunger / 20.0,
            Self::Thirst => vitals.thirst / 20.0,
            Self::Xp => vitals.xp,
        };
        value.clamp(0.0, 1.0)
    }
}

const TRACK_BG: Color = Color::srgba(0.04, 0.05, 0.07, 0.55);
const TRACK_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);
const SLOT_BG: Color = Color::srgba(1.0, 1.0, 1.0, 0.05);
const SLOT_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.13);
const SLOT_SELECTED_BG: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
const SLOT_SELECTED_BORDER: Color = Color::srgb(0.95, 0.97, 1.0);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<Vitals>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (
                    select_hotbar,
                    sync_hud_visibility,
                    update_hotbar_selection,
                    update_slot_icons,
                    update_item_label,
                    update_vitals,
                    update_bars,
                ),
            );
    }
}

fn nearest_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    })
}

fn pixel_icon(images: &mut Assets<Image>, rows: [&str; 9], color: Color) -> Handle<Image> {
    let srgba = color.to_srgba();
    let mut data = Vec::with_capacity(9 * 9 * 4);
    for row in rows {
        for pixel in row.chars() {
            if pixel == '#' {
                data.extend_from_slice(&[
                    (srgba.red * 255.0) as u8,
                    (srgba.green * 255.0) as u8,
                    (srgba.blue * 255.0) as u8,
                    255,
                ]);
            } else {
                data.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: 9,
            height: 9,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = nearest_sampler();
    images.add(image)
}

fn spawn_bar(
    parent: &mut ChildSpawnerCommands,
    kind: BarKind,
    icon: Handle<Image>,
    icon_outer_right: bool,
) {
    parent
        .spawn(Node {
            width: px(126.0),
            height: px(14.0),
            display: Display::Flex,
            flex_direction: if icon_outer_right {
                FlexDirection::RowReverse
            } else {
                FlexDirection::Row
            },
            align_items: AlignItems::Center,
            column_gap: px(4.0),
            ..default()
        })
        .with_children(|bar| {
            bar.spawn((
                ImageNode {
                    image_mode: NodeImageMode::Stretch,
                    ..ImageNode::new(icon)
                },
                Node {
                    width: px(14.0),
                    height: px(14.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            bar.spawn((
                Node {
                    flex_grow: 1.0,
                    height: px(9.0),
                    border: UiRect::all(px(1.0)),
                    border_radius: BorderRadius::all(px(3.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(TRACK_BG),
                BorderColor::all(TRACK_BORDER),
            ))
            .with_children(|track| {
                track.spawn((
                    BarFill(kind),
                    Node {
                        width: percent(100.0),
                        height: percent(100.0),
                        border_radius: BorderRadius::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(kind.color()),
                ));
            });
        });
}

fn spawn_hud(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
) {
    let pixel = UVec2::splat(16);
    commands.insert_resource(ItemAtlases {
        block: AtlasSet {
            image: asset_server.load("textures/atlas.png"),
            layout: layouts.add(TextureAtlasLayout::from_grid(pixel, 64, 4, None, None)),
            columns: 64,
            rows: 4,
        },
        flora: AtlasSet {
            image: asset_server.load("textures/atlas_flora.png"),
            layout: layouts.add(TextureAtlasLayout::from_grid(pixel, 32, 4, None, None)),
            columns: 32,
            rows: 4,
        },
        leaves: AtlasSet {
            image: asset_server.load("textures/atlas_leaves.png"),
            layout: layouts.add(TextureAtlasLayout::from_grid(pixel, 64, 4, None, None)),
            columns: 64,
            rows: 4,
        },
    });
    let heart = pixel_icon(
        &mut images,
        [
            "..##.##..",
            ".#######.",
            "#########",
            "#########",
            ".#######.",
            "..#####..",
            "...###...",
            "....#....",
            ".........",
        ],
        Color::srgb(0.94, 0.30, 0.34),
    );
    let armor = pixel_icon(
        &mut images,
        [
            "#########",
            "#########",
            "##.###.##",
            "##.###.##",
            ".#######.",
            ".#######.",
            "..#####..",
            "...###...",
            "....#....",
        ],
        Color::srgb(0.76, 0.83, 0.91),
    );
    let mana = pixel_icon(
        &mut images,
        [
            "....#....",
            "...###...",
            "..#####..",
            ".#######.",
            "#########",
            ".#######.",
            "..#####..",
            "...###...",
            "....#....",
        ],
        Color::srgb(0.52, 0.68, 1.0),
    );
    let stamina = pixel_icon(
        &mut images,
        [
            ".....####",
            "....####.",
            "...####..",
            "..#####..",
            "...####..",
            "....####.",
            ".....###.",
            "......#..",
            ".........",
        ],
        Color::srgb(0.55, 0.91, 0.63),
    );
    let hunger = pixel_icon(
        &mut images,
        [
            "..###....",
            ".#####...",
            ".######..",
            "..#####..",
            "..#####..",
            ".#####...",
            "#####....",
            "###......",
            "#........",
        ],
        Color::srgb(0.96, 0.72, 0.42),
    );
    let thirst = pixel_icon(
        &mut images,
        [
            "....#....",
            "....#....",
            "...###...",
            "..#####..",
            ".#######.",
            ".#######.",
            "..#####..",
            "...###...",
            ".........",
        ],
        Color::srgb(0.40, 0.80, 0.97),
    );
    let font: Handle<Font> = asset_server.load("fonts/mono.ttf");

    commands
        .spawn((
            ItemLabel,
            Node {
                position_type: PositionType::Absolute,
                left: px(-200.0),
                top: px(-200.0),
                padding: UiRect::axes(px(6.0), px(3.0)),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.72)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.14)),
            Visibility::Hidden,
            GlobalZIndex(21),
        ))
        .with_children(|label| {
            label.spawn((
                ItemLabelText,
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.96, 1.0)),
            ));
        });

    commands
        .spawn((
            GameHud,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                bottom: px(0.0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                column_gap: px(14.0),
                padding: UiRect::bottom(px(14.0)),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: px(5.0),
                ..default()
            })
            .with_children(|column| {
                spawn_bar(column, BarKind::Mana, mana.clone(), false);
                spawn_bar(column, BarKind::Armor, armor.clone(), false);
                spawn_bar(column, BarKind::Health, heart.clone(), false);
            });

            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(5.0),
                ..default()
            })
            .with_children(|center| {
                center.spawn((
                    LevelText,
                    Text::new("0"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 1.0, 0.55)),
                ));
                center
                    .spawn((
                        Node {
                            width: px(364.0),
                            height: px(7.0),
                            border: UiRect::all(px(1.0)),
                            border_radius: BorderRadius::all(px(3.0)),
                            ..default()
                        },
                        BackgroundColor(TRACK_BG),
                        BorderColor::all(TRACK_BORDER),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            BarFill(BarKind::Xp),
                            Node {
                                width: percent(0.0),
                                height: percent(100.0),
                                border_radius: BorderRadius::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(BarKind::Xp.color()),
                        ));
                    });

                center
                    .spawn((
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            column_gap: px(2.0),
                            padding: UiRect::all(px(3.0)),
                            border: UiRect::all(px(1.0)),
                            border_radius: BorderRadius::all(px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.60)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.08)),
                    ))
                    .with_children(|hotbar| {
                        for index in 0..HOTBAR_SLOTS {
                            let selected = index == 0;
                            hotbar
                                .spawn((
                                    SlotButton(index),
                                    Node {
                                        width: px(38.0),
                                        height: px(38.0),
                                        border: UiRect::all(px(1.0)),
                                        border_radius: BorderRadius::all(px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(if selected { SLOT_SELECTED_BG } else { SLOT_BG }),
                                    BorderColor::all(if selected {
                                        SLOT_SELECTED_BORDER
                                    } else {
                                        SLOT_BORDER
                                    }),
                                ))
                                .with_children(|slot| {
                                    slot.spawn((
                                        ItemIcon(index),
                                        ImageNode::default(),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: px(4.0),
                                            top: px(4.0),
                                            width: px(28.0),
                                            height: px(28.0),
                                            ..default()
                                        },
                                        Visibility::Hidden,
                                    ));
                                });
                        }
                    });
            });

            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: px(5.0),
                ..default()
            })
            .with_children(|column| {
                spawn_bar(column, BarKind::Thirst, thirst.clone(), true);
                spawn_bar(column, BarKind::Hunger, hunger.clone(), true);
                spawn_bar(column, BarKind::Stamina, stamina.clone(), true);
            });
        });
}

fn select_hotbar(
    screen: Res<ScreenState>,
    creative: Res<CreativeState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut inventory: ResMut<Inventory>,
) {
    if screen.screen != Screen::Playing || creative.open {
        return;
    }
    const DIGITS: [KeyCode; HOTBAR_SLOTS] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    for (index, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key) {
            inventory.selected = index;
        }
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
        inventory.selected =
            (inventory.selected as i32 - steps).rem_euclid(HOTBAR_SLOTS as i32) as usize;
    }
}

fn sync_hud_visibility(
    screen: Res<ScreenState>,
    hud: Res<HudVisible>,
    mut root: Query<&mut Visibility, With<GameHud>>,
) {
    let visible = screen.screen == Screen::Playing && hud.0;
    let target = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut root {
        if *visibility != target {
            *visibility = target;
        }
    }
}

fn update_hotbar_selection(
    inventory: Res<Inventory>,
    mut slots: Query<(&SlotButton, &mut BorderColor, &mut BackgroundColor)>,
) {
    if !inventory.is_changed() {
        return;
    }
    for (slot, mut border, mut background) in &mut slots {
        let selected = slot.0 == inventory.selected;
        *border = BorderColor::all(if selected {
            SLOT_SELECTED_BORDER
        } else {
            SLOT_BORDER
        });
        background.0 = if selected {
            SLOT_SELECTED_BG
        } else {
            SLOT_BG
        };
    }
}

fn update_bars(
    vitals: Res<Vitals>,
    mut fills: Query<(&BarFill, &mut Node)>,
    mut level: Query<&mut Text, With<LevelText>>,
) {
    for (fill, mut node) in &mut fills {
        node.width = percent(fill.0.fraction(&vitals) * 100.0);
    }
    if vitals.is_changed() {
        for mut text in &mut level {
            text.0 = vitals.level.to_string();
        }
    }
}

fn update_slot_icons(
    inventory: Res<Inventory>,
    atlases: Res<ItemAtlases>,
    mut icons: Query<(&ItemIcon, &mut ImageNode, &mut Visibility)>,
) {
    if !inventory.is_changed() {
        return;
    }
    for (icon, mut image, mut visibility) in &mut icons {
        match inventory.slots[icon.0] {
            Some(stack) => {
                let (atlas, layout, index) = atlases.icon(stack.block);
                *image = ImageNode::from_atlas_image(atlas, TextureAtlas { layout, index });
                *visibility = Visibility::Inherited;
            }
            None => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn update_vitals(
    time: Res<Time>,
    rules: Res<GameRules>,
    menu: Res<MenuState>,
    chat: Res<ChatState>,
    environment: Res<EnvironmentState>,
    players: Query<&Player, With<Camera3d>>,
    mut vitals: ResMut<Vitals>,
) {
    if rules.gamemode != 0 || menu.open || chat.open {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let dt = time.delta_secs().min(0.1);
    if dt <= 0.0 {
        return;
    }
    let speed = player.speed_mps();
    let sprinting = player.sprinting && speed > 1.0;

    vitals.stamina = if sprinting {
        (vitals.stamina - 14.0 * dt).max(0.0)
    } else {
        (vitals.stamina + 9.0 * dt).min(100.0)
    };
    vitals.mana = (vitals.mana + 3.0 * dt).min(100.0);

    let thirst_rate = if sprinting { 0.05 } else { 0.02 };
    if environment.underwater {
        vitals.thirst = (vitals.thirst + 8.0 * dt).min(20.0);
    } else {
        vitals.thirst = (vitals.thirst - thirst_rate * dt).max(0.0);
    }

    let distance = speed * dt;
    vitals.exhaustion += distance * if sprinting { 0.1 } else { 0.01 };
    while vitals.exhaustion >= 4.0 {
        vitals.exhaustion -= 4.0;
        vitals.hunger = (vitals.hunger - 1.0).max(0.0);
    }

    if vitals.health < 20.0 && vitals.hunger >= 18.0 && vitals.thirst >= 6.0 {
        vitals.regen_timer += dt;
        if vitals.regen_timer >= 4.0 {
            vitals.regen_timer = 0.0;
            vitals.health = (vitals.health + 1.0).min(20.0);
            vitals.exhaustion += 6.0;
        }
    } else {
        vitals.regen_timer = 0.0;
    }

    if vitals.hunger <= 0.0 || vitals.thirst <= 0.0 {
        vitals.starve_timer += dt;
        if vitals.starve_timer >= 4.0 {
            vitals.starve_timer = 0.0;
            vitals.health = (vitals.health - 1.0).max(0.0);
        }
    } else {
        vitals.starve_timer = 0.0;
    }
}

fn block_label(block: Block) -> String {
    let raw = format!("{:?}", block);
    let mut out = String::with_capacity(raw.len() + 6);
    for (index, character) in raw.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            out.push(' ');
        }
        out.push(character);
    }
    out
}

fn ray_hits_item(origin: Vec3, direction: Vec3, center: Vec3, half: f32, max: f32) -> Option<f32> {
    let origin = origin.to_array();
    let direction = direction.to_array();
    let minimum = (center - Vec3::splat(half)).to_array();
    let maximum = (center + Vec3::splat(half)).to_array();
    let mut near = 0.0f32;
    let mut far = max;
    for axis in 0..3 {
        if direction[axis].abs() < 1e-6 {
            if origin[axis] < minimum[axis] || origin[axis] > maximum[axis] {
                return None;
            }
        } else {
            let mut first = (minimum[axis] - origin[axis]) / direction[axis];
            let mut second = (maximum[axis] - origin[axis]) / direction[axis];
            if first > second {
                std::mem::swap(&mut first, &mut second);
            }
            near = near.max(first);
            far = far.min(second);
            if near > far {
                return None;
            }
        }
    }
    Some(near)
}

fn update_item_label(
    screen: Res<ScreenState>,
    menu: Res<MenuState>,
    creative: Res<CreativeState>,
    hud: Res<HudVisible>,
    camera: Query<
        (&GlobalTransform, &Camera),
        (With<Camera3d>, With<Player>, Without<ReflectionCamera>),
    >,
    items: Query<(&Transform, &BlockDrop), Without<ItemLabel>>,
    mut label: Query<(&mut Node, &mut Visibility), With<ItemLabel>>,
    mut text: Query<&mut Text, With<ItemLabelText>>,
) {
    let Ok((mut node, mut visibility)) = label.single_mut() else {
        return;
    };
    let active = screen.screen == Screen::Playing && hud.0 && !menu.open && !creative.open;
    if !active {
        *visibility = Visibility::Hidden;
        return;
    }
    let Ok((camera_transform, camera)) = camera.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let origin = camera_transform.translation();
    let direction = camera_transform.rotation() * Vec3::NEG_Z;
    let mut best: Option<(f32, Vec3, Block, u32)> = None;
    for (transform, drop) in &items {
        if let Some(distance) = ray_hits_item(origin, direction, transform.translation, 0.2, 4.5) {
            if best.map(|found| distance < found.0).unwrap_or(true) {
                best = Some((distance, transform.translation, drop.block, drop.count));
            }
        }
    }
    let Some((_, position, block, count)) = best else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok(point) = camera.world_to_viewport(camera_transform, position + Vec3::Y * 0.42) else {
        *visibility = Visibility::Hidden;
        return;
    };
    node.left = px(point.x - 44.0);
    node.top = px(point.y - 30.0);
    for mut current in &mut text {
        current.0 = format!("{} x{}", block_label(block), count);
    }
    *visibility = Visibility::Inherited;
}
