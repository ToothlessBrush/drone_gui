// Bevy 3D drone scene

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// Marker component for the drone entity
#[derive(Component)]
pub struct Drone;

/// Marker for the viewport camera
#[derive(Component)]
pub struct ViewportCamera;

/// Component to store current drone orientation
#[derive(Component)]
pub struct DroneOrientation {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

/// Resource to hold the render target image handle
#[derive(Resource)]
pub struct ViewportImage {
    pub handle: Handle<Image>,
}

impl Default for DroneOrientation {
    fn default() -> Self {
        Self {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

/// Setup the 3D drone scene
pub fn setup_drone_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Create render target image for the viewport
    // Using smaller resolution for better performance on Raspberry Pi
    let size = Extent3d {
        width: 320,
        height: 240,
        depth_or_array_layers: 1,
    };

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
    commands.insert_resource(ViewportImage {
        handle: image_handle.clone(),
    });
    // Colors
    let body_color = Color::srgb(0.5, 0.5, 0.5);
    let arm_color = Color::srgb(0.4, 0.4, 0.4);
    let motor_color = Color::srgb(0.2, 0.2, 0.2);
    let front_color = Color::srgb(0.0, 0.8, 0.0);

    // Create materials
    let body_material = materials.add(StandardMaterial {
        base_color: body_color,
        ..default()
    });
    let arm_material = materials.add(StandardMaterial {
        base_color: arm_color,
        ..default()
    });
    let motor_material = materials.add(StandardMaterial {
        base_color: motor_color,
        ..default()
    });
    let front_material = materials.add(StandardMaterial {
        base_color: front_color,
        ..default()
    });

    // Parent entity for the entire drone
    let drone_entity = commands
        .spawn((
            Name::new("Drone"),
            Drone,
            DroneOrientation::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Central body (cube)
    let body = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.15, 0.3))),
            MeshMaterial3d(body_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    commands.entity(drone_entity).add_child(body);

    // Front indicator (small green cube)
    let front_marker = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.08, 0.08, 0.02))),
            MeshMaterial3d(front_material),
            Transform::from_xyz(0.0, 0.0, 0.2),
        ))
        .id();
    commands.entity(drone_entity).add_child(front_marker);

    // Four arms at 45° angles
    let arm_length = 0.5;
    let arm_width = 0.08;
    let arm_height = 0.05;

    for i in 0..4 {
        let angle = (i as f32) * std::f32::consts::PI / 2.0 + std::f32::consts::PI / 4.0;
        let dir_x = angle.cos();
        let dir_z = angle.sin();

        // Arm (rotated cube)
        let arm_pos = Vec3::new(dir_x * arm_length / 2.0, 0.0, dir_z * arm_length / 2.0);
        let arm = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(arm_length, arm_height, arm_width))),
                MeshMaterial3d(arm_material.clone()),
                Transform::from_translation(arm_pos).with_rotation(Quat::from_rotation_y(angle)),
            ))
            .id();
        commands.entity(drone_entity).add_child(arm);

        // Motor (cylinder)
        let motor_pos = Vec3::new(dir_x * arm_length, arm_height, dir_z * arm_length);
        let motor = commands
            .spawn((
                Mesh3d(meshes.add(Cylinder::new(0.06, 0.08))),
                MeshMaterial3d(motor_material.clone()),
                Transform::from_translation(motor_pos)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::PI / 2.0)),
            ))
            .id();
        commands.entity(drone_entity).add_child(motor);

        // Propeller (flat cylinder)
        let prop_pos = Vec3::new(dir_x * arm_length, arm_height + 0.08, dir_z * arm_length);
        let propeller = commands
            .spawn((
                Mesh3d(meshes.add(Cylinder::new(0.12, 0.01))),
                MeshMaterial3d(motor_material.clone()),
                Transform::from_translation(prop_pos),
            ))
            .id();
        commands.entity(drone_entity).add_child(propeller);
    }

    // Viewport camera - renders to texture for egui display
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Image(image_handle.clone()),
            ..default()
        },
        Transform::from_xyz(0.0, 1.5, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ViewportCamera,
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::PI / 4.0,
            -std::f32::consts::PI / 4.0,
            0.0,
        )),
    ));

    commands.spawn((
        Mesh3d(meshes.add(create_grid_mesh(10.0, 20))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.5, 0.5, 0.3),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
}

// Generate grid mesh
fn create_grid_mesh(size: f32, divisions: usize) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let step = size / divisions as f32;

    for i in 0..=divisions {
        let offset = i as f32 * step - size / 2.0;

        // Lines along X
        positions.push([offset, 0.0, -size / 2.0]);
        positions.push([offset, 0.0, size / 2.0]);

        // Lines along Z
        positions.push([-size / 2.0, 0.0, offset]);
        positions.push([size / 2.0, 0.0, offset]);

        // Fade color based on distance from center
        let fade = 1.0 - (i as f32 / divisions as f32);
        let alpha = fade * 0.3;
        colors.push([0.5, 0.5, 0.5, alpha]);
        colors.push([0.5, 0.5, 0.5, alpha]);
        colors.push([0.5, 0.5, 0.5, alpha]);
        colors.push([0.5, 0.5, 0.5, alpha]);
    }

    Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

/// System to update drone orientation from telemetry data
pub fn update_drone_orientation(
    mut query: Query<(&mut Transform, &DroneOrientation), With<Drone>>,
) {
    for (mut transform, orientation) in query.iter_mut() {
        // Convert degrees to radians and apply rotation
        let rotation = Quat::from_euler(
            EulerRot::YXZ,
            orientation.yaw.to_radians(),
            orientation.pitch.to_radians(),
            orientation.roll.to_radians(),
        );
        transform.rotation = rotation;
    }
}
