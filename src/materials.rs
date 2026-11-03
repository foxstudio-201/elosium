use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

use bevy::pbr::{ExtendedMaterial, MaterialExtension};

pub const FOG_DENSITY: f32 = 0.009;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WindSettings {
    pub timing: Vec4,
    pub player: Vec4,
    pub bend: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WindExtension {
    #[uniform(100)]
    pub settings: WindSettings,
}

impl MaterialExtension for WindExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/wind_vertex.wesl".into()
    }
}

pub type WindMaterial = ExtendedMaterial<StandardMaterial, WindExtension>;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct CloudParams {
    pub color: Vec4,
    pub camera: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CloudMaterial {
    #[uniform(0)]
    pub params: CloudParams,
}

impl Material for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cloud.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct CausticParams {
    pub timing: Vec4,
    pub surface: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CausticExtension {
    #[uniform(100)]
    pub params: CausticParams,
}

impl MaterialExtension for CausticExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/block.wesl".into()
    }
}

pub type BlockMaterial = ExtendedMaterial<StandardMaterial, CausticExtension>;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WaterParams {
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub camera_pos: Vec4,
    pub fog: Vec4,
    pub fog_color: Vec4,
    pub water_color: Vec4,
    pub sky_zenith: Vec4,
    pub timing: Vec4,
    pub reflection: Vec4,
    pub underwater: Vec4,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SkyParams {
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub camera_pos: Vec4,
    pub zenith: Vec4,
    pub horizon: Vec4,
    pub timing: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterExtension {
    #[uniform(100)]
    pub params: WaterParams,
    #[texture(101)]
    #[sampler(102)]
    pub reflection: Handle<Image>,
    #[texture(103)]
    #[sampler(104)]
    pub water: Handle<Image>,
    #[texture(105)]
    #[sampler(106)]
    pub normal: Handle<Image>,
}

impl MaterialExtension for WaterExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wesl".into()
    }
}

pub type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterExtension>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub params: SkyParams,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wesl".into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(Resource)]
pub struct GameMaterials {
    pub block: Handle<BlockMaterial>,
    pub particle: Handle<BlockMaterial>,
    pub wind: Handle<WindMaterial>,
    pub flora: Handle<WindMaterial>,
    pub leaves: Handle<WindMaterial>,
    pub water: Handle<WaterMaterial>,
    pub water_shadow: Handle<StandardMaterial>,
    pub sky: Handle<SkyMaterial>,
    pub cloud: [Handle<CloudMaterial>; 3],
}

pub fn create_materials(
    asset_server: &AssetServer,
    blocks: &mut Assets<BlockMaterial>,
    winds: &mut Assets<WindMaterial>,
    waters: &mut Assets<WaterMaterial>,
    standards: &mut Assets<StandardMaterial>,
    skies: &mut Assets<SkyMaterial>,
    cloud_materials: &mut Assets<CloudMaterial>,
    reflection: Handle<Image>,
) -> GameMaterials {
    let atlas: Handle<Image> = asset_server.load("textures/atlas.png");
    let atlas_flora: Handle<Image> = asset_server.load("textures/atlas_flora.png");
    let atlas_leaves: Handle<Image> = asset_server.load("textures/atlas_leaves.png");

    let block = blocks.add(BlockMaterial {
        base: StandardMaterial {
            base_color_texture: Some(atlas.clone()),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            reflectance: 0.0,
            alpha_mode: AlphaMode::Opaque,
            ..default()
        },
        extension: CausticExtension {
            params: CausticParams {
                timing: Vec4::ZERO,
                surface: Vec4::ZERO,
            },
        },
    });
    let particle = blocks.add(BlockMaterial {
        base: StandardMaterial {
            base_color_texture: Some(atlas.clone()),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            reflectance: 0.0,
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        },
        extension: CausticExtension {
            params: CausticParams {
                timing: Vec4::ZERO,
                surface: Vec4::ZERO,
            },
        },
    });

    let wind_base = |texture: Handle<Image>| StandardMaterial {
        base_color_texture: Some(texture),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    };
    let wind_extension = || WindExtension {
        settings: WindSettings {
            timing: Vec4::new(0.0, 0.06, 0.0, 0.0),
            player: Vec4::ZERO,
            bend: Vec4::ZERO,
        },
    };

    let wind = winds.add(WindMaterial {
        base: wind_base(atlas.clone()),
        extension: wind_extension(),
    });

    let flora = winds.add(WindMaterial {
        base: wind_base(atlas_flora),
        extension: wind_extension(),
    });

    let leaves = winds.add(WindMaterial {
        base: wind_base(atlas_leaves),
        extension: wind_extension(),
    });

    let water_frames: Handle<Image> = asset_server.load("textures/water_frames.png");
    let water_normal: Handle<Image> = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            });
        })
        .load("textures/water_normal.png");
    let water = waters.add(WaterMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.12,
            metallic: 0.0,
            reflectance: 0.5,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        },
        extension: WaterExtension {
            params: WaterParams {
                sun_dir: Vec4::new(0.45, 0.84, 0.34, 0.3),
                sun_color: Vec4::new(1.0, 0.95, 0.86, 0.0),
                camera_pos: Vec4::ZERO,
                fog: Vec4::new(FOG_DENSITY, 0.0, 0.0, 0.0),
                fog_color: Vec4::new(0.52, 0.68, 0.92, 0.0),
                water_color: Vec4::new(0.035, 0.315, 0.335, 0.0),
                sky_zenith: Vec4::new(0.12, 0.28, 0.62, 0.0),
                timing: Vec4::new(0.0, crate::world::WATER_SURFACE_HEIGHT, 0.0, 0.0),
                reflection: Vec4::ZERO,
                underwater: Vec4::ZERO,
            },
            reflection,
            water: water_frames,
            normal: water_normal,
        },
    });

    let water_shadow_texture: Handle<Image> = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            });
        })
        .load("textures/water_shadow.png");
    let water_shadow = standards.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(water_shadow_texture),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let cloud_alphas = [0.85, 0.62, 0.45];
    let cloud = cloud_alphas.map(|alpha| {
        cloud_materials.add(CloudMaterial {
            params: CloudParams {
                color: Vec4::new(1.0, 1.0, 1.02, alpha),
                camera: Vec4::ZERO,
            },
        })
    });

    let sky = skies.add(SkyMaterial {
        params: SkyParams {
            sun_dir: Vec4::new(0.45, 0.84, 0.34, 0.0),
            sun_color: Vec4::new(1.0, 0.92, 0.74, 0.0),
            camera_pos: Vec4::ZERO,
            zenith: Vec4::new(0.12, 0.28, 0.62, 0.0),
            horizon: Vec4::new(0.55, 0.72, 0.95, 0.0),
            timing: Vec4::ZERO,
        },
    });

    GameMaterials {
        block,
        particle,
        wind,
        flora,
        leaves,
        water,
        water_shadow,
        sky,
        cloud,
    }
}
