use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{Render, RenderApp};
use bevy::tasks::block_on;
use bevy::utils::Uuid;
use cosmic_text::Align;
use futures_signals::signal::Mutable;
use quirky::widget::SizeConstraint;
use quirky::{clone, QuirkyApp};
use quirky_widgets::layouts::anchored_container::{AnchorPoint, AnchoredContainerBuilder};
use quirky_widgets::theming::QuirkyTheme;
use quirky_widgets::widgets::button::ButtonBuilder;
use quirky_widgets::widgets::label::LabelBuilder;
use std::f32::consts::PI;
use std::sync::Arc;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(QuirkyPlugin {});
    app.add_systems(Update, cube_rotator_system);
    app.run();
}

pub struct QuirkyPlugin {}

impl Plugin for QuirkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<QuirkyAppCmp>::default());
        app.add_systems(Startup, quirky_startup);
        app.sub_app_mut(RenderApp)
            .add_systems(Render, quirky_render);
    }
}

#[derive(Component, ExtractComponent, Clone)]
pub struct QuirkyAppCmp {
    pub app: Arc<QuirkyApp>,
    pub image: Handle<Image>,
    pub text: Mutable<Arc<str>>,
}

fn quirky_render(gpu_images: ResMut<RenderAssets<Image>>, query: Query<&QuirkyAppCmp>) {
    for cmp in &query {
        if let Some(texture) = gpu_images.get(cmp.image.clone()) {
            if rand::random::<u16>() < 5000 {
                cmp.text
                    .set(format!("Hello, world of Bevy! 🐈 - {}", Uuid::new_v4()).into());
            }

            cmp.app.draw(&texture.texture_view).unwrap();
        }
    }
}

fn quirky_startup(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let quirky_device = device.wgpu_device();
    let quirky_device = unsafe { Arc::from_raw(quirky_device as *const wgpu::Device) };
    println!("hai");
    let content = Mutable::new("Hello, world of Bevy! 🐈".into());
    let quirky_app = Arc::new(QuirkyApp::new(
        quirky_device,
        queue.0.clone(),
        TextureFormat::Bgra8UnormSrgb,
        |resources, context, surface_format| {
            quirky_widgets::init(
                resources,
                context,
                surface_format,
                Some(QuirkyTheme::dark_default()),
            );
        },
        |r| {
            println!("wut");
            AnchoredContainerBuilder::new()
                .anchor_point(AnchorPoint::Center)
                .child(
                    ButtonBuilder::new()
                        .size_constraint(SizeConstraint::MaxSize(UVec2::new(500, 40)))
                        .content(
                            LabelBuilder::new()
                                .text_align(Align::Center)
                                .text_signal(clone!(content, move || content.signal_cloned()))
                                .build(),
                        )
                        .build(),
                )
                .build()
        },
    ));

    let quirky_app_cloned = quirky_app.clone();
    quirky_app_cloned.viewport_size.set(UVec2::new(1024, 1024));

    std::thread::spawn(move || {
        block_on(quirky_app_cloned.run(|| {}));
    });

    let size = Extent3d {
        width: 1024,
        height: 1024,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    image.resize(size);
    let image_handle = images.add(image);

    // The cube that will be rendered to the texture.
    commands.spawn(QuirkyAppCmp {
        app: quirky_app,
        image: image_handle.clone(),
        text: content,
    });

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    let cube_size = 8.0;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // Main pass cube, with material containing the rendered first pass texture.
    commands.spawn((
        PbrBundle {
            mesh: cube_handle,
            material: material_handle,
            transform: Transform::from_xyz(0.0, 0.0, 1.5)
                .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
            ..default()
        },
        MainPassCube,
    ));

    // The main pass camera.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct MainPassCube;
/// Rotates the outer cube (main pass)
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in &mut query {
        // transform.rotate_x(1.0 * time.delta_seconds());
        transform.rotate_y(0.7 * time.delta_seconds());
    }
}
