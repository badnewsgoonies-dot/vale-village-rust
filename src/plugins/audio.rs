use bevy::audio::{AudioSource, PlaybackSettings as BevyPlaybackSettings, Volume};
use bevy::prelude::*;

use crate::plugins::core_plugin::GameState;

// ---------------------------------------------------------------------------
// Audio settings resource
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Resource)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub music_enabled: bool,
    pub sfx_enabled: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            music_volume: 0.7,
            sfx_volume: 0.9,
            music_enabled: true,
            sfx_enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Audio track handles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Resource, Default)]
pub struct MusicTracks {
    pub title_theme: Option<Handle<AudioSource>>,
    pub overworld_theme: Option<Handle<AudioSource>>,
    pub battle_theme: Option<Handle<AudioSource>>,
    #[allow(dead_code)]
    pub boss_theme: Option<Handle<AudioSource>>,
    #[allow(dead_code)]
    pub shop_theme: Option<Handle<AudioSource>>,
    #[allow(dead_code)]
    pub victory_fanfare: Option<Handle<AudioSource>>,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct SfxHandles {
    pub menu_select: Option<Handle<AudioSource>>,
    pub menu_cancel: Option<Handle<AudioSource>>,
    pub attack_hit: Option<Handle<AudioSource>>,
    pub magic_cast: Option<Handle<AudioSource>>,
    pub heal: Option<Handle<AudioSource>>,
    pub level_up: Option<Handle<AudioSource>>,
    pub item_pickup: Option<Handle<AudioSource>>,
    pub door_open: Option<Handle<AudioSource>>,
}

/// Marker for the currently playing background music entity.
#[derive(Component)]
pub struct BgmMarker;

/// Request to play an SFX by key name (for example "menu_select").
#[derive(Event, Debug, Clone)]
pub struct PlaySfxEvent(pub String);

// ---------------------------------------------------------------------------
// Audio plugin
// ---------------------------------------------------------------------------

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioSettings::default())
            .insert_resource(MusicTracks::default())
            .insert_resource(SfxHandles::default())
            .add_event::<PlaySfxEvent>()
            .add_systems(OnEnter(GameState::Loading), load_audio_assets)
            .add_systems(OnEnter(GameState::MainMenu), play_title_theme_on_enter)
            .add_systems(OnEnter(GameState::Overworld), play_overworld_theme_on_enter)
            .add_systems(OnEnter(GameState::Battle), play_battle_theme_on_enter)
            .add_systems(OnExit(GameState::Loading), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::MainMenu), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::Overworld), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::Battle), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::Shop), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::Inventory), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::Settings), stop_bgm_on_state_exit)
            .add_systems(OnExit(GameState::Paused), stop_bgm_on_state_exit)
            .add_systems(Update, play_sfx_events);
    }
}

fn play_title_theme_on_enter(
    mut commands: Commands,
    tracks: Res<MusicTracks>,
    settings: Res<AudioSettings>,
    bgm_query: Query<Entity, With<BgmMarker>>,
) {
    play_bgm(&mut commands, &bgm_query, &tracks.title_theme, &settings);
}

fn play_overworld_theme_on_enter(
    mut commands: Commands,
    tracks: Res<MusicTracks>,
    settings: Res<AudioSettings>,
    bgm_query: Query<Entity, With<BgmMarker>>,
) {
    play_bgm(
        &mut commands,
        &bgm_query,
        &tracks.overworld_theme,
        &settings,
    );
}

fn play_battle_theme_on_enter(
    mut commands: Commands,
    tracks: Res<MusicTracks>,
    settings: Res<AudioSettings>,
    bgm_query: Query<Entity, With<BgmMarker>>,
) {
    play_bgm(&mut commands, &bgm_query, &tracks.battle_theme, &settings);
}

fn stop_bgm_on_state_exit(mut commands: Commands, bgm_query: Query<Entity, With<BgmMarker>>) {
    stop_bgm(&mut commands, &bgm_query);
}

fn play_sfx_events(
    mut commands: Commands,
    mut events: EventReader<PlaySfxEvent>,
    sfx: Res<SfxHandles>,
    settings: Res<AudioSettings>,
) {
    for event in events.read() {
        let handle = match event.0.as_str() {
            "menu_select" => &sfx.menu_select,
            "menu_cancel" => &sfx.menu_cancel,
            "attack_hit" => &sfx.attack_hit,
            "magic_cast" => &sfx.magic_cast,
            "heal" => &sfx.heal,
            "level_up" => &sfx.level_up,
            "item_pickup" => &sfx.item_pickup,
            "door_open" => &sfx.door_open,
            key => {
                warn!("Unknown SFX key: {key}");
                continue;
            }
        };

        let _ = play_sfx(&mut commands, handle, &settings);
    }
}

// ---------------------------------------------------------------------------
// Helper functions for playing audio (to be called from other systems)
// ---------------------------------------------------------------------------

/// Play a one-shot SFX. Returns the spawned entity.
pub fn play_sfx(
    commands: &mut Commands,
    handle: &Option<Handle<AudioSource>>,
    settings: &AudioSettings,
) -> Option<Entity> {
    if !settings.sfx_enabled {
        return None;
    }
    let source = handle.as_ref()?;
    let volume = settings.master_volume * settings.sfx_volume;
    Some(
        commands
            .spawn((
                AudioPlayer(source.clone()),
                BevyPlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Despawn,
                    volume: Volume::new(volume),
                    ..default()
                },
            ))
            .id(),
    )
}

/// Start playing background music (loops). Despawns any existing BGM.
pub fn play_bgm(
    commands: &mut Commands,
    bgm_query: &Query<Entity, With<BgmMarker>>,
    handle: &Option<Handle<AudioSource>>,
    settings: &AudioSettings,
) {
    if !settings.music_enabled {
        return;
    }
    // Despawn existing BGM
    for entity in bgm_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(source) = handle {
        let volume = settings.master_volume * settings.music_volume;
        commands.spawn((
            AudioPlayer(source.clone()),
            BevyPlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::new(volume),
                ..default()
            },
            BgmMarker,
        ));
    }
}

/// Stop all background music.
pub fn stop_bgm(commands: &mut Commands, bgm_query: &Query<Entity, With<BgmMarker>>) {
    for entity in bgm_query.iter() {
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------------------------
// Asset loading
// ---------------------------------------------------------------------------

/// Loads all audio assets from `assets/audio/` into the `MusicTracks` and
/// `SfxHandles` resources. Runs once on `OnEnter(GameState::Loading)`.
fn load_audio_assets(
    asset_server: Res<AssetServer>,
    mut tracks: ResMut<MusicTracks>,
    mut sfx: ResMut<SfxHandles>,
) {
    // Music tracks
    tracks.title_theme = Some(asset_server.load("audio/music/title_theme.ogg"));
    tracks.overworld_theme = Some(asset_server.load("audio/music/overworld_theme.ogg"));
    tracks.battle_theme = Some(asset_server.load("audio/music/battle_theme.ogg"));
    tracks.boss_theme = Some(asset_server.load("audio/music/boss_theme.ogg"));
    tracks.shop_theme = Some(asset_server.load("audio/music/shop_theme.ogg"));
    tracks.victory_fanfare = Some(asset_server.load("audio/music/victory_fanfare.ogg"));

    // SFX handles
    sfx.menu_select = Some(asset_server.load("audio/sfx/menu_select.ogg"));
    sfx.menu_cancel = Some(asset_server.load("audio/sfx/menu_cancel.ogg"));
    sfx.attack_hit = Some(asset_server.load("audio/sfx/attack_hit.ogg"));
    sfx.magic_cast = Some(asset_server.load("audio/sfx/magic_cast.ogg"));
    sfx.heal = Some(asset_server.load("audio/sfx/heal.ogg"));
    sfx.level_up = Some(asset_server.load("audio/sfx/level_up.ogg"));
    sfx.item_pickup = Some(asset_server.load("audio/sfx/item_pickup.ogg"));
    sfx.door_open = Some(asset_server.load("audio/sfx/door_open.ogg"));

    info!("Audio assets loaded: 6 music tracks, 8 SFX handles");
}
