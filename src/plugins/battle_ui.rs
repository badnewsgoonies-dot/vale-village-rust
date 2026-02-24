//! Battle UI: enemy/party displays, HP/PP bars, action menu, target selection,
//! damage numbers, turn order bar.
//!
//! This is the VISUAL layer only. It mirrors live `BattleUnit` ECS data into
//! lightweight UI-side caches and renders from those caches.

use bevy::prelude::*;

use super::core_plugin::GameState;
use crate::battle::types::{
    BattleAction, BattlePhase, BattleRewards, BattleStateRes, BattleUnit, CommandMenu,
    CommandSelectState, DamageEvent, EndBattleEvent, LevelUpEvent, UnitSide,
};
use crate::components::stats::Element;

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

#[derive(Component)]
struct VictoryScreenRoot;

#[derive(Component)]
struct DefeatScreenRoot;

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct BattleResultCache {
    rewards: Option<BattleRewards>,
    level_ups: Vec<LevelUpEvent>,
    displayed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
        app.init_resource::<BattleResultCache>()
            .add_systems(OnEnter(GameState::Battle), setup_battle_ui)
            .add_systems(
                Update,
                (
                    sync_battle_display,
                    rebuild_battle_unit_panels,
                    battle_action_input,
                    battle_target_input,
                    update_hp_bars,
                    update_turn_order_display,
                    update_damage_numbers,
                    update_battle_message,
                    cache_end_battle_event,
                    spawn_damage_numbers,
                    animate_damage_numbers,
                )
                    .chain()
                    .run_if(in_state(GameState::Battle)),
            )
            .add_systems(
                Update,
                battle_victory_ui_system.run_if(in_state(BattlePhase::Victory)),
            )
            .add_systems(
                Update,
                battle_defeat_ui_system.run_if(in_state(BattlePhase::Defeat)),
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
                ui_state.message = "Djinn not available.".into();
                ui_state.message_timer.reset();
            }
            2 => {
                ui_state.message = "Item not available.".into();
                ui_state.message_timer.reset();
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

// ── Cache end-battle event ────────────────────────────────────────────

fn cache_end_battle_event(
    mut events: EventReader<EndBattleEvent>,
    mut cache: ResMut<BattleResultCache>,
) {
    for event in events.read() {
        cache.rewards = event.rewards.clone();
        cache.level_ups = event.level_ups.clone();
        cache.displayed = false;
    }
}

// ── Victory UI ───────────────────────────────────────────────────────

const DEFEAT_RED: Color = Color::srgb(0.9, 0.15, 0.15);
const OVERLAY_BG: Color = Color::srgba(0.02, 0.02, 0.08, 0.92);

fn battle_victory_ui_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut cache: ResMut<BattleResultCache>,
    mut next_game_state: ResMut<NextState<GameState>>,
    existing: Query<Entity, With<VictoryScreenRoot>>,
) {
    // Spawn UI once
    if !cache.displayed {
        cache.displayed = true;

        let rewards = cache.rewards.as_ref();
        let gold = rewards.map_or(0, |r| r.total_gold);
        let xp = rewards.map_or(0, |r| r.xp_per_unit);
        let items: Vec<String> = rewards.map(|r| r.item_drops.clone()).unwrap_or_default();

        commands
            .spawn((
                VictoryScreenRoot,
                BattleRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(OVERLAY_BG),
                GlobalZIndex(20),
            ))
            .with_children(|root| {
                // "VICTORY!" header
                root.spawn((
                    Text::new("VICTORY!"),
                    TextFont {
                        font_size: 48.0,
                        ..default()
                    },
                    TextColor(BRIGHT_GOLD),
                ));

                // Gold earned
                root.spawn((
                    Text::new(format!("Gold earned: {gold}")),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(GOLD_TEXT),
                ));

                // XP per unit
                root.spawn((
                    Text::new(format!("XP per unit: {xp}")),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(GOLD_TEXT),
                ));

                // Level-up messages
                for lu in &cache.level_ups {
                    root.spawn((
                        Text::new(format!(
                            "{} leveled up! Lv.{} -> Lv.{}",
                            lu.unit_name, lu.old_level, lu.new_level
                        )),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(BRIGHT_GOLD),
                    ));

                    if !lu.new_abilities.is_empty() {
                        root.spawn((
                            Text::new(format!("  Learned: {}", lu.new_abilities.join(", "))),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(DIM_TEXT),
                        ));
                    }
                }

                // Item drops
                if !items.is_empty() {
                    root.spawn((
                        Text::new(format!("Items found: {}", items.join(", "))),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(GOLD_TEXT),
                    ));
                }

                // Spacer
                root.spawn(Node {
                    height: Val::Px(20.0),
                    ..default()
                });

                // Prompt
                root.spawn((
                    Text::new("Press Enter to continue"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
            });
    }

    // Wait for Enter to return to overworld
    if keys.just_pressed(KeyCode::Enter) && !existing.is_empty() {
        next_game_state.set(GameState::Overworld);
    }
}

// ── Defeat UI ────────────────────────────────────────────────────────

fn battle_defeat_ui_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut cache: ResMut<BattleResultCache>,
    mut next_game_state: ResMut<NextState<GameState>>,
    existing: Query<Entity, With<DefeatScreenRoot>>,
) {
    // Spawn UI once
    if !cache.displayed {
        cache.displayed = true;

        commands
            .spawn((
                DefeatScreenRoot,
                BattleRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(OVERLAY_BG),
                GlobalZIndex(20),
            ))
            .with_children(|root| {
                // "DEFEAT" header
                root.spawn((
                    Text::new("DEFEAT"),
                    TextFont {
                        font_size: 48.0,
                        ..default()
                    },
                    TextColor(DEFEAT_RED),
                ));

                root.spawn((
                    Text::new("Your party has been defeated..."),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));

                // Spacer
                root.spawn(Node {
                    height: Val::Px(20.0),
                    ..default()
                });

                root.spawn((
                    Text::new("Press Enter to return to title"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
            });
    }

    // Wait for Enter to return to main menu
    if keys.just_pressed(KeyCode::Enter) && !existing.is_empty() {
        next_game_state.set(GameState::MainMenu);
    }
}

// ── Damage numbers ───────────────────────────────────────────────────

fn spawn_damage_numbers(
    mut commands: Commands,
    mut damage_events: EventReader<DamageEvent>,
    units: Query<&BattleUnit>,
    enemy_res: Res<BattleEnemies>,
    party_res: Res<BattleParty>,
) {
    for event in damage_events.read() {
        // Determine position based on target
        let target_unit = units.iter().find(|u| u.id == event.target_id);
        let (x, y) = if let Some(unit) = target_unit {
            match unit.side {
                UnitSide::Enemy => {
                    let index = enemy_res
                        .enemies
                        .iter()
                        .position(|e| e.id == event.target_id)
                        .unwrap_or(0);
                    let spacing = 140.0;
                    let total_width = (enemy_res.enemies.len().saturating_sub(1)) as f32 * spacing;
                    let start_x = -total_width / 2.0;
                    (start_x + index as f32 * spacing, 80.0)
                }
                UnitSide::Player => {
                    let index = party_res
                        .members
                        .iter()
                        .position(|m| m.id == event.target_id)
                        .unwrap_or(0);
                    let spacing = 150.0;
                    let total_width = (party_res.members.len().saturating_sub(1)) as f32 * spacing;
                    let start_x = -total_width / 2.0;
                    (start_x + index as f32 * spacing, -100.0)
                }
            }
        } else {
            (0.0, 0.0)
        };

        let color = if event.was_blocked {
            Color::srgb(0.5, 0.5, 0.9)
        } else {
            Color::srgb(1.0, 0.3, 0.2)
        };

        let text = if event.damage >= 0 {
            format!("{}", event.damage)
        } else {
            // Negative means healing display
            format!("+{}", -event.damage)
        };

        commands.spawn((
            DamageNumber {
                lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                velocity: Vec2::new(0.0, 60.0),
            },
            Text2d::new(text),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(x, y, 100.0),
        ));
    }
}

fn animate_damage_numbers(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageNumber, &mut Transform, &mut TextColor)>,
) {
    for (entity, mut dmg, mut tf, mut color) in &mut query {
        dmg.lifetime.tick(time.delta());
        tf.translation.x += dmg.velocity.x * time.delta_secs();
        tf.translation.y += dmg.velocity.y * time.delta_secs();

        let alpha = 1.0 - dmg.lifetime.fraction();
        let base = color.0.to_srgba();
        color.0 = Color::srgba(base.red, base.green, base.blue, alpha);

        if dmg.lifetime.finished() {
            commands.entity(entity).despawn();
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
    commands.remove_resource::<BattleResultCache>();
}
