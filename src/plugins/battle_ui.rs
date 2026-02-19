//! Battle UI: enemy/party displays, HP/PP bars, action menu, target selection,
//! damage numbers, turn order bar.
//!
//! This is the VISUAL layer only. It mirrors live `BattleUnit` ECS data into
//! lightweight UI-side caches and renders from those caches.

use bevy::prelude::*;

use super::core_plugin::GameState;
use crate::battle::types::{
    BattleAction, BattlePhase, BattleStateRes, BattleUnit, CommandMenu, CommandSelectState,
    UnitSide,
};
use crate::components::stats::Element;
use crate::plugins::core_plugin::{GameData, Party};

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
struct EnemyDisplay {
    #[allow(dead_code)]
    index: usize,
}

#[derive(Component)]
struct PartyDisplay {
    #[allow(dead_code)]
    index: usize,
}

#[derive(Component)]
struct HpBar {
    unit_index: usize,
    is_enemy: bool,
}

#[derive(Component)]
struct PpBar {
    unit_index: usize,
}

#[derive(Component)]
struct HpText {
    index: usize,
    is_enemy: bool,
}

#[derive(Component)]
struct PpText {
    index: usize,
}

#[derive(Component)]
struct ActionMenuItem {
    index: usize,
}

#[derive(Component)]
struct ActionMenuRoot;

#[derive(Component)]
struct BattleMessageText;

#[derive(Component)]
struct TurnOrderDisplay;

#[derive(Component)]
struct TurnOrderText;

#[derive(Component)]
struct EnemyArea;

#[derive(Component)]
struct PartyArea;

#[derive(Component)]
struct EnemyPanelRoot;

#[derive(Component)]
struct PartyPanelRoot;

#[derive(Component)]
struct DamageNumber {
    lifetime: Timer,
    velocity: Vec2,
}

#[derive(Component)]
struct EnemyTargetIndicator {
    index: usize,
}

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum BattleUiPhase {
    ActionSelect,
    TargetSelect,
    ItemSelect,
    ItemTargetSelect,
    DjinnSelect,
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

/// UI-facing snapshot derived from a `BattleUnit` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleDisplayUnit {
    pub id: u32,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub pp: i32,
    pub max_pp: i32,
    pub element: Element,
    pub alive: bool,
}

impl From<&BattleUnit> for BattleDisplayUnit {
    fn from(unit: &BattleUnit) -> Self {
        Self {
            id: unit.id,
            name: unit.name.clone(),
            hp: unit.hp,
            max_hp: unit.max_hp,
            pp: unit.pp,
            max_pp: unit.max_pp,
            element: unit.element,
            alive: unit.is_alive(),
        }
    }
}

/// Tracks the item selection sub-menu state.
#[derive(Resource, Debug, Default)]
struct ItemSelectUiState {
    cursor: usize,
    items: Vec<(String, String, bool)>, // (item_id, display_name, is_healing)
}

/// Tracks the djinn selection sub-menu state.
#[derive(Resource, Debug, Default)]
struct DjinnSelectUiState {
    cursor: usize,
    djinn: Vec<(String, String)>, // (djinn_id, display_name)
}

#[derive(Component)]
struct ItemSelectPanelRoot;

#[derive(Component)]
struct ItemSelectEntry {
    index: usize,
}

#[derive(Component)]
struct DjinnSelectPanelRoot;

#[derive(Component)]
struct DjinnSelectEntry {
    index: usize,
}

#[derive(Resource, Debug, Default)]
struct BattleEnemies {
    enemies: Vec<BattleDisplayUnit>,
}

#[derive(Resource, Debug, Default)]
struct BattleParty {
    members: Vec<BattleDisplayUnit>,
}

const ACTION_LABELS: &[&str] = &["Fight", "Djinn", "Item", "Defend", "Flee"];

// ── Plugin ────────────────────────────────────────────────────────────

pub struct BattleUiPlugin;

impl Plugin for BattleUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Battle), setup_battle_ui)
            .add_systems(
                Update,
                (
                    sync_battle_display,
                    rebuild_battle_unit_panels,
                    battle_action_input,
                    battle_target_input,
                    battle_item_select_input,
                    battle_item_target_input,
                    battle_djinn_select_input,
                    update_hp_bars,
                    update_turn_order_display,
                    update_damage_numbers,
                    update_battle_message,
                )
                    .chain()
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
    commands.insert_resource(BattleUiState::default());
    commands.insert_resource(BattleEnemies::default());
    commands.insert_resource(BattleParty::default());
    commands.insert_resource(ItemSelectUiState::default());
    commands.insert_resource(DjinnSelectUiState::default());

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
                    TurnOrderText,
                    Text::new("Turn: --"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
            });

            // ── Enemy area ───────────────────────────────────
            root.spawn((
                EnemyArea,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(35.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(40.0),
                    ..default()
                },
            ));

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
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(BRIGHT_GOLD),
                ));
            });

            // ── Party area ───────────────────────────────────
            root.spawn((
                PartyArea,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(25.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(30.0),
                    padding: UiRect::horizontal(Val::Px(20.0)),
                    ..default()
                },
            ));

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
                        BorderColor(if is_sel {
                            GOLD_TEXT
                        } else {
                            Color::srgba(0.3, 0.3, 0.3, 0.5)
                        }),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(label.to_string()),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT }),
                        ));
                    });
                }
            });
        });
}

// ── Systems ───────────────────────────────────────────────────────────

fn sync_battle_display(
    units: Query<&BattleUnit>,
    mut enemies: ResMut<BattleEnemies>,
    mut party: ResMut<BattleParty>,
) {
    let mut next_enemies: Vec<BattleDisplayUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Enemy)
        .map(BattleDisplayUnit::from)
        .collect();
    next_enemies.sort_by_key(|u| u.id);

    let mut next_party: Vec<BattleDisplayUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player)
        .map(BattleDisplayUnit::from)
        .collect();
    next_party.sort_by_key(|u| u.id);

    if enemies.enemies != next_enemies {
        enemies.enemies = next_enemies;
    }
    if party.members != next_party {
        party.members = next_party;
    }
}

fn rebuild_battle_unit_panels(
    mut commands: Commands,
    enemies: Res<BattleEnemies>,
    party: Res<BattleParty>,
    enemy_areas: Query<Entity, With<EnemyArea>>,
    party_areas: Query<Entity, With<PartyArea>>,
    enemy_panels: Query<Entity, With<EnemyPanelRoot>>,
    party_panels: Query<Entity, With<PartyPanelRoot>>,
) {
    if !enemies.is_changed() && !party.is_changed() {
        return;
    }

    for entity in &enemy_panels {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &party_panels {
        commands.entity(entity).despawn_recursive();
    }

    let Ok(enemy_area) = enemy_areas.get_single() else {
        return;
    };
    commands.entity(enemy_area).with_children(|area| {
        for (i, enemy) in enemies.enemies.iter().enumerate() {
            area.spawn((
                EnemyPanelRoot,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|col| {
                // Target indicator (hidden by default)
                col.spawn((
                    EnemyTargetIndicator { index: i },
                    Text::new("v"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::NONE),
                    Node {
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
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
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
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
                    HpBar {
                        unit_index: i,
                        is_enemy: true,
                    },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(HP_GREEN),
                ));

                col.spawn((
                    HpText {
                        index: i,
                        is_enemy: true,
                    },
                    Text::new(format!("{}/{}", enemy.hp, enemy.max_hp)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            });
        }
    });

    let Ok(party_area) = party_areas.get_single() else {
        return;
    };
    commands.entity(party_area).with_children(|area| {
        for (i, member) in party.members.iter().enumerate() {
            area.spawn((
                PartyPanelRoot,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    min_width: Val::Px(120.0),
                    ..default()
                },
            ))
            .with_children(|col| {
                col.spawn((
                    PartyDisplay { index: i },
                    Text::new(&member.name),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(GOLD_TEXT),
                ));

                // HP
                col.spawn((
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(10.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(HP_BAR_BG),
                ))
                .with_child((
                    HpBar {
                        unit_index: i,
                        is_enemy: false,
                    },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(HP_GREEN),
                ));

                col.spawn((
                    HpText {
                        index: i,
                        is_enemy: false,
                    },
                    Text::new(format!("HP {}/{}", member.hp, member.max_hp)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.9, 0.7)),
                ));

                // PP
                col.spawn((
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(6.0),
                        margin: UiRect::top(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(PP_BAR_BG),
                ))
                .with_child((
                    PpBar { unit_index: i },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(PP_BLUE),
                ));

                col.spawn((
                    PpText { index: i },
                    Text::new(format!("PP {}/{}", member.pp, member.max_pp)),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.7, 0.9)),
                ));
            });
        }
    });
}

fn push_pending_action(cmd_state: &mut CommandSelectState, action: BattleAction) -> bool {
    let idx = cmd_state.selecting_unit_index;
    if idx < cmd_state.pending_actions.len() {
        cmd_state.pending_actions[idx] = Some(action);
        cmd_state.selecting_unit_index += 1;
        cmd_state.menu = CommandMenu::ItemSelect;
        cmd_state.cursor_index = 0;
        cmd_state.selected_ability = None;
        cmd_state.selected_djinn = None;
        true
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_action_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ui_state: ResMut<BattleUiState>,
    enemies: Res<BattleEnemies>,
    battle_phase: Res<State<BattlePhase>>,
    mut cmd_state: ResMut<CommandSelectState>,
    items: Query<(&ActionMenuItem, &Children, Entity)>,
    mut bg_query: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut text_query: Query<&mut TextColor>,
    mut indicators: Query<(&EnemyTargetIndicator, &mut TextColor), Without<ActionMenuItem>>,
) {
    if ui_state.phase != BattleUiPhase::ActionSelect {
        return;
    }
    if *battle_phase.get() != BattlePhase::CommandSelect {
        return;
    }
    // Keep the core command-state menu in a neutral branch so this UI owns command entry.
    cmd_state.menu = CommandMenu::ItemSelect;
    cmd_state.cursor_index = 0;

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
            *border = BorderColor(if is_sel {
                GOLD_TEXT
            } else {
                Color::srgba(0.3, 0.3, 0.3, 0.5)
            });
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
                let alive_indices: Vec<usize> = enemies
                    .enemies
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, e)| e.alive.then_some(idx))
                    .collect();
                if alive_indices.is_empty() {
                    ui_state.message = "No targets available.".into();
                    ui_state.message_timer.reset();
                } else {
                    ui_state.phase = BattleUiPhase::TargetSelect;
                    ui_state.target_cursor = 0;
                }
            }
            1 => {
                // Djinn select - open djinn sub-menu
                ui_state.phase = BattleUiPhase::DjinnSelect;
            }
            2 => {
                // Item select - open item sub-menu
                ui_state.phase = BattleUiPhase::ItemSelect;
            }
            3 => {
                if push_pending_action(&mut cmd_state, BattleAction::Defend) {
                    ui_state.message = "Defend queued.".into();
                } else {
                    ui_state.message = "No acting unit available.".into();
                }
                ui_state.message_timer.reset();
            }
            4 => {
                if push_pending_action(&mut cmd_state, BattleAction::Flee) {
                    ui_state.message = "Flee queued.".into();
                } else {
                    ui_state.message = "No acting unit available.".into();
                }
                ui_state.message_timer.reset();
            }
            _ => {}
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        if push_pending_action(&mut cmd_state, BattleAction::Flee) {
            ui_state.message = "Flee queued.".into();
        } else {
            ui_state.message = "No acting unit available.".into();
        }
        ui_state.message_timer.reset();
    }
}

fn battle_target_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ui_state: ResMut<BattleUiState>,
    enemies: Res<BattleEnemies>,
    battle_phase: Res<State<BattlePhase>>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut indicators: Query<(&EnemyTargetIndicator, &mut TextColor)>,
) {
    if ui_state.phase != BattleUiPhase::TargetSelect {
        return;
    }
    if *battle_phase.get() != BattlePhase::CommandSelect {
        return;
    }
    // Keep command-state input neutral while selecting targets through this UI.
    cmd_state.menu = CommandMenu::ItemSelect;
    cmd_state.cursor_index = 0;

    ui_state.cooldown.tick(time.delta());

    let alive_indices: Vec<usize> = enemies
        .enemies
        .iter()
        .enumerate()
        .filter_map(|(idx, e)| e.alive.then_some(idx))
        .collect();
    if alive_indices.is_empty() {
        ui_state.phase = BattleUiPhase::Victory;
        return;
    }
    if ui_state.target_cursor >= alive_indices.len() {
        ui_state.target_cursor = 0;
    }

    if ui_state.cooldown.finished() {
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
            ui_state.target_cursor = if ui_state.target_cursor == 0 {
                alive_indices.len() - 1
            } else {
                ui_state.target_cursor - 1
            };
            ui_state.cooldown.reset();
        }
        if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
            ui_state.target_cursor = (ui_state.target_cursor + 1) % alive_indices.len();
            ui_state.cooldown.reset();
        }
    }
    let selected_enemy_index = alive_indices[ui_state.target_cursor];

    // Show target indicator
    for (ind, mut tc) in &mut indicators {
        tc.0 = if ind.index == selected_enemy_index {
            BRIGHT_GOLD
        } else {
            Color::NONE
        };
    }

    if keys.just_pressed(KeyCode::Escape) {
        ui_state.phase = BattleUiPhase::ActionSelect;
        cmd_state.selected_ability = None;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        if let Some(enemy) = enemies.enemies.get(selected_enemy_index) {
            if push_pending_action(
                &mut cmd_state,
                BattleAction::Attack {
                    target_id: enemy.id,
                },
            ) {
                ui_state.message = format!("Attack queued on {}.", enemy.name);
            } else {
                ui_state.message = "No acting unit available.".into();
            }
            ui_state.message_timer.reset();
        }
        ui_state.phase = BattleUiPhase::ActionSelect;
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_item_select_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<BattleUiState>,
    mut item_state: ResMut<ItemSelectUiState>,
    battle_phase: Res<State<BattlePhase>>,
    party: Res<Party>,
    game_data: Res<GameData>,
    existing_panels: Query<Entity, With<ItemSelectPanelRoot>>,
    mut entries: Query<(&ItemSelectEntry, &mut Text, &mut TextColor)>,
) {
    if ui_state.phase != BattleUiPhase::ItemSelect {
        // Despawn item panel if phase changed
        for entity in &existing_panels {
            commands.entity(entity).despawn_recursive();
        }
        return;
    }
    if *battle_phase.get() != BattlePhase::CommandSelect {
        return;
    }

    // Build item list on first frame of this phase
    if item_state.items.is_empty() && existing_panels.is_empty() {
        let mut items: Vec<(String, String, bool)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item_id in &party.inventory {
            if seen.contains(item_id) {
                continue;
            }
            seen.insert(item_id.clone());
            if let Some(def) = game_data.items.get(item_id) {
                let count = party.inventory.iter().filter(|id| *id == item_id).count();
                let is_healing = def.effect.hp_restore > 0
                    || def.effect.pp_restore > 0
                    || def.effect.revive
                    || !def.effect.removes_status.is_empty();
                items.push((
                    item_id.clone(),
                    format!("{} x{}", def.name, count),
                    is_healing,
                ));
            }
        }

        if items.is_empty() {
            ui_state.message = "No usable items.".into();
            ui_state.message_timer.reset();
            ui_state.phase = BattleUiPhase::ActionSelect;
            return;
        }

        item_state.items = items.clone();
        item_state.cursor = 0;

        // Spawn the item select overlay
        commands
            .spawn((
                ItemSelectPanelRoot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(25.0),
                    top: Val::Percent(20.0),
                    width: Val::Percent(50.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(MENU_BG),
                BorderColor(GOLD_TEXT),
                GlobalZIndex(20),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Select Item"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(BRIGHT_GOLD),
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                for (i, (_, display_name, _)) in items.iter().enumerate() {
                    let is_sel = i == 0;
                    let prefix = if is_sel { "> " } else { "  " };
                    panel.spawn((
                        ItemSelectEntry { index: i },
                        Text::new(format!("{prefix}{display_name}")),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT }),
                    ));
                }

                panel.spawn((
                    Text::new("[Up/Down] Select  [Enter] Use  [Esc] Back"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            });
    }

    // Navigate
    let total = item_state.items.len();
    if total > 0 {
        if keys.just_pressed(KeyCode::ArrowUp) {
            item_state.cursor = if item_state.cursor == 0 {
                total - 1
            } else {
                item_state.cursor - 1
            };
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            item_state.cursor = (item_state.cursor + 1) % total;
        }

        // Update visuals
        for (entry, mut text, mut color) in &mut entries {
            let is_sel = entry.index == item_state.cursor;
            let base = text
                .as_str()
                .trim_start_matches("> ")
                .trim_start_matches("  ");
            let prefix = if is_sel { "> " } else { "  " };
            **text = format!("{prefix}{base}");
            *color = TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT });
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        for entity in &existing_panels {
            commands.entity(entity).despawn_recursive();
        }
        item_state.items.clear();
        ui_state.phase = BattleUiPhase::ActionSelect;
    }

    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
        && item_state.items.get(item_state.cursor).is_some()
    {
        // Despawn item select panel
        for entity in &existing_panels {
            commands.entity(entity).despawn_recursive();
        }

        // Both healing and damage items go to target select
        ui_state.phase = BattleUiPhase::ItemTargetSelect;
        ui_state.target_cursor = 0;
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_item_target_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<BattleUiState>,
    mut item_state: ResMut<ItemSelectUiState>,
    enemies: Res<BattleEnemies>,
    party_display: Res<BattleParty>,
    battle_phase: Res<State<BattlePhase>>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut party: ResMut<Party>,
    mut indicators: Query<(&EnemyTargetIndicator, &mut TextColor)>,
) {
    if ui_state.phase != BattleUiPhase::ItemTargetSelect {
        return;
    }
    if *battle_phase.get() != BattlePhase::CommandSelect {
        return;
    }

    let Some((item_id, _, is_healing)) = item_state.items.get(item_state.cursor).cloned() else {
        ui_state.phase = BattleUiPhase::ActionSelect;
        item_state.items.clear();
        return;
    };

    if is_healing {
        // Target ally
        let alive_allies: Vec<usize> = party_display
            .members
            .iter()
            .enumerate()
            .filter_map(|(idx, m)| m.alive.then_some(idx))
            .collect();

        if alive_allies.is_empty() {
            ui_state.phase = BattleUiPhase::ActionSelect;
            item_state.items.clear();
            return;
        }

        if ui_state.target_cursor >= alive_allies.len() {
            ui_state.target_cursor = 0;
        }

        // Hide enemy indicators
        for (_, mut tc) in &mut indicators {
            tc.0 = Color::NONE;
        }

        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft) {
            ui_state.target_cursor = if ui_state.target_cursor == 0 {
                alive_allies.len() - 1
            } else {
                ui_state.target_cursor - 1
            };
        }
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowRight) {
            ui_state.target_cursor = (ui_state.target_cursor + 1) % alive_allies.len();
        }

        let selected_idx = alive_allies[ui_state.target_cursor];
        ui_state.message = format!(
            "Use {} on: {}",
            item_state
                .items
                .get(item_state.cursor)
                .map(|(_, n, _)| n.as_str())
                .unwrap_or("?"),
            party_display
                .members
                .get(selected_idx)
                .map(|m| m.name.as_str())
                .unwrap_or("?")
        );

        if keys.just_pressed(KeyCode::Escape) {
            ui_state.phase = BattleUiPhase::ItemSelect;
            ui_state.message = String::new();
        }

        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
            if let Some(ally) = party_display.members.get(selected_idx) {
                if push_pending_action(
                    &mut cmd_state,
                    BattleAction::Item {
                        item_id: item_id.clone(),
                        target_id: ally.id,
                    },
                ) {
                    // Consume the item from party inventory
                    if let Some(pos) = party.inventory.iter().position(|id| *id == item_id) {
                        party.inventory.remove(pos);
                    }
                    ui_state.message = format!("Item queued on {}.", ally.name);
                } else {
                    ui_state.message = "No acting unit available.".into();
                }
                ui_state.message_timer.reset();
            }
            item_state.items.clear();
            ui_state.phase = BattleUiPhase::ActionSelect;
        }
    } else {
        // Target enemy (damage item)
        let alive_enemies: Vec<usize> = enemies
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(idx, e)| e.alive.then_some(idx))
            .collect();

        if alive_enemies.is_empty() {
            ui_state.phase = BattleUiPhase::ActionSelect;
            item_state.items.clear();
            return;
        }

        if ui_state.target_cursor >= alive_enemies.len() {
            ui_state.target_cursor = 0;
        }

        if keys.just_pressed(KeyCode::ArrowLeft) {
            ui_state.target_cursor = if ui_state.target_cursor == 0 {
                alive_enemies.len() - 1
            } else {
                ui_state.target_cursor - 1
            };
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            ui_state.target_cursor = (ui_state.target_cursor + 1) % alive_enemies.len();
        }

        let selected_enemy_index = alive_enemies[ui_state.target_cursor];

        // Show target indicator
        for (ind, mut tc) in &mut indicators {
            tc.0 = if ind.index == selected_enemy_index {
                BRIGHT_GOLD
            } else {
                Color::NONE
            };
        }

        ui_state.message = format!(
            "Use {} on: {}",
            item_state
                .items
                .get(item_state.cursor)
                .map(|(_, n, _)| n.as_str())
                .unwrap_or("?"),
            enemies
                .enemies
                .get(selected_enemy_index)
                .map(|e| e.name.as_str())
                .unwrap_or("?")
        );

        if keys.just_pressed(KeyCode::Escape) {
            ui_state.phase = BattleUiPhase::ItemSelect;
            ui_state.message = String::new();
            // Hide indicators
            for (_, mut tc) in &mut indicators {
                tc.0 = Color::NONE;
            }
        }

        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
            if let Some(enemy) = enemies.enemies.get(selected_enemy_index) {
                if push_pending_action(
                    &mut cmd_state,
                    BattleAction::Item {
                        item_id: item_id.clone(),
                        target_id: enemy.id,
                    },
                ) {
                    // Consume the item
                    if let Some(pos) = party.inventory.iter().position(|id| *id == item_id) {
                        party.inventory.remove(pos);
                    }
                    ui_state.message = format!("Item queued on {}.", enemy.name);
                } else {
                    ui_state.message = "No acting unit available.".into();
                }
                ui_state.message_timer.reset();
            }
            item_state.items.clear();
            ui_state.phase = BattleUiPhase::ActionSelect;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_djinn_select_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<BattleUiState>,
    mut djinn_state: ResMut<DjinnSelectUiState>,
    battle_phase: Res<State<BattlePhase>>,
    cmd_state: Res<CommandSelectState>,
    units: Query<&BattleUnit>,
    game_data: Res<GameData>,
    djinn_battle: Res<crate::battle::types::DjinnBattleRes>,
    existing_panels: Query<Entity, With<DjinnSelectPanelRoot>>,
    enemies: Res<BattleEnemies>,
    mut entries: Query<(&DjinnSelectEntry, &mut Text, &mut TextColor)>,
    mut next_cmd_state: ResMut<CommandSelectState>,
) {
    if ui_state.phase != BattleUiPhase::DjinnSelect {
        for entity in &existing_panels {
            commands.entity(entity).despawn_recursive();
        }
        return;
    }
    if *battle_phase.get() != BattlePhase::CommandSelect {
        return;
    }

    // Build djinn list on first frame
    if djinn_state.djinn.is_empty() && existing_panels.is_empty() {
        // Get current selecting unit
        let player_units: Vec<&BattleUnit> = units
            .iter()
            .filter(|u| u.side == UnitSide::Player && u.is_alive())
            .collect();

        let unit = player_units.get(cmd_state.selecting_unit_index);
        let Some(unit) = unit else {
            ui_state.phase = BattleUiPhase::ActionSelect;
            return;
        };

        // Find set djinn for this unit
        let set_djinn: Vec<(String, String)> = djinn_battle
            .trackers
            .iter()
            .filter(|t| {
                t.owner_unit_id == unit.id && t.state == crate::battle::types::DjinnBattleState::Set
            })
            .map(|t| {
                let name = game_data
                    .djinn
                    .get(&t.djinn_id)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| t.djinn_id.clone());
                (t.djinn_id.clone(), name)
            })
            .collect();

        if set_djinn.is_empty() {
            ui_state.message = "No djinn available.".into();
            ui_state.message_timer.reset();
            ui_state.phase = BattleUiPhase::ActionSelect;
            return;
        }

        djinn_state.djinn = set_djinn.clone();
        djinn_state.cursor = 0;

        commands
            .spawn((
                DjinnSelectPanelRoot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(25.0),
                    top: Val::Percent(20.0),
                    width: Val::Percent(50.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(MENU_BG),
                BorderColor(GOLD_TEXT),
                GlobalZIndex(20),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Select Djinn to Unleash"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(BRIGHT_GOLD),
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                for (i, (_, name)) in set_djinn.iter().enumerate() {
                    let is_sel = i == 0;
                    let prefix = if is_sel { "> " } else { "  " };
                    panel.spawn((
                        DjinnSelectEntry { index: i },
                        Text::new(format!("{prefix}{name}")),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT }),
                    ));
                }

                panel.spawn((
                    Text::new("[Up/Down] Select  [Enter] Unleash  [Esc] Back"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            });
    }

    // Navigate
    let total = djinn_state.djinn.len();
    if total > 0 {
        if keys.just_pressed(KeyCode::ArrowUp) {
            djinn_state.cursor = if djinn_state.cursor == 0 {
                total - 1
            } else {
                djinn_state.cursor - 1
            };
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            djinn_state.cursor = (djinn_state.cursor + 1) % total;
        }

        for (entry, mut text, mut color) in &mut entries {
            let is_sel = entry.index == djinn_state.cursor;
            let base = text
                .as_str()
                .trim_start_matches("> ")
                .trim_start_matches("  ");
            let prefix = if is_sel { "> " } else { "  " };
            **text = format!("{prefix}{base}");
            *color = TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT });
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        for entity in &existing_panels {
            commands.entity(entity).despawn_recursive();
        }
        djinn_state.djinn.clear();
        ui_state.phase = BattleUiPhase::ActionSelect;
    }

    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
        && let Some((djinn_id, djinn_name)) = djinn_state.djinn.get(djinn_state.cursor).cloned()
    {
        for entity in &existing_panels {
            commands.entity(entity).despawn_recursive();
        }

        // Pick first alive enemy as target for unleash
        let alive_enemies: Vec<usize> = enemies
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(idx, e)| e.alive.then_some(idx))
            .collect();

        if let Some(&target_idx) = alive_enemies.first() {
            if let Some(enemy) = enemies.enemies.get(target_idx) {
                if push_pending_action(
                    &mut next_cmd_state,
                    BattleAction::DjinnUnleash {
                        djinn_id: djinn_id.clone(),
                        target_id: enemy.id,
                    },
                ) {
                    ui_state.message = format!("Djinn {} unleashed!", djinn_name);
                } else {
                    ui_state.message = "No acting unit available.".into();
                }
                ui_state.message_timer.reset();
            }
        } else {
            ui_state.message = "No targets available.".into();
            ui_state.message_timer.reset();
        }

        djinn_state.djinn.clear();
        ui_state.phase = BattleUiPhase::ActionSelect;
    }
}

fn update_hp_bars(
    enemies: Res<BattleEnemies>,
    party: Res<BattleParty>,
    mut hp_bars: Query<(&HpBar, &mut Node)>,
    mut hp_texts: Query<(&HpText, &mut Text)>,
    mut pp_bars: Query<(&PpBar, &mut Node)>,
    mut pp_texts: Query<(&PpText, &mut Text)>,
) {
    for (bar, mut node) in &mut hp_bars {
        let ratio = if bar.is_enemy {
            enemies
                .enemies
                .get(bar.unit_index)
                .map(|e| {
                    if e.max_hp > 0 {
                        e.hp as f32 / e.max_hp as f32
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0)
        } else {
            party
                .members
                .get(bar.unit_index)
                .map(|m| {
                    if m.max_hp > 0 {
                        m.hp as f32 / m.max_hp as f32
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0)
        };
        node.width = Val::Percent(ratio * 100.0);
    }

    for (text_ref, mut text) in &mut hp_texts {
        if text_ref.is_enemy {
            if let Some(enemy) = enemies.enemies.get(text_ref.index) {
                **text = format!("{}/{}", enemy.hp, enemy.max_hp);
            }
        } else if let Some(member) = party.members.get(text_ref.index) {
            **text = format!("HP {}/{}", member.hp, member.max_hp);
        }
    }

    for (bar, mut node) in &mut pp_bars {
        let ratio = party
            .members
            .get(bar.unit_index)
            .map(|m| {
                if m.max_pp > 0 {
                    m.pp as f32 / m.max_pp as f32
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
        node.width = Val::Percent(ratio * 100.0);
    }

    for (text_ref, mut text) in &mut pp_texts {
        if let Some(member) = party.members.get(text_ref.index) {
            **text = format!("PP {}/{}", member.pp, member.max_pp);
        }
    }
}

fn update_turn_order_display(
    battle_state: Res<BattleStateRes>,
    units: Query<&BattleUnit>,
    mut text_query: Query<&mut Text, With<TurnOrderText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let mut names: Vec<String> = Vec::with_capacity(battle_state.turn_order.len());
    for unit_id in &battle_state.turn_order {
        if let Some(unit) = units.iter().find(|u| u.id == *unit_id) {
            names.push(unit.name.clone());
        } else {
            names.push(format!("#{unit_id}"));
        }
    }

    if names.is_empty() {
        **text = "Turn: --".into();
    } else {
        **text = format!("Turn {}: {}", battle_state.turn_number, names.join(" > "));
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

fn cleanup_battle(
    mut commands: Commands,
    query: Query<Entity, With<BattleRoot>>,
    item_panels: Query<Entity, With<ItemSelectPanelRoot>>,
    djinn_panels: Query<Entity, With<DjinnSelectPanelRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    for entity in &item_panels {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &djinn_panels {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<BattleUiState>();
    commands.remove_resource::<BattleEnemies>();
    commands.remove_resource::<BattleParty>();
    commands.remove_resource::<ItemSelectUiState>();
    commands.remove_resource::<DjinnSelectUiState>();
}
