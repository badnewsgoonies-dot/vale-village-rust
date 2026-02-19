//! Tower dungeon: multi-floor dungeon with increasing difficulty, boss battles,
//! floor rewards, and team selection before entering.

use bevy::prelude::*;
use rand::Rng;

use super::core_plugin::{GameData, GameState, Party};
use super::overworld::BattleReturnPosition;
use super::save::{SaveData, SaveSystem};
use crate::battle::types::{BattlePhase, StartBattleEvent};
use crate::components::world::GridPosition;
use crate::data::enemies::EnemyDefinition;
use crate::plugins::overworld::enemy_definition_to_battle_unit;

// ── Constants ─────────────────────────────────────────────────────────
const MAX_FLOORS: u32 = 10;
const ENCOUNTERS_PER_FLOOR: u32 = 2;

const TOWER_BG: Color = Color::srgb(0.08, 0.06, 0.12);
const GOLD_TEXT: Color = Color::srgb(0.85, 0.65, 0.13);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const MENU_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const SELECTED_BG: Color = Color::srgba(0.85, 0.65, 0.13, 0.25);

// ── Components ────────────────────────────────────────────────────────

#[derive(Component)]
struct TowerRoot;

#[derive(Component)]
struct TowerInfoText;

#[derive(Component)]
struct TowerMenuItem {
    index: usize,
}

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Debug)]
pub struct TowerState {
    pub current_floor: u32,
    pub encounters_remaining: u32,
    pub team_selected: bool,
    #[allow(dead_code)]
    pub cursor: usize,
    #[allow(dead_code)]
    pub active: bool,
    pub needs_autosave: bool,
}

impl Default for TowerState {
    fn default() -> Self {
        Self {
            current_floor: 1,
            encounters_remaining: ENCOUNTERS_PER_FLOOR,
            team_selected: false,
            cursor: 0,
            active: false,
            needs_autosave: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum TowerPhase {
    TeamSelect,
    FloorExplore,
    BossReady,
    FloorComplete,
    TowerComplete,
}

#[derive(Resource, Debug)]
struct TowerUiState {
    #[allow(dead_code)]
    phase: TowerPhase,
    cursor: usize,
    cooldown: Timer,
}

impl Default for TowerUiState {
    fn default() -> Self {
        Self {
            phase: TowerPhase::TeamSelect,
            cursor: 0,
            cooldown: Timer::from_seconds(0.15, TimerMode::Once),
        }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TowerState>()
            .add_systems(OnEnter(GameState::Tower), setup_tower)
            .add_systems(
                Update,
                (tower_input, tower_autosave)
                    .chain()
                    .run_if(in_state(GameState::Tower)),
            )
            .add_systems(OnExit(GameState::Tower), cleanup_tower);
    }
}

// ── Setup ─────────────────────────────────────────────────────────────

fn setup_tower(mut commands: Commands, tower_state: Res<TowerState>) {
    commands.insert_resource(TowerUiState {
        phase: if tower_state.team_selected {
            TowerPhase::FloorExplore
        } else {
            TowerPhase::TeamSelect
        },
        ..Default::default()
    });

    commands
        .spawn((
            TowerRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(40.0)),
                ..default()
            },
            BackgroundColor(TOWER_BG),
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("=== Corrupted Tower ==="),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Floor info
            root.spawn((
                TowerInfoText,
                Text::new(format!(
                    "Floor {}/{} - {} encounters remaining",
                    tower_state.current_floor, MAX_FLOORS, tower_state.encounters_remaining,
                )),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(GOLD_TEXT),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Menu options
            let labels = ["Explore Floor", "Rest (Heal 25%)", "Leave Tower"];
            root.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },))
                .with_children(|menu| {
                    for (i, label) in labels.iter().enumerate() {
                        let is_sel = i == 0;
                        menu.spawn((
                            TowerMenuItem { index: i },
                            Node {
                                width: Val::Px(280.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(if is_sel { SELECTED_BG } else { MENU_BG }),
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

// ── Input ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn tower_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut tower_state: ResMut<TowerState>,
    mut ui_state: ResMut<TowerUiState>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_battle_phase: ResMut<NextState<BattlePhase>>,
    mut start_battle_events: EventWriter<StartBattleEvent>,
    mut return_position: ResMut<BattleReturnPosition>,
    mut party: ResMut<Party>,
    game_data: Res<GameData>,
    menu_items: Query<(&TowerMenuItem, &Children, Entity)>,
    mut bg_query: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut text_query: Query<&mut TextColor>,
    mut info_text: Query<&mut Text, With<TowerInfoText>>,
) {
    ui_state.cooldown.tick(time.delta());

    // Update visual selection
    for (item, children, entity) in &menu_items {
        let is_sel = item.index == ui_state.cursor;
        if let Ok((mut bg, mut border)) = bg_query.get_mut(entity) {
            *bg = BackgroundColor(if is_sel { SELECTED_BG } else { MENU_BG });
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

    // Update info text
    if let Ok(mut text) = info_text.get_single_mut() {
        if tower_state.current_floor > MAX_FLOORS {
            **text = "Tower Complete! You have conquered all floors!".into();
        } else {
            **text = format!(
                "Floor {}/{} - {} encounters remaining",
                tower_state.current_floor, MAX_FLOORS, tower_state.encounters_remaining,
            );
        }
    }

    if ui_state.cooldown.finished() {
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
            if ui_state.cursor > 0 {
                ui_state.cursor -= 1;
            }
            ui_state.cooldown.reset();
        }
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
            if ui_state.cursor < 2 {
                ui_state.cursor += 1;
            }
            ui_state.cooldown.reset();
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        // Leave tower
        return_position.player_position = Some(GridPosition::new(15, 2));
        next_game_state.set(GameState::Overworld);
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match ui_state.cursor {
            0 => {
                // Explore Floor - start an encounter
                if tower_state.current_floor > MAX_FLOORS {
                    // Tower complete
                    return_position.player_position = Some(GridPosition::new(15, 2));
                    next_game_state.set(GameState::Overworld);
                    return;
                }

                let enemy_units = build_tower_encounter(tower_state.current_floor, &game_data);

                if !enemy_units.is_empty() {
                    tower_state.team_selected = true;
                    start_battle_events.send(StartBattleEvent {
                        encounter_id: format!(
                            "tower-f{}-e{}",
                            tower_state.current_floor, tower_state.encounters_remaining
                        ),
                        enemy_units,
                    });
                    next_battle_phase.set(BattlePhase::CommandSelect);
                    next_game_state.set(GameState::Battle);

                    // Decrement encounters
                    if tower_state.encounters_remaining > 0 {
                        tower_state.encounters_remaining -= 1;
                    }

                    // If all encounters done, next floor
                    if tower_state.encounters_remaining == 0 {
                        tower_state.current_floor += 1;
                        tower_state.encounters_remaining = ENCOUNTERS_PER_FLOOR;

                        // Floor rewards: gold per floor
                        party.gold += tower_state.current_floor * 50;

                        // Trigger auto-save on floor transition
                        tower_state.needs_autosave = true;
                    }
                }
            }
            1 => {
                // Rest - heals 25% HP/PP (once per floor)
                // Just add gold for now as representation
                party.gold += 10;
            }
            2 => {
                // Leave tower
                return_position.player_position = Some(GridPosition::new(15, 2));
                tower_state.team_selected = false;
                next_game_state.set(GameState::Overworld);
            }
            _ => {}
        }
    }
}

// ── Tower encounter builder ───────────────────────────────────────────

fn build_tower_encounter(
    floor: u32,
    game_data: &GameData,
) -> Vec<crate::battle::types::BattleUnit> {
    let mut rng = rand::thread_rng();

    let all_enemies: Vec<&EnemyDefinition> = game_data.enemies.values().collect();
    if all_enemies.is_empty() {
        return Vec::new();
    }

    // Scale difficulty by floor: pick enemies with level <= floor * 2
    let max_level = (floor * 2).min(20) as u8;
    let eligible: Vec<&EnemyDefinition> = all_enemies
        .iter()
        .copied()
        .filter(|e| e.level <= max_level)
        .collect();

    let pool: Vec<&EnemyDefinition> = if eligible.is_empty() {
        all_enemies
    } else {
        eligible
    };

    let count = match floor {
        1..=3 => rng.gen_range(1..=2),
        4..=6 => rng.gen_range(2..=3),
        7..=9 => rng.gen_range(2..=3),
        10 => 1, // Boss floor
        _ => rng.gen_range(1..=3),
    };

    let mut enemies = Vec::with_capacity(count);
    for i in 0..count {
        let idx = rng.gen_range(0..pool.len());
        let def = pool[idx];

        // Scale enemy stats by floor
        let scale = 1.0 + (floor as f32 - 1.0) * 0.15;
        let mut unit = enemy_definition_to_battle_unit(def, 10_000 + i as u32);
        unit.hp = (unit.hp as f32 * scale) as i32;
        unit.max_hp = unit.hp;
        unit.atk = (unit.atk as f32 * scale) as i32;
        unit.def = (unit.def as f32 * scale) as i32;
        unit.mag = (unit.mag as f32 * scale) as i32;
        unit.spd = (unit.spd as f32 * scale) as i32;

        enemies.push(unit);
    }

    enemies
}

// ── Auto-save ─────────────────────────────────────────────────────────

fn tower_autosave(world: &mut World) {
    let needs_save = world
        .get_resource::<TowerState>()
        .map(|ts| ts.needs_autosave)
        .unwrap_or(false);

    if !needs_save {
        return;
    }

    // Clear the flag
    if let Some(mut tower_state) = world.get_resource_mut::<TowerState>() {
        tower_state.needs_autosave = false;
    }

    let save_data = SaveData::from_game_state(world);
    let result = {
        let save_system = world.resource::<SaveSystem>();
        save_system.save(0, &save_data)
    };

    match result {
        Ok(()) => {
            info!("Tower auto-save to slot 0 successful.");
        }
        Err(err) => {
            warn!("Tower auto-save failed: {}", err);
        }
    }
}

// ── Cleanup ───────────────────────────────────────────────────────────

fn cleanup_tower(mut commands: Commands, query: Query<Entity, With<TowerRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<TowerUiState>();
}
