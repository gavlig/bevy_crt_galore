use bevy::{ prelude::*, window :: PresentMode };
use bevy_crt_galore::*;
use iyes_perf_ui :: prelude :: *;

fn main() {
    App::new()
        .add_plugins((
			DefaultPlugins.set(WindowPlugin {
				primary_window: Some(Window {
					present_mode: PresentMode::AutoNoVsync,
					..default()
				}),
				..default()
			}),

            CrtGalorePlugin,
            PerfUiPlugin,

			bevy::diagnostic::FrameTimeDiagnosticsPlugin,
	        bevy::diagnostic::SystemInformationDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
                .looking_at(Vec3::default(), Vec3::Y),
			camera: Camera {
				hdr : true,
				..default()
			},
            ..default()
        },
        // Add the setting to the camera.
        //
        // This component is also used to determine on which camera to run the
        // post processing effect.
        CrtGaloreSettings::STRONG,
    ));

    // cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::default())),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Rotates,
    ));
    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });
    // diagnostics
	commands.spawn((
	    PerfUiRoot { position : PerfUiPosition::BottomLeft, ..default() },
	    PerfUiEntryFPS::default(),
	    PerfUiEntryFPSWorst::default(),
	    PerfUiEntryFrameTime::default(),
	    PerfUiEntryFrameTimeWorst::default(),
	    PerfUiEntryFrameCount::default(),
	    PerfUiEntryCpuUsage::default(),
	    PerfUiEntryMemUsage::default(),
	));
}

#[derive(Component)]
struct Rotates;

/// Rotates any entity around the x and y axis
fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in &mut query {
        transform.rotate_x(0.55 * time.delta_seconds());
        transform.rotate_z(0.15 * time.delta_seconds());
    }
}