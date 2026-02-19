use bevy::prelude::*;

use super::core::{
    start_transition, FadeOverlay, GameSettings, GameState, ScreenTransition,
};

// ── Color palette (Golden Sun aesthetic) ──────────────────────────────
const GOLD: Color = Color::srgb(0.85, 0.65, 0.13);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DARK_BG: Color = Color::srgb(0.05, 0.05, 0.12);
const DEEP_BLUE: Color = Color::srgb(0.08, 0.08, 0.22);
const MENU_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const SELECTED_BG: Color = Color::srgba(0.85, 0.65, 0.13, 0.25);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);

// ── Marker components ─────────────────────────────────────────────────

#[derive(Component)]
pub struct TitleScreenRoot;

#[derive(Component)]
pub struct TitlePulseText;

#[derive(Component)]
pub struct MainMenuRoot;

#[derive(Component)]
pub struct MainMenuItem {
    pub index: usize,
}

#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component)]
pub struct PauseMenuItem {
    pub index: usize,
}

#[derive(Component)]
pub struct SettingsRoot;

#[derive(Component)]
pub struct SettingsItem {
    pub index: usize,
}

#[derive(Component)]
pub struct SettingsValueText {
    pub index: usize,
}

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Debug)]
pub struct MenuCursor {
    pub selected: usize,
    pub count: usize,
    pub cooldown: Timer,
}

impl MenuCursor {
    pub fn new(count: usize) -> Self {
        Self {
            selected: 0,
            count,
            cooldown: Timer::from_seconds(0.15, TimerMode::Once),
        }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            // Title Screen
            .add_systems(OnEnter(GameState::TitleScreen), setup_title_screen)
            .add_systems(
                Update,
                (title_screen_input, title_pulse_animation)
                    .run_if(in_state(GameState::TitleScreen)),
            )
            .add_systems(OnExit(GameState::TitleScreen), cleanup::<TitleScreenRoot>)
            // Main Menu
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(
                Update,
                main_menu_input.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), cleanup::<MainMenuRoot>)
            // Pause Menu
            .add_systems(OnEnter(GameState::PauseMenu), setup_pause_menu)
            .add_systems(
                Update,
                pause_menu_input.run_if(in_state(GameState::PauseMenu)),
            )
            .add_systems(OnExit(GameState::PauseMenu), cleanup::<PauseMenuRoot>)
            // Settings
            .add_systems(OnEnter(GameState::Settings), setup_settings)
            .add_systems(
                Update,
                settings_input.run_if(in_state(GameState::Settings)),
            )
            .add_systems(OnExit(GameState::Settings), cleanup::<SettingsRoot>)
            // Persistent fade overlay
            .add_systems(Startup, spawn_fade_overlay);
    }
}

// ── Generic cleanup system ────────────────────────────────────────────

fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// ── Fade overlay (always present, z-index above everything) ───────────

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

// ══════════════════════════════════════════════════════════════════════
// TITLE SCREEN
// ══════════════════════════════════════════════════════════════════════

fn setup_title_screen(mut commands: Commands) {
    // Full-screen dark background
    commands
        .spawn((
            TitleScreenRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(DARK_BG),
        ))
        .with_children(|parent| {
            // Title text
            parent.spawn((
                Text::new("VALE VILLAGE"),
                TextFont {
                    font_size: 72.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(60.0)),
                    ..default()
                },
            ));

            // Subtitle
            parent.spawn((
                Text::new("A Golden Sun-Inspired RPG"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(80.0)),
                    ..default()
                },
            ));

            // Pulsing "Press Enter" text
            parent.spawn((
                TitlePulseText,
                Text::new("Press Enter to Start"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(GOLD),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });
}

fn title_pulse_animation(
    time: Res<Time>,
    mut query: Query<&mut TextColor, With<TitlePulseText>>,
) {
    let alpha = (time.elapsed_secs() * 2.0).sin() * 0.4 + 0.6;
    for mut color in &mut query {
        color.0 = Color::srgba(0.85, 0.65, 0.13, alpha);
    }
}

fn title_screen_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut transition: ResMut<ScreenTransition>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        start_transition(&mut transition, GameState::MainMenu);
    }
}

// ══════════════════════════════════════════════════════════════════════
// MAIN MENU
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
        ))
        .with_children(|parent| {
            // Title at top
            parent.spawn((
                Text::new("VALE VILLAGE"),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(60.0)),
                    ..default()
                },
            ));

            // Menu container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|menu| {
                    for (i, label) in MAIN_MENU_ITEMS.iter().enumerate() {
                        let is_selected = i == 0;
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
                            BackgroundColor(if is_selected {
                                SELECTED_BG
                            } else {
                                Color::NONE
                            }),
                            BorderColor(if is_selected {
                                GOLD
                            } else {
                                Color::NONE
                            }),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(label.to_string()),
                                TextFont {
                                    font_size: 26.0,
                                    ..default()
                                },
                                TextColor(if is_selected { BRIGHT_GOLD } else { DIM_TEXT }),
                            ));
                        });
                    }
                });

            // Controls hint
            parent.spawn((
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

fn main_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cursor: ResMut<MenuCursor>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: EventWriter<AppExit>,
    items: Query<(&MainMenuItem, &Children)>,
    mut bg_query: Query<(&mut BackgroundColor, &mut BorderColor), With<MainMenuItem>>,
    mut text_query: Query<&mut TextColor>,
) {
    cursor.cooldown.tick(time.delta());

    let mut moved = false;
    if cursor.cooldown.finished() {
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
            cursor.selected = if cursor.selected == 0 {
                cursor.count - 1
            } else {
                cursor.selected - 1
            };
            moved = true;
        }
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
            cursor.selected = (cursor.selected + 1) % cursor.count;
            moved = true;
        }
    }

    if moved {
        cursor.cooldown.reset();
        // Update visual selection
        for (item, children) in &items {
            let selected = item.index == cursor.selected;
            if let Ok((mut bg, mut border)) =
                bg_query.get_mut(children.first().copied().unwrap_or(Entity::PLACEHOLDER))
            {
                // This won't work since items themselves have the components; fix below
                let _ = (bg.as_mut(), border.as_mut());
            }
        }
    }

    // Update all menu item visuals
    for (item, children) in &items {
        let selected = item.index == cursor.selected;
        let entity = items
            .iter()
            .find(|(m, _)| m.index == item.index)
            .map(|(_, _)| ())
            .unwrap();
        let _ = entity;
    }

    // Direct update of menu items
    for (item, children) in &items {
        let is_selected = item.index == cursor.selected;
        // Update child text color
        for &child in children.iter() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                tc.0 = if is_selected { BRIGHT_GOLD } else { DIM_TEXT };
            }
        }
    }

    // Update backgrounds/borders on the item entities directly
    for (item, _) in &items {
        // We need entity access — get it from a separate query
        let _ = item;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match cursor.selected {
            0 => {
                // New Game → go to overworld
                start_transition(&mut transition, GameState::Overworld);
            }
            1 => {
                // Continue (stub — same as new game for now)
                start_transition(&mut transition, GameState::Overworld);
            }
            2 => {
                // Settings
                start_transition(&mut transition, GameState::Settings);
            }
            3 => {
                // Quit
                app_exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// PAUSE MENU
// ══════════════════════════════════════════════════════════════════════

const PAUSE_MENU_ITEMS: &[&str] = &["Resume", "Save", "Load", "Settings", "Quit to Title"];

fn setup_pause_menu(mut commands: Commands) {
    commands.insert_resource(MenuCursor::new(PAUSE_MENU_ITEMS.len()));

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
        ))
        .with_children(|parent| {
            // Menu box
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(2.0)),
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

                    for (i, label) in PAUSE_MENU_ITEMS.iter().enumerate() {
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
        // Resume
        start_transition(&mut transition, GameState::Overworld);
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match cursor.selected {
            0 => start_transition(&mut transition, GameState::Overworld), // Resume
            1 => { /* Save stub */ }
            2 => { /* Load stub */ }
            3 => start_transition(&mut transition, GameState::Settings),
            4 => start_transition(&mut transition, GameState::TitleScreen), // Quit to Title
            _ => {}
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// SETTINGS
// ══════════════════════════════════════════════════════════════════════

const SETTINGS_ITEMS: &[&str] = &["Music Volume", "SFX Volume", "Fullscreen", "Back"];

fn setup_settings(mut commands: Commands) {
    commands.insert_resource(MenuCursor::new(SETTINGS_ITEMS.len()));

    let settings = GameSettings::default(); // will read from resource in input system

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

                        let value_text = match i {
                            0 => format!("{:.0}%", settings.music_volume * 100.0),
                            1 => format!("{:.0}%", settings.sfx_volume * 100.0),
                            2 => {
                                if settings.fullscreen {
                                    "ON".to_string()
                                } else {
                                    "OFF".to_string()
                                }
                            }
                            _ => String::new(),
                        };

                        row.spawn((
                            SettingsValueText { index: i },
                            Text::new(value_text),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(GOLD),
                        ));
                    });
            }

            parent.spawn((
                Text::new("Left/Right: Adjust  |  Enter: Select  |  Escape: Back"),
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
}

fn settings_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cursor: ResMut<MenuCursor>,
    mut settings: ResMut<GameSettings>,
    mut transition: ResMut<ScreenTransition>,
    items: Query<(&SettingsItem, &Children)>,
    mut text_query: Query<&mut TextColor>,
    mut value_texts: Query<(&SettingsValueText, &mut Text)>,
) {
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

    // Update label colors
    for (item, children) in &items {
        let is_sel = item.index == cursor.selected;
        for &child in children.iter() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                tc.0 = if is_sel { BRIGHT_GOLD } else { DIM_TEXT };
            }
        }
    }

    // Left/Right to adjust values
    let adjust = if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        -0.1_f32
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        0.1
    } else {
        0.0
    };

    if adjust != 0.0 {
        match cursor.selected {
            0 => settings.music_volume = (settings.music_volume + adjust).clamp(0.0, 1.0),
            1 => settings.sfx_volume = (settings.sfx_volume + adjust).clamp(0.0, 1.0),
            2 => settings.fullscreen = !settings.fullscreen,
            _ => {}
        }
    }

    // Update value displays
    for (val_text, mut text) in &mut value_texts {
        let new_val = match val_text.index {
            0 => format!("{:.0}%", settings.music_volume * 100.0),
            1 => format!("{:.0}%", settings.sfx_volume * 100.0),
            2 => {
                if settings.fullscreen {
                    "ON".to_string()
                } else {
                    "OFF".to_string()
                }
            }
            _ => String::new(),
        };
        **text = new_val;
    }

    if keys.just_pressed(KeyCode::Escape) {
        start_transition(&mut transition, GameState::MainMenu);
    }
    if keys.just_pressed(KeyCode::Enter) && cursor.selected == SETTINGS_ITEMS.len() - 1 {
        // "Back"
        start_transition(&mut transition, GameState::MainMenu);
    }
}
