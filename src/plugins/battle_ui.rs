//! Battle UI: enemy/party displays, HP/PP bars, action menu, target selection,
//! damage numbers, turn order bar.
//!
//! This is the VISUAL layer only. It reads from placeholder `BattleEnemies` and
//! `BattleParty` resources. When the real battle system (src/battle/) is
//! integrated, these resources should be replaced by queries on `BattleUnit`.

use bevy::prelude::*;

use crate::components::stats::Element;
use super::core_plugin::GameState;
use super::ui::{start_transition, ScreenTransition};

// ── Colors ────────────────────────────────────────────────────────────
const BATTLE_BG: Color = Color::srgb(0.06, 0.06, 0.14);
const HP_GREEN: Color = Color::srgb(0.2, 0.75, 0.2);
const HP_BAR_BG: Color = Color::srgb(0.2, 0.2, 0.2);
const PP_BLUE: Color = Color::srgb(0.2, 0.4, 0.85);
const PP_BAR_BG: Color = Color::srgb(0.15, 0.15, 0.2);
const GOLD_TEXT: Color = Color::srgb(0.85, 0.65, 0.13);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);
const MENU_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const SELECTED_BG: Color = Color::srgba(0.85, 0.65, 0.13, 0.25);

// ── Marker components ─────────────────────────────────────────────────

#[derive(Component)]
struct BattleRoot;

#[derive(Component)]
struct EnemyDisplay { index: usize }

#[derive(Component)]
struct PartyDisplay { index: usize }

#[derive(Component)]
struct HpBar { unit_index: usize, is_enemy: bool }

#[derive(Component)]
struct PpBar { unit_index: usize }

#[derive(Component)]
struct HpText { index: usize, is_enemy: bool }

#[derive(Component)]
struct PpText { index: usize }

#[derive(Component)]
struct ActionMenuItem { index: usize }

#[derive(Component)]
struct ActionMenuRoot;

#[derive(Component)]
struct BattleMessageText;

#[derive(Component)]
struct TurnOrderDisplay;

#[derive(Component)]
struct DamageNumber { lifetime: Timer, velocity: Vec2 }

#[derive(Component)]
struct EnemyTargetIndicator { index: usize }

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleUiPhase {
    ActionSelect,
    TargetSelect,
    Animating,
    Victory,
    Defeat,
}

#[derive(Resource, Debug)]
struct BattleUiState {
    action_cursor: usize,
    target_cursor: usize,
    phase: BattleUiPhase,
    message: String,
    message_timer: Timer,
    cooldown: Timer,
}

impl Default for BattleUiState {
    fn default() -> Self {
        Self {
            action_cursor: 0,
            target_cursor: 0,
            phase: BattleUiPhase::ActionSelect,
            message: String::new(),
            message_timer: Timer::from_seconds(2.0, TimerMode::Once),
            cooldown: Timer::from_seconds(0.12, TimerMode::Once),
        }
    }
}

/// Placeholder battle unit for the UI layer.
#[derive(Debug, Clone)]
pub struct BattleDisplayUnit {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub pp: i32,
    pub max_pp: i32,
    pub element: Element,
    pub alive: bool,
}

#[derive(Resource, Debug)]
struct BattleEnemies { enemies: Vec<BattleDisplayUnit> }

#[derive(Resource, Debug)]
struct BattleParty { members: Vec<BattleDisplayUnit> }

const ACTION_LABELS: &[&str] = &["Fight", "Djinn", "Item", "Defend", "Flee"];

// ── Plugin ────────────────────────────────────────────────────────────

pub struct BattleUiPlugin;

impl Plugin for BattleUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Battle), setup_battle_ui)
            .add_systems(
                Update,
                (
                    battle_action_input,
                    battle_target_input,
                    update_hp_bars,
                    update_damage_numbers,
                    update_battle_message,
                )
                    .run_if(in_state(GameState::Battle)),
            )
            .add_systems(OnExit(GameState::Battle), cleanup_battle);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn element_color(el: &Element) -> Color {
    match el {
        Element::Venus => Color::srgb(0.55, 0.4, 0.15),
        Element::Mars => Color::srgb(0.7, 0.2, 0.1),
        Element::Mercury => Color::srgb(0.15, 0.35, 0.7),
        Element::Jupiter => Color::srgb(0.5, 0.3, 0.7),
        Element::Neutral => Color::srgb(0.4, 0.4, 0.4),
    }
}

// ── Setup ─────────────────────────────────────────────────────────────

fn setup_battle_ui(mut commands: Commands) {
    let enemies = BattleEnemies {
        enemies: vec![
            BattleDisplayUnit { name: "Slime".into(), hp: 45, max_hp: 45, pp: 0, max_pp: 0, element: Element::Mercury, alive: true },
            BattleDisplayUnit { name: "Goblin".into(), hp: 60, max_hp: 60, pp: 10, max_pp: 10, element: Element::Venus, alive: true },
            BattleDisplayUnit { name: "Bat".into(), hp: 30, max_hp: 30, pp: 5, max_pp: 5, element: Element::Jupiter, alive: true },
        ],
    };
    let party = BattleParty {
        members: vec![
            BattleDisplayUnit { name: "Adept".into(), hp: 120, max_hp: 120, pp: 40, max_pp: 40, element: Element::Venus, alive: true },
            BattleDisplayUnit { name: "War Mage".into(), hp: 95, max_hp: 95, pp: 30, max_pp: 30, element: Element::Mars, alive: true },
            BattleDisplayUnit { name: "Mystic".into(), hp: 80, max_hp: 80, pp: 60, max_pp: 60, element: Element::Mercury, alive: true },
            BattleDisplayUnit { name: "Ranger".into(), hp: 100, max_hp: 100, pp: 35, max_pp: 35, element: Element::Jupiter, alive: true },
        ],
    };

    commands.insert_resource(BattleUiState::default());

    commands
        .spawn((
            BattleRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(BATTLE_BG),
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            // ── Turn order bar ───────────────────────────────
            root.spawn((
                TurnOrderDisplay,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Text::new("Turn: Adept > Ranger > Goblin > Mystic > War Mage > Bat > Slime"),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(DIM_TEXT),
                ));
            });

            // ── Enemy area ───────────────────────────────────
            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(35.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(40.0),
                ..default()
            })
            .with_children(|area| {
                for (i, enemy) in enemies.enemies.iter().enumerate() {
                    area.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|col| {
                        // Target indicator (hidden by default)
                        col.spawn((
                            EnemyTargetIndicator { index: i },
                            Text::new("v"),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(Color::NONE),
                            Node { margin: UiRect::bottom(Val::Px(2.0)), ..default() },
                        ));

                        // Sprite placeholder
                        col.spawn((
                            EnemyDisplay { index: i },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                margin: UiRect::bottom(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(element_color(&enemy.element)),
                        ));

                        // Name
                        col.spawn((
                            Text::new(&enemy.name),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                        ));

                        // HP bar
                        col.spawn((
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(8.0),
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(HP_BAR_BG),
                        ))
                        .with_child((
                            HpBar { unit_index: i, is_enemy: true },
                            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                            BackgroundColor(HP_GREEN),
                        ));

                        col.spawn((
                            HpText { index: i, is_enemy: true },
                            Text::new(format!("{}/{}", enemy.hp, enemy.max_hp)),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                    });
                }
            });

            // ── Message area ─────────────────────────────────
            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|area| {
                area.spawn((
                    BattleMessageText,
                    Text::new(""),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(BRIGHT_GOLD),
                ));
            });

            // ── Party area ───────────────────────────────────
            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(25.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(30.0),
                padding: UiRect::horizontal(Val::Px(20.0)),
                ..default()
            })
            .with_children(|area| {
                for (i, member) in party.members.iter().enumerate() {
                    area.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        min_width: Val::Px(120.0),
                        ..default()
                    })
                    .with_children(|col| {
                        col.spawn((
                            Text::new(&member.name),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(GOLD_TEXT),
                        ));

                        // HP
                        col.spawn((
                            Node { width: Val::Px(100.0), height: Val::Px(10.0), margin: UiRect::top(Val::Px(4.0)), ..default() },
                            BackgroundColor(HP_BAR_BG),
                        ))
                        .with_child((
                            HpBar { unit_index: i, is_enemy: false },
                            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                            BackgroundColor(HP_GREEN),
                        ));

                        col.spawn((
                            HpText { index: i, is_enemy: false },
                            Text::new(format!("HP {}/{}", member.hp, member.max_hp)),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.7, 0.9, 0.7)),
                        ));

                        // PP
                        col.spawn((
                            Node { width: Val::Px(100.0), height: Val::Px(6.0), margin: UiRect::top(Val::Px(2.0)), ..default() },
                            BackgroundColor(PP_BAR_BG),
                        ))
                        .with_child((
                            PpBar { unit_index: i },
                            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                            BackgroundColor(PP_BLUE),
                        ));

                        col.spawn((
                            PpText { index: i },
                            Text::new(format!("PP {}/{}", member.pp, member.max_pp)),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::srgb(0.6, 0.7, 0.9)),
                        ));
                    });
                }
            });

            // ── Action menu ──────────────────────────────────
            root.spawn((
                ActionMenuRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::top(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(MENU_BG),
                BorderColor(Color::srgb(0.4, 0.35, 0.2)),
            ))
            .with_children(|menu| {
                for (i, label) in ACTION_LABELS.iter().enumerate() {
                    let is_sel = i == 0;
                    menu.spawn((
                        ActionMenuItem { index: i },
                        Node {
                            width: Val::Px(100.0),
                            height: Val::Px(34.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(if is_sel { SELECTED_BG } else { Color::NONE }),
                        BorderColor(if is_sel { GOLD_TEXT } else { Color::srgba(0.3, 0.3, 0.3, 0.5) }),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(label.to_string()),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT }),
                        ));
                    });
                }
            });
        });

    commands.insert_resource(enemies);
    commands.insert_resource(party);
}

// ── Systems ───────────────────────────────────────────────────────────

fn battle_action_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ui_state: ResMut<BattleUiState>,
    mut transition: ResMut<ScreenTransition>,
    items: Query<(&ActionMenuItem, &Children, Entity)>,
    mut bg_query: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut text_query: Query<&mut TextColor>,
    mut indicators: Query<(&EnemyTargetIndicator, &mut TextColor), Without<ActionMenuItem>>,
) {
    if ui_state.phase != BattleUiPhase::ActionSelect {
        return;
    }

    ui_state.cooldown.tick(time.delta());

    if ui_state.cooldown.finished() {
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
            ui_state.action_cursor = if ui_state.action_cursor == 0 {
                ACTION_LABELS.len() - 1
            } else {
                ui_state.action_cursor - 1
            };
            ui_state.cooldown.reset();
        }
        if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
            ui_state.action_cursor = (ui_state.action_cursor + 1) % ACTION_LABELS.len();
            ui_state.cooldown.reset();
        }
    }

    // Update action menu visuals
    for (item, children, entity) in &items {
        let is_sel = item.index == ui_state.action_cursor;
        if let Ok((mut bg, mut border)) = bg_query.get_mut(entity) {
            *bg = BackgroundColor(if is_sel { SELECTED_BG } else { Color::NONE });
            *border = BorderColor(if is_sel { GOLD_TEXT } else { Color::srgba(0.3, 0.3, 0.3, 0.5) });
        }
        for &child in children.iter() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                tc.0 = if is_sel { BRIGHT_GOLD } else { DIM_TEXT };
            }
        }
    }

    // Hide target indicators during action select
    for (_, mut tc) in &mut indicators {
        tc.0 = Color::NONE;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match ui_state.action_cursor {
            0 => {
                ui_state.phase = BattleUiPhase::TargetSelect;
                ui_state.target_cursor = 0;
            }
            1 => {
                ui_state.message = "Djinn system coming soon!".into();
                ui_state.message_timer.reset();
            }
            2 => {
                ui_state.message = "No items yet!".into();
                ui_state.message_timer.reset();
            }
            3 => {
                ui_state.message = "Adept defends!".into();
                ui_state.message_timer.reset();
            }
            4 => start_transition(&mut transition, GameState::Overworld),
            _ => {}
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        start_transition(&mut transition, GameState::Overworld);
    }
}

fn battle_target_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ui_state: ResMut<BattleUiState>,
    enemies: Res<BattleEnemies>,
    mut indicators: Query<(&EnemyTargetIndicator, &mut TextColor)>,
) {
    if ui_state.phase != BattleUiPhase::TargetSelect {
        return;
    }

    ui_state.cooldown.tick(time.delta());

    let alive_count = enemies.enemies.iter().filter(|e| e.alive).count();
    if alive_count == 0 {
        ui_state.phase = BattleUiPhase::Victory;
        return;
    }

    if ui_state.cooldown.finished() {
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
            ui_state.target_cursor = if ui_state.target_cursor == 0 {
                alive_count - 1
            } else {
                ui_state.target_cursor - 1
            };
            ui_state.cooldown.reset();
        }
        if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
            ui_state.target_cursor = (ui_state.target_cursor + 1) % alive_count;
            ui_state.cooldown.reset();
        }
    }

    // Show target indicator
    for (ind, mut tc) in &mut indicators {
        tc.0 = if ind.index == ui_state.target_cursor {
            BRIGHT_GOLD
        } else {
            Color::NONE
        };
    }

    if keys.just_pressed(KeyCode::Escape) {
        ui_state.phase = BattleUiPhase::ActionSelect;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        if let Some(enemy) = enemies.enemies.get(ui_state.target_cursor) {
            ui_state.message = format!("Adept attacks {}!", enemy.name);
            ui_state.message_timer.reset();
        }
        ui_state.phase = BattleUiPhase::ActionSelect;
    }
}

fn update_hp_bars(
    enemies: Res<BattleEnemies>,
    party: Res<BattleParty>,
    mut hp_bars: Query<(&HpBar, &mut Node)>,
) {
    for (bar, mut node) in &mut hp_bars {
        let ratio = if bar.is_enemy {
            enemies
                .enemies
                .get(bar.unit_index)
                .map(|e| if e.max_hp > 0 { e.hp as f32 / e.max_hp as f32 } else { 0.0 })
                .unwrap_or(0.0)
        } else {
            party
                .members
                .get(bar.unit_index)
                .map(|m| if m.max_hp > 0 { m.hp as f32 / m.max_hp as f32 } else { 0.0 })
                .unwrap_or(0.0)
        };
        node.width = Val::Percent(ratio * 100.0);
    }
}

fn update_damage_numbers(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageNumber, &mut Transform, &mut TextColor)>,
) {
    for (entity, mut dmg, mut tf, mut color) in &mut query {
        dmg.lifetime.tick(time.delta());
        tf.translation.x += dmg.velocity.x * time.delta_secs();
        tf.translation.y += dmg.velocity.y * time.delta_secs();

        let alpha = 1.0 - dmg.lifetime.fraction();
        color.0 = Color::srgba(1.0, 0.3, 0.2, alpha);

        if dmg.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_battle_message(
    time: Res<Time>,
    mut ui_state: ResMut<BattleUiState>,
    mut text_query: Query<&mut Text, With<BattleMessageText>>,
) {
    ui_state.message_timer.tick(time.delta());

    if let Ok(mut text) = text_query.get_single_mut() {
        if ui_state.message_timer.finished() {
            **text = String::new();
        } else {
            **text = ui_state.message.clone();
        }
    }
}

fn cleanup_battle(mut commands: Commands, query: Query<Entity, With<BattleRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<BattleUiState>();
    commands.remove_resource::<BattleEnemies>();
    commands.remove_resource::<BattleParty>();
}
