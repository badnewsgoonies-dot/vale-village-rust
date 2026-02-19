mod battle;
mod components;
mod data;
mod plugins;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

use battle::BattlePlugin;
use plugins::audio::GameAudioPlugin;
use plugins::battle_ui::BattleUiPlugin;
use plugins::core_plugin::CoreGamePlugin;
use plugins::inventory::InventoryPlugin;
use plugins::overworld::OverworldPlugin;
use plugins::save::SavePlugin;
use plugins::shop::ShopPlugin;
use plugins::tower::TowerPlugin;
use plugins::ui::UiPlugin;

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
        // Core systems
        .add_plugins(CoreGamePlugin)
        .add_plugins(SavePlugin)
        .add_plugins(GameAudioPlugin)
        // UI & gameplay
        .add_plugins(UiPlugin)
        .add_plugins(OverworldPlugin)
        .add_plugins(ShopPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(BattleUiPlugin)
        .add_plugins(BattlePlugin)
        .add_plugins(TowerPlugin)
        // Default camera (used by UI screens; overworld spawns its own)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
