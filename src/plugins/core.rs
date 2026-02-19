use bevy::prelude::*;

/// Top-level game state (which screen are we on).
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    TitleScreen,
    MainMenu,
    Overworld,
    Battle,
    Shop,
    Inventory,
    Settings,
    PauseMenu,
}

/// Party data persisted across screens.
#[derive(Resource, Debug, Default)]
pub struct PartyResource {
    pub members: Vec<Entity>,
    pub gold: u32,
}

/// Inventory resource.
#[derive(Resource, Debug, Default)]
pub struct InventoryResource {
    pub items: Vec<InventoryItem>,
}

/// A single inventory item.
#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub name: String,
    pub quantity: u32,
    pub item_type: ItemType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemType {
    Consumable,
    Equipment,
    KeyItem,
}

/// Settings resource.
#[derive(Resource, Debug)]
pub struct GameSettings {
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub fullscreen: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            music_volume: 0.7,
            sfx_volume: 0.8,
            fullscreen: false,
        }
    }
}

/// Screen transition state for fade effects.
#[derive(Resource, Debug)]
pub struct ScreenTransition {
    pub active: bool,
    pub fading_out: bool,
    pub alpha: f32,
    pub target_state: Option<GameState>,
    pub speed: f32,
}

impl Default for ScreenTransition {
    fn default() -> Self {
        Self {
            active: false,
            fading_out: false,
            alpha: 0.0,
            target_state: None,
            speed: 2.0,
        }
    }
}

/// Plugin that registers all core resources and states.
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<PartyResource>()
            .init_resource::<InventoryResource>()
            .init_resource::<GameSettings>()
            .init_resource::<ScreenTransition>()
            .add_systems(Update, screen_transition_system);
    }
}

/// Handles fade-in / fade-out transitions between screens.
fn screen_transition_system(
    mut transition: ResMut<ScreenTransition>,
    mut next_state: ResMut<NextState<GameState>>,
    time: Res<Time>,
    mut fade_query: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    if !transition.active {
        return;
    }

    let dt = time.delta_secs() * transition.speed;

    if transition.fading_out {
        transition.alpha += dt;
        if transition.alpha >= 1.0 {
            transition.alpha = 1.0;
            // Switch state at peak darkness
            if let Some(target) = transition.target_state.take() {
                next_state.set(target);
            }
            transition.fading_out = false;
        }
    } else {
        transition.alpha -= dt;
        if transition.alpha <= 0.0 {
            transition.alpha = 0.0;
            transition.active = false;
        }
    }

    // Update the fade overlay color
    for mut bg in &mut fade_query {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, transition.alpha));
    }
}

/// Marker for the full-screen fade overlay.
#[derive(Component)]
pub struct FadeOverlay;

/// Start a fade transition to a new game state.
pub fn start_transition(transition: &mut ScreenTransition, target: GameState) {
    transition.active = true;
    transition.fading_out = true;
    transition.alpha = 0.0;
    transition.target_state = Some(target);
}
