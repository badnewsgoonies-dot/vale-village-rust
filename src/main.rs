mod components;
mod data;
mod plugins;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

use plugins::core_plugin::CoreGamePlugin;
use plugins::save::SavePlugin;
use plugins::audio::GameAudioPlugin;

fn main() {
    App::new()
        // Default Bevy plugins with custom window settings
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Vale Village".into(),
                        resolution: WindowResolution::new(960.0, 540.0),
                        present_mode: PresentMode::AutoVsync,
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                // Pixel-art rendering: nearest-neighbor sampling
                .set(ImagePlugin::default_nearest()),
        )
        // Game plugins
        .add_plugins(CoreGamePlugin)
        .add_plugins(SavePlugin)
        .add_plugins(GameAudioPlugin)
        // Camera
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
