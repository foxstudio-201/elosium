use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    ui::{InteractionDisabled, Pressed, UiGlobalTransform},
    ui_widgets::Button,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

#[derive(Resource, Clone)]
pub struct UiAssets {
    pub panel_plain: Handle<Image>,
    pub panel_inset: Handle<Image>,
    pub panel_ornate: Handle<Image>,
    pub header_banner: Handle<Image>,
    pub divider: Handle<Image>,
    pub tooltip: Handle<Image>,
    pub slot_empty: Handle<Image>,
    pub slot_hover: Handle<Image>,
    pub slot_selected: Handle<Image>,
    pub btn_normal: Handle<Image>,
    pub btn_hover: Handle<Image>,
    pub btn_pressed: Handle<Image>,
    pub btn_disabled: Handle<Image>,
    pub btn_icon_normal: Handle<Image>,
    pub btn_icon_hover: Handle<Image>,
    pub btn_icon_pressed: Handle<Image>,
    pub btn_icon_disabled: Handle<Image>,
    pub checkbox_off: Handle<Image>,
    pub checkbox_on: Handle<Image>,
    pub toggle_off: Handle<Image>,
    pub toggle_on: Handle<Image>,
    pub bar_frame: Handle<Image>,
    pub bar_fill_hp: Handle<Image>,
    pub bar_fill_mp: Handle<Image>,
    pub bar_segments: Handle<Image>,
    pub cursor_pointer: Handle<Image>,
}

fn load(server: &AssetServer, file: &str) -> Handle<Image> {
    load_path(server, format!("ui/emberhold/{file}"))
}

fn load_path(server: &AssetServer, path: String) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::linear();
        })
        .load::<Image>(path)
}

impl FromWorld for UiAssets {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        Self {
            panel_plain: load(server, "panel_plain.png"),
            panel_inset: load(server, "panel_inset.png"),
            panel_ornate: load(server, "panel_ornate.png"),
            header_banner: load(server, "header_banner.png"),
            divider: load(server, "divider.png"),
            tooltip: load(server, "tooltip.png"),
            slot_empty: load(server, "slot_empty.png"),
            slot_hover: load(server, "slot_hover.png"),
            slot_selected: load(server, "slot_selected.png"),
            btn_normal: load(server, "btn_standard_normal.png"),
            btn_hover: load(server, "btn_standard_hover.png"),
            btn_pressed: load(server, "btn_standard_pressed.png"),
            btn_disabled: load(server, "btn_standard_disabled.png"),
            btn_icon_normal: load(server, "btn_icon_normal.png"),
            btn_icon_hover: load(server, "btn_icon_hover.png"),
            btn_icon_pressed: load(server, "btn_icon_pressed.png"),
            btn_icon_disabled: load(server, "btn_icon_disabled.png"),
            checkbox_off: load(server, "checkbox_off.png"),
            checkbox_on: load(server, "checkbox_on.png"),
            toggle_off: load(server, "toggle_off.png"),
            toggle_on: load(server, "toggle_on.png"),
            bar_frame: load(server, "bar_frame.png"),
            bar_fill_hp: load(server, "bar_fill_hp.png"),
            bar_fill_mp: load(server, "bar_fill_mp.png"),
            bar_segments: load(server, "bar_segments.png"),
            cursor_pointer: load_path(server, "ui/cursor/pointer.png".to_string()),
        }
    }
}

pub fn sliced(image: Handle<Image>, border: f32) -> ImageNode {
    ImageNode {
        image,
        image_mode: NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(border),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        }),
        ..default()
    }
}

pub fn stretched(image: Handle<Image>) -> ImageNode {
    ImageNode {
        image,
        image_mode: NodeImageMode::Stretch,
        ..default()
    }
}

#[derive(Component, Clone)]
pub struct UiButtonSkin {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
    pub disabled: Handle<Image>,
}

#[derive(Component)]
pub struct ThemeSkip;

#[derive(Component)]
pub struct UiCursor;

pub struct UiThemePlugin;

impl Plugin for UiThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiAssets>()
            .add_systems(Startup, spawn_ui_cursor)
            .add_systems(Update, (theme_new_buttons, update_button_skin))
            .add_systems(Last, update_ui_cursor);
    }
}

fn spawn_ui_cursor(assets: Res<UiAssets>, mut commands: Commands) {
    commands.spawn((
        UiCursor,
        ImageNode {
            image: assets.cursor_pointer.clone(),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(32.0),
            height: Val::Px(32.0),
            ..default()
        },
        GlobalZIndex(2000),
        Visibility::Hidden,
    ));
}

fn update_ui_cursor(
    window: Query<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions>,
    mut pointer: Query<(&mut Node, &mut Visibility), With<UiCursor>>,
) {
    let Ok((mut node, mut visibility)) = pointer.single_mut() else {
        return;
    };
    let menus = cursor.grab_mode == CursorGrabMode::None;
    match window.single().ok().and_then(Window::cursor_position) {
        Some(position) if menus => {
            node.left = Val::Px(position.x - 4.0);
            node.top = Val::Px(position.y - 4.0);
            *visibility = Visibility::Inherited;
        }
        _ => *visibility = Visibility::Hidden,
    }
    cursor.visible = false;
}

fn theme_new_buttons(
    assets: Res<UiAssets>,
    mut commands: Commands,
    buttons: Query<
        Entity,
        (
            Added<Button>,
            Without<UiButtonSkin>,
            Without<ThemeSkip>,
        ),
    >,
) {
    for entity in &buttons {
        commands.entity(entity).insert((
            sliced(assets.btn_normal.clone(), 16.0),
            UiButtonSkin {
                normal: assets.btn_normal.clone(),
                hover: assets.btn_hover.clone(),
                pressed: assets.btn_pressed.clone(),
                disabled: assets.btn_disabled.clone(),
            },
            BackgroundColor(Color::NONE),
        ));
    }
}

fn update_button_skin(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut buttons: Query<(
        &UiGlobalTransform,
        &ComputedNode,
        &UiButtonSkin,
        &mut ImageNode,
        Has<Pressed>,
        Has<InteractionDisabled>,
    )>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let cursor = window
        .cursor_position()
        .map(|point| point * window.scale_factor());
    for (transform, node, skin, mut image, pressed, disabled) in &mut buttons {
        let hovered = cursor.is_some_and(|point| node.contains_point(*transform, point));
        let next = if disabled {
            skin.disabled.clone()
        } else if pressed {
            skin.pressed.clone()
        } else if hovered {
            skin.hover.clone()
        } else {
            skin.normal.clone()
        };
        if image.image != next {
            image.image = next;
        }
    }
}
