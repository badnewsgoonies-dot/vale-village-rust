use bevy::prelude::*;
use bevy::audio::{AudioSource, PlaybackSettings as BevyPlaybackSettings, Volume};

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
    pub boss_theme: Option<Handle<AudioSource>>,
    pub shop_theme: Option<Handle<AudioSource>>,
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

// ---------------------------------------------------------------------------
// Audio plugin
// ---------------------------------------------------------------------------

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioSettings::default());
        app.insert_resource(MusicTracks::default());
        app.insert_resource(SfxHandles::default());
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
