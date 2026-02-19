//! UI screens: Main Menu, Pause Menu, Settings.
//!
//! Uses the `GameState` from `core_plugin`. The initial state is `Loading` which
//! transitions to `MainMenu` automatically (handled in core_plugin). This plugin
//! builds the MainMenu UI, Pause overlay, and Settings screen.

use bevy::prelude::*;

use super::core_plugin::GameState;
use super::audio::AudioSettings;

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
            // Persistent systems
            .add_systems(Startup, spawn_fade_overlay)
            .add_systems(Update, screen_transition_system)
            // Main Menu
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(
                Update,
                (main_menu_input, title_pulse_animation)
                    .run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), cleanup::<MainMenuRoot>)
            // Pause Menu
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(
                Update,
                pause_menu_input.run_if(in_state(GameState::Paused)),
            )
            .add_systems(OnExit(GameState::Paused), cleanup::<PauseMenuRoot>)
            // Settings
            .add_systems(OnEnter(GameState::Settings), setup_settings)
            .add_systems(
                Update,
                settings_input.run_if(in_state(GameState::Settings)),
            )
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
        });
}

fn title_pulse_animation(
    time: Res<Time>,
    mut query: Query<&mut TextColor, With<TitlePulseText>>,
) {
    let alpha = (time.elapsed_secs() * 2.0).sin() * 0.3 + 0.7;
    for mut color in &mut query {
        color.0 = Color::srgba(0.5, 0.5, 0.5, alpha * 0.7);
    }
}

fn main_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cursor: ResMut<MenuCursor>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: EventWriter<AppExit>,
    items: Query<(&MainMenuItem, &Children, Entity)>,
    mut bg_query: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut text_query: Query<&mut TextColor>,
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

    // Update visuals
    for (item, children, entity) in &items {
        let is_sel = item.index == cursor.selected;

        // Update item background/border
        if let Ok((mut bg, mut border)) = bg_query.get_mut(entity) {
            *bg = BackgroundColor(if is_sel { SELECTED_BG } else { Color::NONE });
            *border = BorderColor(if is_sel { GOLD } else { Color::NONE });
        }

        // Update child text color
        for &child in children.iter() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                tc.0 = if is_sel { BRIGHT_GOLD } else { DIM_TEXT };
            }
        }
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match cursor.selected {
            0 => start_transition(&mut transition, GameState::Overworld), // New Game
            1 => start_transition(&mut transition, GameState::Overworld), // Continue (stub)
            2 => start_transition(&mut transition, GameState::Settings),
            3 => { app_exit.send(AppExit::Success); },
            _ => {}
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// PAUSE MENU
// ══════════════════════════════════════════════════════════════════════

const PAUSE_ITEMS: &[&str] = &["Resume", "Save", "Load", "Settings", "Quit to Title"];

fn setup_pause_menu(mut commands: Commands) {
    commands.insert_resource(MenuCursor::new(PAUSE_ITEMS.len()));

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
                });
        });
}

fn pause_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cursor: ResMut<MenuCursor>,
    mut transition: ResMut<ScreenTransition>,
    mut items: Query<(&PauseMenuItem, &mut TextColor)>,
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

    for (item, mut color) in &mut items {
        color.0 = if item.index == cursor.selected {
            BRIGHT_GOLD
        } else {
            DIM_TEXT
        };
    }

    if keys.just_pressed(KeyCode::Escape) {
        start_transition(&mut transition, GameState::Overworld);
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match cursor.selected {
            0 => start_transition(&mut transition, GameState::Overworld),
            1 => { /* Save — stub */ }
            2 => { /* Load — stub */ }
            3 => start_transition(&mut transition, GameState::Settings),
            4 => start_transition(&mut transition, GameState::MainMenu),
            _ => {}
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// SETTINGS
// ══════════════════════════════════════════════════════════════════════

const SETTINGS_ITEMS: &[&str] = &["Music Volume", "SFX Volume", "Fullscreen", "Back"];

/// Local settings mirror (so we can read/write AudioSettings).
#[derive(Resource, Debug)]
struct SettingsUiState {
    music_volume: f32,
    sfx_volume: f32,
    fullscreen: bool,
    /// Where we came from so we can go back.
    return_to: GameState,
}

fn setup_settings(
    mut commands: Commands,
    audio: Res<AudioSettings>,
    _state: Res<State<GameState>>,
) {
    commands.insert_resource(MenuCursor::new(SETTINGS_ITEMS.len()));

    // Remember where we came from — though we are already IN Settings,
    // we track via the previous state. Default to MainMenu.
    let return_to = GameState::MainMenu;

    let ui_state = SettingsUiState {
        music_volume: audio.music_volume,
        sfx_volume: audio.sfx_volume,
        fullscreen: false,
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

fn settings_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cursor: ResMut<MenuCursor>,
    mut transition: ResMut<ScreenTransition>,
    mut audio: ResMut<AudioSettings>,
    mut ui_state: ResMut<SettingsUiState>,
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
    let adjust = if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        -0.1_f32
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        0.1
    } else {
        0.0
    };

    if adjust != 0.0 {
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
                ui_state.fullscreen = !ui_state.fullscreen;
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
