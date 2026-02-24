//! UI screens: Main Menu, Pause Menu, Settings.
//!
//! Uses the `GameState` from `core_plugin`. The initial state is `Loading` which
//! transitions to `MainMenu` automatically (handled in core_plugin). This plugin
//! builds the MainMenu UI, Pause overlay, and Settings screen.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode, WindowResolution};

use super::audio::AudioSettings;
use super::core_plugin::{GameState, Party};
use super::save::{SaveData, SaveSystem};

// ── Color palette (Golden Sun aesthetic) ──────────────────────────────
const GOLD: Color = Color::srgb(0.85, 0.65, 0.13);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DARK_BG: Color = Color::srgb(0.05, 0.05, 0.12);
const MENU_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const SELECTED_BG: Color = Color::srgba(0.85, 0.65, 0.13, 0.25);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);

// ── Marker components ─────────────────────────────────────────────────

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct MainMenuItem {
    index: usize,
}

#[derive(Component)]
struct TitlePulseText;

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct PauseMenuItem {
    index: usize,
}

#[derive(Component)]
struct PauseMenuFeedbackText;

#[derive(Component)]
struct MainMenuFeedbackText;

#[derive(Component)]
struct SaveSlotMenuRoot;

#[derive(Component)]
struct SaveSlotItem {
    index: usize,
}

#[derive(Component)]
struct SettingsRoot;

#[derive(Component)]
struct SettingsItem {
    index: usize,
}

#[derive(Component)]
struct SettingsValueText {
    index: usize,
}

#[derive(Component)]
pub struct FadeOverlay;

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Debug)]
struct MenuCursor {
    selected: usize,
    count: usize,
    cooldown: Timer,
}

impl MenuCursor {
    fn new(count: usize) -> Self {
        Self {
            selected: 0,
            count,
            cooldown: Timer::from_seconds(0.15, TimerMode::Once),
        }
    }
}

#[derive(Resource, Debug)]
struct PauseMenuFeedback {
    message: Option<String>,
    timer: Timer,
}

impl Default for PauseMenuFeedback {
    fn default() -> Self {
        Self {
            message: None,
            timer: Timer::from_seconds(1.5, TimerMode::Once),
        }
    }
}

#[derive(Resource, Debug)]
struct MainMenuFeedback {
    message: Option<String>,
    timer: Timer,
}

impl Default for MainMenuFeedback {
    fn default() -> Self {
        Self {
            message: None,
            timer: Timer::from_seconds(1.5, TimerMode::Once),
        }
    }
}

/// Whether the save-slot sub-menu is in Save or Load mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveSlotMode {
    Save,
    Load,
}

/// Resource tracking the save-slot selection sub-menu state.
#[derive(Resource, Debug)]
struct SaveSlotMenu {
    visible: bool,
    mode: SaveSlotMode,
    cursor: usize,
    cooldown: Timer,
}

impl SaveSlotMenu {
    fn new(mode: SaveSlotMode) -> Self {
        Self {
            visible: true,
            mode,
            cursor: 0,
            cooldown: Timer::from_seconds(0.15, TimerMode::Once),
        }
    }
}

/// Available resolution presets for the display settings.
const RESOLUTIONS: &[(f32, f32)] = &[(960.0, 540.0), (1280.0, 720.0), (1920.0, 1080.0)];

/// Display settings resource tracking fullscreen and resolution state.
#[derive(Resource, Debug, Default)]
struct DisplaySettings {
    fullscreen: bool,
    resolution_index: usize,
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
            speed: 2.5,
        }
    }
}

/// Start a fade transition to a new game state.
pub fn start_transition(transition: &mut ScreenTransition, target: GameState) {
    if transition.active {
        return; // don't interrupt an ongoing transition
    }
    transition.active = true;
    transition.fading_out = true;
    transition.alpha = 0.0;
    transition.target_state = Some(target);
}

// ── Plugin ────────────────────────────────────────────────────────────

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenTransition>()
            .init_resource::<DisplaySettings>()
            // Persistent systems
            .add_systems(Startup, spawn_fade_overlay)
            .add_systems(Update, screen_transition_system)
            // Main Menu
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(Update, main_menu_input)
            .add_systems(
                Update,
                (title_pulse_animation, main_menu_feedback_update)
                    .run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(
                OnExit(GameState::MainMenu),
                (cleanup::<MainMenuRoot>, clear_main_menu_feedback),
            )
            // Pause Menu
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(Update, pause_menu_input.run_if(in_state(GameState::Paused)))
            .add_systems(
                Update,
                pause_menu_feedback_update.run_if(in_state(GameState::Paused)),
            )
            .add_systems(
                OnExit(GameState::Paused),
                (
                    cleanup::<PauseMenuRoot>,
                    cleanup::<SaveSlotMenuRoot>,
                    clear_pause_menu_feedback,
                    clear_save_slot_menu,
                ),
            )
            // Settings
            .add_systems(OnEnter(GameState::Settings), setup_settings)
            .add_systems(Update, settings_input.run_if(in_state(GameState::Settings)))
            .add_systems(OnExit(GameState::Settings), cleanup::<SettingsRoot>);
    }
}

// ── Shared utilities ──────────────────────────────────────────────────

fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn spawn_fade_overlay(mut commands: Commands) {
    commands.spawn((
        FadeOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ZIndex(999),
    ));
}

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

    for mut bg in &mut fade_query {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, transition.alpha));
    }
}

// ══════════════════════════════════════════════════════════════════════
// MAIN MENU  (entered from Loading → MainMenu via core_plugin)
// ══════════════════════════════════════════════════════════════════════

const MAIN_MENU_ITEMS: &[&str] = &["New Game", "Continue", "Settings", "Quit"];

fn setup_main_menu(mut commands: Commands) {
    commands.insert_resource(MenuCursor::new(MAIN_MENU_ITEMS.len()));
    commands.insert_resource(MainMenuFeedback::default());

    commands
        .spawn((
            MainMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(DARK_BG),
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("VALE VILLAGE"),
                TextFont {
                    font_size: 72.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Subtitle
            parent.spawn((
                Text::new("A Golden Sun-Inspired RPG"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(60.0)),
                    ..default()
                },
            ));

            // Menu items
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|menu| {
                    for (i, label) in MAIN_MENU_ITEMS.iter().enumerate() {
                        let is_sel = i == 0;
                        menu.spawn((
                            MainMenuItem { index: i },
                            Node {
                                width: Val::Px(280.0),
                                height: Val::Px(48.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::vertical(Val::Px(4.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(if is_sel { SELECTED_BG } else { Color::NONE }),
                            BorderColor(if is_sel { GOLD } else { Color::NONE }),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(label.to_string()),
                                TextFont {
                                    font_size: 26.0,
                                    ..default()
                                },
                                TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT }),
                            ));
                        });
                    }
                });

            // Pulsing prompt
            parent.spawn((
                TitlePulseText,
                Text::new("Arrow Keys: Navigate  |  Enter: Select"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Feedback text (e.g. "No save found")
            parent.spawn((
                MainMenuFeedbackText,
                Text::new(""),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
                Node {
                    min_height: Val::Px(22.0),
                    margin: UiRect::top(Val::Px(14.0)),
                    ..default()
                },
            ));
        });
}

fn title_pulse_animation(time: Res<Time>, mut query: Query<&mut TextColor, With<TitlePulseText>>) {
    let alpha = (time.elapsed_secs() * 2.0).sin() * 0.3 + 0.7;
    for mut color in &mut query {
        color.0 = Color::srgba(0.5, 0.5, 0.5, alpha * 0.7);
    }
}

fn main_menu_input(world: &mut World) {
    // Only run while in MainMenu state
    {
        let state = world.resource::<State<GameState>>();
        if *state.get() != GameState::MainMenu {
            return;
        }
    }

    if world.resource::<ScreenTransition>().active {
        return;
    }

    let delta = world.resource::<Time>().delta();
    let (up_pressed, down_pressed, confirm_pressed) = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        (
            keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW),
            keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS),
            keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
        )
    };

    {
        let mut cursor = world.resource_mut::<MenuCursor>();
        cursor.cooldown.tick(delta);

        if cursor.cooldown.finished() {
            if up_pressed {
                cursor.selected = if cursor.selected == 0 {
                    cursor.count - 1
                } else {
                    cursor.selected - 1
                };
                cursor.cooldown.reset();
            }
            if down_pressed {
                cursor.selected = (cursor.selected + 1) % cursor.count;
                cursor.cooldown.reset();
            }
        }
    }

    let selected = world.resource::<MenuCursor>().selected;

    // Update visuals
    {
        let mut items = world.query::<(&MainMenuItem, &Children, Entity)>();
        let item_data: Vec<(usize, Vec<Entity>, Entity)> = items
            .iter(world)
            .map(|(item, children, entity)| {
                (item.index, children.iter().copied().collect(), entity)
            })
            .collect();

        for (index, children, entity) in &item_data {
            let is_sel = *index == selected;

            // Update item background/border
            if let Some(mut bg) = world.entity_mut(*entity).get_mut::<BackgroundColor>() {
                *bg = BackgroundColor(if is_sel { SELECTED_BG } else { Color::NONE });
            }
            if let Some(mut border) = world.entity_mut(*entity).get_mut::<BorderColor>() {
                *border = BorderColor(if is_sel { GOLD } else { Color::NONE });
            }

            // Update child text color
            for child in children {
                if let Some(mut tc) = world.entity_mut(*child).get_mut::<TextColor>() {
                    tc.0 = if is_sel { BRIGHT_GOLD } else { DIM_TEXT };
                }
            }
        }
    }

    if !confirm_pressed {
        return;
    }

    match selected {
        0 => {
            // New Game: reset party to default for a fresh start
            world.insert_resource(Party::default());
            let mut transition = world.resource_mut::<ScreenTransition>();
            start_transition(&mut transition, GameState::Overworld);
        }
        1 => {
            // Continue: attempt to load save slot 1
            let result = {
                let save_system = world.resource::<SaveSystem>();
                save_system.load(1)
            };

            match result {
                Ok(save_data) => {
                    save_data.apply_to_game(world);
                    let mut transition = world.resource_mut::<ScreenTransition>();
                    start_transition(&mut transition, GameState::Overworld);
                }
                Err(_error) => {
                    warn!("No save found for slot 1: {}", _error);
                    set_main_menu_feedback(world, "No save found");
                }
            }
        }
        2 => {
            let mut transition = world.resource_mut::<ScreenTransition>();
            start_transition(&mut transition, GameState::Settings);
        }
        3 => {
            world.send_event(AppExit::Success);
        }
        _ => {}
    }
}

fn set_main_menu_feedback(world: &mut World, message: impl Into<String>) {
    let message = message.into();
    if let Some(mut feedback) = world.get_resource_mut::<MainMenuFeedback>() {
        feedback.message = Some(message);
        feedback.timer = Timer::from_seconds(1.5, TimerMode::Once);
    } else {
        world.insert_resource(MainMenuFeedback {
            message: Some(message),
            timer: Timer::from_seconds(1.5, TimerMode::Once),
        });
    }
}

fn main_menu_feedback_update(
    time: Res<Time>,
    feedback: Option<ResMut<MainMenuFeedback>>,
    mut text_query: Query<&mut Text, With<MainMenuFeedbackText>>,
) {
    let Ok(mut feedback_text) = text_query.get_single_mut() else {
        return;
    };

    let mut text_value = String::new();
    if let Some(mut feedback) = feedback {
        if feedback.message.is_some() {
            feedback.timer.tick(time.delta());
            if feedback.timer.finished() {
                feedback.message = None;
            }
        }

        if let Some(message) = feedback.message.as_ref() {
            text_value = message.clone();
        }
    }

    **feedback_text = text_value;
}

fn clear_main_menu_feedback(mut commands: Commands) {
    commands.remove_resource::<MainMenuFeedback>();
}

// ══════════════════════════════════════════════════════════════════════
// PAUSE MENU
// ══════════════════════════════════════════════════════════════════════

const PAUSE_ITEMS: &[&str] = &["Resume", "Save", "Load", "Settings", "Quit to Title"];

fn setup_pause_menu(mut commands: Commands) {
    commands.insert_resource(MenuCursor::new(PAUSE_ITEMS.len()));
    commands.insert_resource(PauseMenuFeedback::default());

    commands
        .spawn((
            PauseMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            GlobalZIndex(50),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        min_width: Val::Px(260.0),
                        ..default()
                    },
                    BackgroundColor(MENU_BG),
                    BorderColor(GOLD),
                ))
                .with_children(|menu| {
                    menu.spawn((
                        Text::new("PAUSED"),
                        TextFont {
                            font_size: 36.0,
                            ..default()
                        },
                        TextColor(BRIGHT_GOLD),
                        Node {
                            margin: UiRect::bottom(Val::Px(20.0)),
                            ..default()
                        },
                    ));

                    for (i, label) in PAUSE_ITEMS.iter().enumerate() {
                        menu.spawn((
                            PauseMenuItem { index: i },
                            Text::new(label.to_string()),
                            TextFont {
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(if i == 0 { BRIGHT_GOLD } else { DIM_TEXT }),
                            Node {
                                margin: UiRect::vertical(Val::Px(6.0)),
                                ..default()
                            },
                        ));
                    }

                    menu.spawn((
                        PauseMenuFeedbackText,
                        Text::new(""),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(BRIGHT_GOLD),
                        Node {
                            min_height: Val::Px(22.0),
                            margin: UiRect::top(Val::Px(14.0)),
                            ..default()
                        },
                    ));
                });
        });
}

fn pause_menu_input(world: &mut World) {
    if world.resource::<ScreenTransition>().active {
        return;
    }

    // If the save-slot sub-menu is open, delegate to its handler instead.
    let slot_menu_visible = world
        .get_resource::<SaveSlotMenu>()
        .is_some_and(|m| m.visible);
    if slot_menu_visible {
        save_slot_menu_input(world);
        return;
    }

    let delta = world.resource::<Time>().delta();
    let (up_pressed, down_pressed, escape_pressed, confirm_pressed) = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        (
            keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW),
            keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS),
            keys.just_pressed(KeyCode::Escape),
            keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
        )
    };

    {
        let mut cursor = world.resource_mut::<MenuCursor>();
        cursor.cooldown.tick(delta);

        if cursor.cooldown.finished() {
            if up_pressed {
                cursor.selected = if cursor.selected == 0 {
                    cursor.count - 1
                } else {
                    cursor.selected - 1
                };
                cursor.cooldown.reset();
            }
            if down_pressed {
                cursor.selected = (cursor.selected + 1) % cursor.count;
                cursor.cooldown.reset();
            }
        }
    }

    let selected = world.resource::<MenuCursor>().selected;

    {
        let mut items = world.query::<(&PauseMenuItem, &mut TextColor)>();
        for (item, mut color) in items.iter_mut(world) {
            color.0 = if item.index == selected {
                BRIGHT_GOLD
            } else {
                DIM_TEXT
            };
        }
    }

    if escape_pressed {
        let mut transition = world.resource_mut::<ScreenTransition>();
        start_transition(&mut transition, GameState::Overworld);
        return;
    }

    if !confirm_pressed {
        return;
    }

    match selected {
        0 => {
            let mut transition = world.resource_mut::<ScreenTransition>();
            start_transition(&mut transition, GameState::Overworld);
        }
        1 => {
            // Open save-slot sub-menu in Save mode
            open_save_slot_menu(world, SaveSlotMode::Save);
        }
        2 => {
            // Open save-slot sub-menu in Load mode
            open_save_slot_menu(world, SaveSlotMode::Load);
        }
        3 => {
            let mut transition = world.resource_mut::<ScreenTransition>();
            start_transition(&mut transition, GameState::Settings);
        }
        4 => {
            let mut transition = world.resource_mut::<ScreenTransition>();
            start_transition(&mut transition, GameState::MainMenu);
        }
        _ => {}
    }
}

/// Number of save slots shown in the sub-menu.
const SAVE_SLOT_COUNT: usize = 3;

/// Build a display label for a save slot.
/// Returns e.g. "Slot 1: Level 5, Gold 320" or "Slot 1: [Empty]".
fn save_slot_label(slot_index: usize, save_system: &SaveSystem) -> String {
    let display_number = slot_index + 1;
    match save_system.load(slot_index) {
        Ok(data) => {
            // Determine the highest level among party members for display.
            let max_level = data.party_data.iter().map(|m| m.level).max().unwrap_or(1);
            format!(
                "Slot {}: Level {}, Gold {}",
                display_number, max_level, data.gold
            )
        }
        Err(_) => format!("Slot {}: [Empty]", display_number),
    }
}

/// Spawn the save-slot selection sub-menu UI and insert the tracking resource.
fn open_save_slot_menu(world: &mut World, mode: SaveSlotMode) {
    // Build slot labels before spawning (needs SaveSystem borrow).
    let labels: Vec<String> = {
        let save_system = world.resource::<SaveSystem>();
        (0..SAVE_SLOT_COUNT)
            .map(|i| save_slot_label(i, save_system))
            .collect()
    };

    let title = match mode {
        SaveSlotMode::Save => "SAVE GAME",
        SaveSlotMode::Load => "LOAD GAME",
    };

    world.insert_resource(SaveSlotMenu::new(mode));

    // Spawn the UI overlay.
    let mut root_entity = world.spawn((
        SaveSlotMenuRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
        GlobalZIndex(60),
    ));

    root_entity.with_children(|parent| {
        parent
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    min_width: Val::Px(340.0),
                    ..default()
                },
                BackgroundColor(MENU_BG),
                BorderColor(GOLD),
            ))
            .with_children(|menu| {
                // Title
                menu.spawn((
                    Text::new(title.to_string()),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(BRIGHT_GOLD),
                    Node {
                        margin: UiRect::bottom(Val::Px(16.0)),
                        ..default()
                    },
                ));

                // Slot entries
                for (i, label) in labels.iter().enumerate() {
                    menu.spawn((
                        SaveSlotItem { index: i },
                        Text::new(label.clone()),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(if i == 0 { BRIGHT_GOLD } else { DIM_TEXT }),
                        Node {
                            margin: UiRect::vertical(Val::Px(6.0)),
                            ..default()
                        },
                    ));
                }

                // Hint
                menu.spawn((
                    Text::new("Enter: Select  |  Esc: Back"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                    Node {
                        margin: UiRect::top(Val::Px(14.0)),
                        ..default()
                    },
                ));
            });
    });
}

/// Despawn the save-slot sub-menu UI and remove the tracking resource.
fn close_save_slot_menu(world: &mut World) {
    // Despawn all SaveSlotMenuRoot entities and their children.
    let roots: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<SaveSlotMenuRoot>>();
        query.iter(world).collect()
    };
    for entity in roots {
        world.entity_mut(entity).despawn();
    }
    world.remove_resource::<SaveSlotMenu>();
}

/// Exclusive-system handler for the save-slot sub-menu input.
fn save_slot_menu_input(world: &mut World) {
    let delta = world.resource::<Time>().delta();
    let (up_pressed, down_pressed, escape_pressed, confirm_pressed) = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        (
            keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW),
            keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS),
            keys.just_pressed(KeyCode::Escape),
            keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
        )
    };

    // Navigate cursor
    {
        let mut slot_menu = world.resource_mut::<SaveSlotMenu>();
        slot_menu.cooldown.tick(delta);

        if slot_menu.cooldown.finished() {
            if up_pressed {
                slot_menu.cursor = if slot_menu.cursor == 0 {
                    SAVE_SLOT_COUNT - 1
                } else {
                    slot_menu.cursor - 1
                };
                slot_menu.cooldown.reset();
            }
            if down_pressed {
                slot_menu.cursor = (slot_menu.cursor + 1) % SAVE_SLOT_COUNT;
                slot_menu.cooldown.reset();
            }
        }
    }

    let cursor_pos = world.resource::<SaveSlotMenu>().cursor;

    // Highlight the selected slot
    {
        let mut items = world.query::<(&SaveSlotItem, &mut TextColor)>();
        for (item, mut color) in items.iter_mut(world) {
            color.0 = if item.index == cursor_pos {
                BRIGHT_GOLD
            } else {
                DIM_TEXT
            };
        }
    }

    // Escape: close sub-menu, return to pause menu
    if escape_pressed {
        close_save_slot_menu(world);
        return;
    }

    if !confirm_pressed {
        return;
    }

    // Perform the save or load action on the selected slot.
    let mode = world.resource::<SaveSlotMenu>().mode;
    let slot = cursor_pos; // 0-indexed

    match mode {
        SaveSlotMode::Save => {
            let save_data = SaveData::from_game_state(world);
            let result = {
                let save_system = world.resource::<SaveSystem>();
                save_system.save(slot, &save_data)
            };
            close_save_slot_menu(world);
            match result {
                Ok(()) => {
                    set_pause_menu_feedback(world, format!("Game saved to Slot {}!", slot + 1));
                }
                Err(error) => {
                    warn!("Failed to save slot {}: {}", slot, error);
                    set_pause_menu_feedback(world, format!("Save failed: {}", error));
                }
            }
        }
        SaveSlotMode::Load => {
            let result = {
                let save_system = world.resource::<SaveSystem>();
                save_system.load(slot)
            };
            close_save_slot_menu(world);
            match result {
                Ok(save_data) => {
                    save_data.apply_to_game(world);
                    set_pause_menu_feedback(world, format!("Loaded from Slot {}!", slot + 1));
                }
                Err(_error) => {
                    warn!("No save in slot {}: {}", slot, _error);
                    set_pause_menu_feedback(world, format!("No save in Slot {}", slot + 1));
                }
            }
        }
    }
}

fn set_pause_menu_feedback(world: &mut World, message: impl Into<String>) {
    let message = message.into();
    if let Some(mut feedback) = world.get_resource_mut::<PauseMenuFeedback>() {
        feedback.message = Some(message);
        feedback.timer = Timer::from_seconds(1.5, TimerMode::Once);
    } else {
        world.insert_resource(PauseMenuFeedback {
            message: Some(message),
            timer: Timer::from_seconds(1.5, TimerMode::Once),
        });
    }
}

fn pause_menu_feedback_update(
    time: Res<Time>,
    feedback: Option<ResMut<PauseMenuFeedback>>,
    mut text_query: Query<&mut Text, With<PauseMenuFeedbackText>>,
) {
    let Ok(mut feedback_text) = text_query.get_single_mut() else {
        return;
    };

    let mut text_value = String::new();
    if let Some(mut feedback) = feedback {
        if feedback.message.is_some() {
            feedback.timer.tick(time.delta());
            if feedback.timer.finished() {
                feedback.message = None;
            }
        }

        if let Some(message) = feedback.message.as_ref() {
            text_value = message.clone();
        }
    }

    **feedback_text = text_value;
}

fn clear_pause_menu_feedback(mut commands: Commands) {
    commands.remove_resource::<PauseMenuFeedback>();
}

fn clear_save_slot_menu(mut commands: Commands) {
    commands.remove_resource::<SaveSlotMenu>();
}

// ══════════════════════════════════════════════════════════════════════
// SETTINGS
// ══════════════════════════════════════════════════════════════════════

const SETTINGS_ITEMS: &[&str] = &[
    "Music Volume",
    "SFX Volume",
    "Fullscreen",
    "Resolution",
    "Back",
];

/// Local settings mirror (so we can read/write AudioSettings).
#[derive(Resource, Debug)]
struct SettingsUiState {
    music_volume: f32,
    sfx_volume: f32,
    fullscreen: bool,
    resolution_index: usize,
    /// Where we came from so we can go back.
    return_to: GameState,
}

fn setup_settings(
    mut commands: Commands,
    audio: Res<AudioSettings>,
    _state: Res<State<GameState>>,
    display: Res<DisplaySettings>,
) {
    commands.insert_resource(MenuCursor::new(SETTINGS_ITEMS.len()));

    // Remember where we came from — though we are already IN Settings,
    // we track via the previous state. Default to MainMenu.
    let return_to = GameState::MainMenu;

    let ui_state = SettingsUiState {
        music_volume: audio.music_volume,
        sfx_volume: audio.sfx_volume,
        fullscreen: display.fullscreen,
        resolution_index: display.resolution_index,
        return_to,
    };

    commands
        .spawn((
            SettingsRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(DARK_BG),
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SETTINGS"),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            for (i, label) in SETTINGS_ITEMS.iter().enumerate() {
                parent
                    .spawn((
                        SettingsItem { index: i },
                        Node {
                            width: Val::Px(400.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            margin: UiRect::vertical(Val::Px(4.0)),
                            padding: UiRect::horizontal(Val::Px(16.0)),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(label.to_string()),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(if i == 0 { BRIGHT_GOLD } else { DIM_TEXT }),
                        ));

                        let value_str = match i {
                            0 => format!("{:.0}%", ui_state.music_volume * 100.0),
                            1 => format!("{:.0}%", ui_state.sfx_volume * 100.0),
                            2 => (if ui_state.fullscreen { "ON" } else { "OFF" }).into(),
                            3 => {
                                let (w, h) = RESOLUTIONS[ui_state.resolution_index];
                                format!("{:.0}x{:.0}", w, h)
                            }
                            _ => String::new(),
                        };

                        row.spawn((
                            SettingsValueText { index: i },
                            Text::new(value_str),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(GOLD),
                        ));
                    });
            }

            parent.spawn((
                Text::new("Left/Right: Adjust  |  Enter/Esc: Back"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(30.0)),
                    ..default()
                },
            ));
        });

    commands.insert_resource(ui_state);
}

#[allow(clippy::too_many_arguments)]
fn settings_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cursor: ResMut<MenuCursor>,
    mut transition: ResMut<ScreenTransition>,
    mut audio: ResMut<AudioSettings>,
    mut ui_state: ResMut<SettingsUiState>,
    mut display: ResMut<DisplaySettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    items: Query<(&SettingsItem, &Children)>,
    mut text_query: Query<&mut TextColor>,
    mut value_texts: Query<(&SettingsValueText, &mut Text)>,
) {
    if transition.active {
        return;
    }

    cursor.cooldown.tick(time.delta());

    if cursor.cooldown.finished() {
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
            cursor.selected = if cursor.selected == 0 {
                cursor.count - 1
            } else {
                cursor.selected - 1
            };
            cursor.cooldown.reset();
        }
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
            cursor.selected = (cursor.selected + 1) % cursor.count;
            cursor.cooldown.reset();
        }
    }

    // Highlight selected row
    for (item, children) in &items {
        let is_sel = item.index == cursor.selected;
        for &child in children.iter() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                tc.0 = if is_sel { BRIGHT_GOLD } else { DIM_TEXT };
            }
        }
    }

    // Left/Right adjusts
    let left_pressed = keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA);
    let right_pressed = keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD);

    let adjust = if left_pressed {
        -0.1_f32
    } else if right_pressed {
        0.1
    } else {
        0.0
    };

    if adjust != 0.0 || left_pressed || right_pressed {
        match cursor.selected {
            0 => {
                ui_state.music_volume = (ui_state.music_volume + adjust).clamp(0.0, 1.0);
                audio.music_volume = ui_state.music_volume;
            }
            1 => {
                ui_state.sfx_volume = (ui_state.sfx_volume + adjust).clamp(0.0, 1.0);
                audio.sfx_volume = ui_state.sfx_volume;
            }
            2 => {
                // Fullscreen toggle
                if left_pressed || right_pressed {
                    ui_state.fullscreen = !ui_state.fullscreen;
                    display.fullscreen = ui_state.fullscreen;
                    if let Ok(mut window) = windows.get_single_mut() {
                        window.mode = if ui_state.fullscreen {
                            WindowMode::Fullscreen(MonitorSelection::Current)
                        } else {
                            WindowMode::Windowed
                        };
                    }
                }
            }
            3 => {
                // Resolution cycle
                if right_pressed {
                    ui_state.resolution_index = (ui_state.resolution_index + 1) % RESOLUTIONS.len();
                } else if left_pressed {
                    ui_state.resolution_index = if ui_state.resolution_index == 0 {
                        RESOLUTIONS.len() - 1
                    } else {
                        ui_state.resolution_index - 1
                    };
                }
                display.resolution_index = ui_state.resolution_index;
                let (w, h) = RESOLUTIONS[ui_state.resolution_index];
                if let Ok(mut window) = windows.get_single_mut() {
                    window.resolution = WindowResolution::new(w, h);
                }
            }
            _ => {}
        }
    }

    // Update displayed values
    for (vt, mut text) in &mut value_texts {
        let s = match vt.index {
            0 => format!("{:.0}%", ui_state.music_volume * 100.0),
            1 => format!("{:.0}%", ui_state.sfx_volume * 100.0),
            2 => (if ui_state.fullscreen { "ON" } else { "OFF" }).into(),
            3 => {
                let (w, h) = RESOLUTIONS[ui_state.resolution_index];
                format!("{:.0}x{:.0}", w, h)
            }
            _ => String::new(),
        };
        **text = s;
    }

    // Back
    if keys.just_pressed(KeyCode::Escape) {
        start_transition(&mut transition, ui_state.return_to);
    }
    if keys.just_pressed(KeyCode::Enter) && cursor.selected == SETTINGS_ITEMS.len() - 1 {
        start_transition(&mut transition, ui_state.return_to);
    }
}
