//! Bevy ECS systems for the battle flow.
//!
//! Systems drive the battle loop: CommandSelect -> AiSelect -> Resolution -> Victory/Defeat.
//! Pure logic lives in sibling modules; systems bridge ECS queries to pure functions.

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::battle::{ai, damage, djinn, rewards, status, turn_order, types::*};
use crate::components::battle::PartyCombatant;
use crate::data::items::ItemCategory;
use crate::plugins::core_plugin::{GameData, GameState, Party, story};

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Seeded RNG for deterministic battles.
#[derive(Resource)]
pub struct BattleRng(pub StdRng);

impl Default for BattleRng {
    fn default() -> Self {
        Self(StdRng::from_entropy())
    }
}

// ---------------------------------------------------------------------------
// Spawn party BattleUnit entities when entering battle
// ---------------------------------------------------------------------------

/// Creates BattleUnit entities for each active party member, applying
/// equipment stat bonuses from Party.equipment -> GameData.equipment.
pub fn spawn_party_battle_units(
    mut commands: Commands,
    party: Res<Party>,
    game_data: Res<GameData>,
    mut djinn_state: ResMut<DjinnBattleRes>,
) {
    // Initialize djinn trackers from party assignments
    djinn_state.trackers.clear();
    for djinn_id in party.djinn_assignments.keys() {
        if let Some(djinn_def) = game_data.djinn.get(djinn_id) {
            // Find the owner's battle unit index (will be assigned once units spawn)
            // For now, store 0 and fix up after spawning
            djinn_state
                .trackers
                .push(crate::battle::types::DjinnTracker {
                    djinn_id: djinn_id.clone(),
                    state: crate::battle::types::DjinnBattleState::Set,
                    owner_unit_id: 0, // fixed up below
                    last_activated_turn: 0,
                    recovery_turns_remaining: 0,
                });
            let _ = djinn_def; // used for validation
        }
    }

    for (i, unit_id) in party.active.iter().enumerate() {
        let Some(def) = game_data.units.get(unit_id) else {
            continue;
        };

        // Retrieve persisted level/XP or start at level 1
        let (level, xp) = party.unit_levels.get(unit_id).copied().unwrap_or((1, 0));
        let lvl = (level as i32 - 1).max(0);

        // Calculate level-scaled stats (base + growth * (level - 1))
        let base_hp = def.base_hp + def.growth.hp * lvl;
        let base_pp = def.base_pp + def.growth.pp * lvl;
        let base_atk = def.base_atk + def.growth.atk * lvl;
        let base_def = def.base_def + def.growth.def * lvl;
        let base_mag = def.base_mag + def.growth.mag * lvl;
        let base_spd = def.base_spd + def.growth.spd * lvl;

        // Sum equipment stat bonuses
        let (eq_hp, eq_pp, eq_atk, eq_def, eq_mag, eq_spd) =
            equipment_stat_bonuses(unit_id, &party, &game_data);

        let hp = base_hp + eq_hp;
        let pp = base_pp + eq_pp;
        let atk = base_atk + eq_atk;
        let defense = base_def + eq_def;
        let mag = base_mag + eq_mag;
        let spd = base_spd + eq_spd;

        // Collect unlocked abilities for this level
        let ability_ids: Vec<String> = def
            .abilities
            .iter()
            .filter(|a| a.unlock_level <= level)
            .map(|a| a.ability_id.clone())
            .collect();

        // Also collect abilities unlocked by equipment
        let mut equip_abilities = equipment_granted_abilities(unit_id, &party, &game_data);

        let mut all_abilities = ability_ids;
        all_abilities.append(&mut equip_abilities);

        // Collect djinn assigned to this unit and apply set bonuses
        let unit_djinn_ids: Vec<String> = party
            .djinn_assignments
            .iter()
            .filter(|(_, owner)| owner.as_str() == unit_id)
            .map(|(djinn_id, _)| djinn_id.clone())
            .collect();

        // Sum set bonuses from all assigned djinn
        let (mut dj_atk, mut dj_def, mut dj_mag, mut dj_spd, mut dj_hp, mut dj_pp) =
            (0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
        for djinn_id in &unit_djinn_ids {
            if let Some(djinn_def) = game_data.djinn.get(djinn_id) {
                dj_atk += djinn_def.set_bonus.atk;
                dj_def += djinn_def.set_bonus.def;
                dj_mag += djinn_def.set_bonus.mag;
                dj_spd += djinn_def.set_bonus.spd;
                dj_hp += djinn_def.set_bonus.hp;
                dj_pp += djinn_def.set_bonus.pp;
                // Add djinn-granted abilities
                all_abilities.extend(djinn_def.granted_ability_ids.clone());
            }
        }

        let hp = hp + dj_hp;
        let pp = pp + dj_pp;
        let atk = atk + dj_atk;
        let defense = defense + dj_def;
        let mag = mag + dj_mag;
        let spd = spd + dj_spd;

        // Respect persisted HP/PP if available (clamped to current max)
        let (current_hp, current_pp) = party.unit_hp_pp.get(unit_id).copied().unwrap_or((hp, pp));
        let current_hp = current_hp.clamp(1, hp); // at least 1 HP to enter battle alive
        let current_pp = current_pp.clamp(0, pp);

        let battle_id = (i + 1) as u32;

        // Fix up djinn tracker owner IDs
        for djinn_id in &unit_djinn_ids {
            if let Some(tracker) = djinn_state
                .trackers
                .iter_mut()
                .find(|t| t.djinn_id == *djinn_id)
            {
                tracker.owner_unit_id = battle_id;
            }
        }

        let battle_unit = BattleUnit {
            id: battle_id,
            name: def.name.clone(),
            side: UnitSide::Player,
            element: def.element,
            level,
            hp: current_hp,
            max_hp: hp,
            pp: current_pp,
            max_pp: pp,
            atk,
            def: defense,
            mag,
            spd,
            luck: 5 + i32::from(level / 2),
            status_effects: Vec::new(),
            ability_ids: all_abilities,
            djinn_ids: unit_djinn_ids,
            damage_taken: 0,
            damage_dealt: 0,
            xp,
            growth_rates: GrowthRates {
                hp: def.growth.hp,
                pp: def.growth.pp,
                atk: def.growth.atk,
                def: def.growth.def,
                mag: def.growth.mag,
                spd: def.growth.spd,
            },
        };

        commands.spawn((PartyCombatant, battle_unit));
    }
}

/// Sum stat bonuses from all equipment slots for a given unit.
fn equipment_stat_bonuses(
    unit_id: &str,
    party: &Party,
    game_data: &GameData,
) -> (i32, i32, i32, i32, i32, i32) {
    let mut hp = 0;
    let mut pp = 0;
    let mut atk = 0;
    let mut def = 0;
    let mut mag = 0;
    let mut spd = 0;

    if let Some(slots) = party.equipment.get(unit_id) {
        for eq_id in slots.values() {
            if let Some(eq_def) = game_data.equipment.get(eq_id) {
                hp += eq_def.stat_bonus.hp;
                pp += eq_def.stat_bonus.pp;
                atk += eq_def.stat_bonus.atk;
                def += eq_def.stat_bonus.def;
                mag += eq_def.stat_bonus.mag;
                spd += eq_def.stat_bonus.spd;
            }
        }
    }

    (hp, pp, atk, def, mag, spd)
}

/// Collect ability IDs granted by equipped items.
fn equipment_granted_abilities(unit_id: &str, party: &Party, game_data: &GameData) -> Vec<String> {
    let mut abilities = Vec::new();
    if let Some(slots) = party.equipment.get(unit_id) {
        for eq_id in slots.values() {
            if let Some(eq_def) = game_data.equipment.get(eq_id)
                && let Some(ref ability_id) = eq_def.unlocks_ability
            {
                abilities.push(ability_id.clone());
            }
        }
    }
    abilities
}

/// Despawn party BattleUnit entities when leaving battle.
pub fn despawn_party_battle_units(
    mut commands: Commands,
    query: Query<Entity, With<PartyCombatant>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------------------------
// Battle enter / exit
// ---------------------------------------------------------------------------

pub fn battle_enter_system(
    mut commands: Commands,
    mut start_events: EventReader<StartBattleEvent>,
    mut battle_state: ResMut<BattleStateRes>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut rng: ResMut<BattleRng>,
    party_query: Query<&BattleUnit>,
) {
    for event in start_events.read() {
        *battle_state = BattleStateRes {
            turn_number: 1,
            encounter_id: event.encounter_id.clone(),
            ..Default::default()
        };

        for enemy in &event.enemy_units {
            commands.spawn(enemy.clone());
        }

        let all_units: Vec<BattleUnit> = party_query
            .iter()
            .cloned()
            .chain(event.enemy_units.iter().cloned())
            .collect();

        battle_state.turn_order = turn_order::calculate_turn_order(&all_units, &mut rng.0);

        let player_count = all_units
            .iter()
            .filter(|u| u.side == UnitSide::Player && u.is_alive())
            .count();
        *cmd_state = CommandSelectState {
            pending_actions: vec![None; player_count],
            ..Default::default()
        };
    }
}

pub fn battle_exit_system(mut commands: Commands, enemy_query: Query<(Entity, &BattleUnit)>) {
    for (entity, unit) in enemy_query.iter() {
        if unit.side == UnitSide::Enemy {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Command Select
// ---------------------------------------------------------------------------

pub fn command_select_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut battle_state: ResMut<BattleStateRes>,
    mut next_phase: ResMut<NextState<BattlePhase>>,
    units: Query<&BattleUnit>,
    game_data: Res<GameData>,
    party: Res<Party>,
) {
    let player_units: Vec<&BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player && u.is_alive())
        .collect();

    if player_units.is_empty() || cmd_state.selecting_unit_index >= player_units.len() {
        // Transfer pending actions
        for (i, action) in cmd_state.pending_actions.iter().enumerate() {
            if let (Some(action), Some(unit)) = (action, player_units.get(i)) {
                battle_state.actions.push((unit.id, action.clone()));
            }
        }
        next_phase.set(BattlePhase::AiSelect);
        return;
    }

    match cmd_state.menu {
        CommandMenu::TopLevel => {
            let menu_size = 6;
            if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
                cmd_state.cursor_index -= 1;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown) && cmd_state.cursor_index < menu_size - 1 {
                cmd_state.cursor_index += 1;
            }
            if keyboard.just_pressed(KeyCode::Enter) {
                match cmd_state.cursor_index {
                    0 => {
                        cmd_state.menu = CommandMenu::TargetSelect;
                        cmd_state.cursor_index = 0;
                        cmd_state.selected_ability = None;
                    }
                    1 => {
                        cmd_state.menu = CommandMenu::AbilitySelect;
                        cmd_state.cursor_index = 0;
                    }
                    2 => {
                        cmd_state.menu = CommandMenu::DjinnSelect;
                        cmd_state.cursor_index = 0;
                    }
                    3 => {
                        cmd_state.menu = CommandMenu::ItemSelect;
                        cmd_state.cursor_index = 0;
                    }
                    4 => {
                        set_pending_action(&mut cmd_state, BattleAction::Defend);
                    }
                    5 => {
                        set_pending_action(&mut cmd_state, BattleAction::Flee);
                    }
                    _ => {}
                }
            }
        }
        CommandMenu::AbilitySelect => {
            if keyboard.just_pressed(KeyCode::Escape) {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                return;
            }
            let unit = player_units[cmd_state.selecting_unit_index];
            let affordable: Vec<&AbilityDef> = unit
                .ability_ids
                .iter()
                .filter_map(|id| game_data.abilities.get(id))
                .filter(|a| a.mana_cost <= unit.pp && a.unlock_level <= unit.level)
                .collect();
            if affordable.is_empty() {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                return;
            }
            if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
                cmd_state.cursor_index -= 1;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown)
                && cmd_state.cursor_index < affordable.len() - 1
            {
                cmd_state.cursor_index += 1;
            }
            if keyboard.just_pressed(KeyCode::Enter)
                && let Some(ability) = affordable.get(cmd_state.cursor_index)
            {
                cmd_state.selected_ability = Some(ability.id.clone());
                cmd_state.menu = CommandMenu::TargetSelect;
                cmd_state.cursor_index = 0;
            }
        }
        CommandMenu::TargetSelect => {
            if keyboard.just_pressed(KeyCode::Escape) {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                cmd_state.selected_ability = None;
                return;
            }
            let targets: Vec<&BattleUnit> = units
                .iter()
                .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
                .collect();
            if targets.is_empty() {
                return;
            }
            if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
                cmd_state.cursor_index -= 1;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown)
                && cmd_state.cursor_index < targets.len() - 1
            {
                cmd_state.cursor_index += 1;
            }
            if keyboard.just_pressed(KeyCode::Enter)
                && let Some(target) = targets.get(cmd_state.cursor_index)
            {
                let action = if let Some(ref aid) = cmd_state.selected_ability {
                    BattleAction::Ability {
                        ability_id: aid.clone(),
                        target_id: target.id,
                    }
                } else {
                    BattleAction::Attack {
                        target_id: target.id,
                    }
                };
                set_pending_action(&mut cmd_state, action);
                cmd_state.selected_ability = None;
            }
        }
        CommandMenu::DjinnSelect => {
            if keyboard.just_pressed(KeyCode::Escape) {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                return;
            }
            let unit = player_units[cmd_state.selecting_unit_index];
            if unit.djinn_ids.is_empty() {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                return;
            }
            if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
                cmd_state.cursor_index -= 1;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown)
                && cmd_state.cursor_index < unit.djinn_ids.len() - 1
            {
                cmd_state.cursor_index += 1;
            }
            if keyboard.just_pressed(KeyCode::Enter)
                && let Some(djinn_id) = unit.djinn_ids.get(cmd_state.cursor_index)
            {
                cmd_state.selected_djinn = Some(djinn_id.clone());
                cmd_state.menu = CommandMenu::TargetSelect;
                cmd_state.cursor_index = 0;
            }
        }
        CommandMenu::ItemSelect => {
            if keyboard.just_pressed(KeyCode::Escape) {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                return;
            }

            // Build a deduplicated list of consumable items from party inventory.
            let mut seen = std::collections::HashSet::new();
            let consumable_ids: Vec<String> = party
                .inventory
                .iter()
                .filter(|id| {
                    game_data
                        .items
                        .get(*id)
                        .is_some_and(|def| def.category == ItemCategory::Consumable)
                })
                .filter(|id| seen.insert((*id).clone()))
                .cloned()
                .collect();

            if consumable_ids.is_empty() {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                return;
            }

            if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
                cmd_state.cursor_index -= 1;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown)
                && cmd_state.cursor_index < consumable_ids.len() - 1
            {
                cmd_state.cursor_index += 1;
            }
            if keyboard.just_pressed(KeyCode::Enter)
                && let Some(item_id) = consumable_ids.get(cmd_state.cursor_index)
                && let Some(item_def) = game_data.items.get(item_id)
            {
                let effect = &item_def.effect;
                let unit = player_units[cmd_state.selecting_unit_index];

                let is_offensive = effect.damage_amount > 0;
                let is_revive = effect.revive;

                if is_offensive {
                    // Target first alive enemy
                    let target = units
                        .iter()
                        .find(|u| u.side == UnitSide::Enemy && u.is_alive());
                    if let Some(target) = target {
                        set_pending_action(
                            &mut cmd_state,
                            BattleAction::Item {
                                item_id: item_id.clone(),
                                target_id: target.id,
                            },
                        );
                    }
                } else if is_revive {
                    // Target first KO'd ally, or fall back to self
                    let ko_ally = units
                        .iter()
                        .find(|u| u.side == UnitSide::Player && u.is_ko());
                    let target_id = ko_ally.map(|u| u.id).unwrap_or(unit.id);
                    set_pending_action(
                        &mut cmd_state,
                        BattleAction::Item {
                            item_id: item_id.clone(),
                            target_id,
                        },
                    );
                } else {
                    // Healing / PP / status removal: target the selecting unit
                    set_pending_action(
                        &mut cmd_state,
                        BattleAction::Item {
                            item_id: item_id.clone(),
                            target_id: unit.id,
                        },
                    );
                }
            }
        }
    }
}

fn set_pending_action(cmd_state: &mut ResMut<CommandSelectState>, action: BattleAction) {
    let idx = cmd_state.selecting_unit_index;
    if idx < cmd_state.pending_actions.len() {
        cmd_state.pending_actions[idx] = Some(action);
        cmd_state.selecting_unit_index += 1;
        cmd_state.menu = CommandMenu::TopLevel;
        cmd_state.cursor_index = 0;
    }
}

// ---------------------------------------------------------------------------
// AI Select
// ---------------------------------------------------------------------------

pub fn ai_select_system(
    mut battle_state: ResMut<BattleStateRes>,
    mut rng: ResMut<BattleRng>,
    mut next_phase: ResMut<NextState<BattlePhase>>,
    units: Query<&BattleUnit>,
    game_data: Res<GameData>,
) {
    let enemies: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
        .cloned()
        .collect();
    let players: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player && u.is_alive())
        .cloned()
        .collect();

    for enemy in &enemies {
        let action =
            ai::enemy_choose_action(enemy, &enemies, &players, &game_data.abilities, &mut rng.0);
        battle_state.actions.push((enemy.id, action));
    }
    next_phase.set(BattlePhase::Resolution);
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn resolution_system(
    mut battle_state: ResMut<BattleStateRes>,
    mut rng: ResMut<BattleRng>,
    mut next_phase: ResMut<NextState<BattlePhase>>,
    mut units: Query<&mut BattleUnit>,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
    mut ko_events: EventWriter<UnitKoEvent>,
    mut status_events: EventWriter<StatusAppliedEvent>,
    mut djinn_state: ResMut<DjinnBattleRes>,
    game_data: Res<GameData>,
    mut party: ResMut<Party>,
) {
    let idx = battle_state.current_actor_index;

    if idx >= battle_state.turn_order.len() {
        // End of round
        battle_state.turn_number += 1;
        battle_state.current_actor_index = 0;
        battle_state.actions.clear();

        let _recovered = djinn::tick_djinn_recovery(&mut djinn_state);

        let all_units: Vec<BattleUnit> = units.iter().cloned().collect();
        battle_state.turn_order = turn_order::calculate_turn_order(&all_units, &mut rng.0);

        let players_alive = units
            .iter()
            .any(|u| u.side == UnitSide::Player && u.is_alive());
        let enemies_alive = units
            .iter()
            .any(|u| u.side == UnitSide::Enemy && u.is_alive());

        if !enemies_alive {
            next_phase.set(BattlePhase::Victory);
        } else if !players_alive {
            next_phase.set(BattlePhase::Defeat);
        } else {
            next_phase.set(BattlePhase::CommandSelect);
        }
        return;
    }

    let actor_id = battle_state.turn_order[idx];
    battle_state.current_actor_index += 1;

    let action = battle_state
        .actions
        .iter()
        .find(|(id, _)| *id == actor_id)
        .map(|(_, a)| a.clone());
    let action = match action {
        Some(a) => a,
        None => return,
    };

    if !units.iter().any(|u| u.id == actor_id && u.is_alive()) {
        return;
    }

    // Status tick
    {
        if let Some(mut actor) = units.iter_mut().find(|u| u.id == actor_id) {
            let _tick = status::tick_status_effects(&mut actor, &mut rng.0);
            if actor.is_ko() {
                ko_events.send(UnitKoEvent {
                    unit_id: actor.id,
                    unit_name: actor.name.clone(),
                    side: actor.side,
                });
                return;
            }
            if status::is_frozen_or_stunned(&actor) {
                return;
            }
            if status::check_paralyze_failure(&actor, &mut rng.0) {
                return;
            }
        }
    }

    // Execute action
    match action {
        BattleAction::Attack { target_id } => {
            execute_basic_attack(
                actor_id,
                target_id,
                &mut units,
                &mut rng,
                &mut damage_events,
                &mut ko_events,
                &battle_state,
            );
        }
        BattleAction::Ability {
            ability_id,
            target_id,
        } => {
            execute_ability(
                actor_id,
                &ability_id,
                target_id,
                &mut units,
                &mut rng,
                &mut damage_events,
                &mut heal_events,
                &mut ko_events,
                &battle_state,
                &game_data,
            );
        }
        BattleAction::Defend => {}
        BattleAction::Flee => {
            let avg_player_spd = avg_speed(&units, UnitSide::Player);
            let avg_enemy_spd = avg_speed(&units, UnitSide::Enemy);
            let chance = rewards::flee_chance(avg_player_spd, avg_enemy_spd);
            if rng.0.r#gen::<f32>() < chance {
                battle_state.fled = true;
                next_phase.set(BattlePhase::Inactive);
            }
        }
        BattleAction::DjinnUnleash {
            djinn_id,
            target_id,
        } => {
            let _ = djinn::unleash_djinn(&djinn_id, battle_state.turn_number, &mut djinn_state);
            execute_basic_attack(
                actor_id,
                target_id,
                &mut units,
                &mut rng,
                &mut damage_events,
                &mut ko_events,
                &battle_state,
            );
        }
        BattleAction::Summon { djinn_ids } => {
            // Status chance for summon effects (per-enemy roll).
            const SUMMON_STATUS_CHANCE: f32 = 0.75;

            if let Ok(summon_result) = djinn::summon_djinn_enhanced(
                &djinn_ids,
                battle_state.turn_number,
                &mut djinn_state,
                &game_data.djinn,
            ) {
                // Apply damage to all alive enemies
                if summon_result.total_damage > 0 {
                    let enemy_ids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
                        .map(|u| u.id)
                        .collect();
                    for eid in enemy_ids {
                        if let Some(mut target) = units.iter_mut().find(|u| u.id == eid) {
                            let result = damage::apply_damage_with_shields(
                                &mut target,
                                summon_result.total_damage,
                            );
                            damage_events.send(DamageEvent {
                                attacker_id: actor_id,
                                target_id: eid,
                                damage: result.actual_damage,
                                element: None,
                                was_blocked: result.was_blocked,
                            });
                            if target.is_ko() {
                                ko_events.send(UnitKoEvent {
                                    unit_id: target.id,
                                    unit_name: target.name.clone(),
                                    side: target.side,
                                });
                            }
                        }
                    }
                }

                // Apply healing to all alive party members
                if summon_result.total_healing > 0 {
                    let ally_ids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == UnitSide::Player && u.is_alive())
                        .map(|u| u.id)
                        .collect();
                    for aid in ally_ids {
                        if let Some(mut ally) = units.iter_mut().find(|u| u.id == aid) {
                            let old_hp = ally.hp;
                            damage::apply_healing(&mut ally, summon_result.total_healing, false);
                            let healed = ally.hp - old_hp;
                            heal_events.send(HealEvent {
                                source_id: actor_id,
                                target_id: aid,
                                amount: healed,
                                revived: false,
                            });
                        }
                    }
                }

                // Apply stat buffs to the caster
                for (stat_name, amount) in &summon_result.stat_buffs {
                    let stat = match stat_name.as_str() {
                        "atk" => StatKind::Atk,
                        "def" => StatKind::Def,
                        "mag" => StatKind::Mag,
                        "spd" => StatKind::Spd,
                        _ => continue,
                    };
                    let buff_effect = if *amount > 0 {
                        BattleStatusEffect::Buff {
                            stat,
                            modifier: *amount,
                            duration: 3,
                        }
                    } else {
                        BattleStatusEffect::Debuff {
                            stat,
                            modifier: *amount,
                            duration: 3,
                        }
                    };
                    if let Some(mut actor) = units.iter_mut().find(|u| u.id == actor_id) {
                        let applied = status::apply_status_to_unit(&mut actor, buff_effect.clone());
                        status_events.send(StatusAppliedEvent {
                            target_id: actor_id,
                            status: buff_effect,
                            was_immune: !applied,
                        });
                    }
                }

                // Apply status effects to enemies with a chance roll
                for (effect_type, duration) in &summon_result.status_inflicts {
                    let battle_status = match effect_type.as_str() {
                        "poison" => BattleStatusEffect::Poison {
                            duration: *duration as i32,
                        },
                        "burn" => BattleStatusEffect::Burn {
                            duration: *duration as i32,
                        },
                        "freeze" => BattleStatusEffect::Freeze {
                            duration: *duration as i32,
                        },
                        "paralyze" => BattleStatusEffect::Paralyze {
                            duration: *duration as i32,
                        },
                        "stun" => BattleStatusEffect::Stun {
                            duration: *duration as i32,
                        },
                        "blind" => BattleStatusEffect::Blind {
                            duration: *duration as i32,
                        },
                        _ => continue,
                    };
                    let enemy_ids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
                        .map(|u| u.id)
                        .collect();
                    for eid in enemy_ids {
                        // Roll against status chance for each enemy
                        if rng.0.r#gen::<f32>() < SUMMON_STATUS_CHANCE
                            && let Some(mut target) = units.iter_mut().find(|u| u.id == eid)
                        {
                            let applied =
                                status::apply_status_to_unit(&mut target, battle_status.clone());
                            status_events.send(StatusAppliedEvent {
                                target_id: eid,
                                status: battle_status.clone(),
                                was_immune: !applied,
                            });
                        }
                    }
                }
            }
        }
        BattleAction::Item { item_id, target_id } => {
            if let Some(item_def) = game_data.items.get(&item_id).cloned() {
                // Remove one instance from party inventory
                if let Some(pos) = party.inventory.iter().position(|id| id == &item_id) {
                    party.inventory.remove(pos);
                }

                let effect = &item_def.effect;

                // Apply healing / PP restoration
                if (effect.hp_restore > 0 || effect.pp_restore > 0)
                    && let Some(mut target) = units.iter_mut().find(|u| u.id == target_id)
                {
                    if effect.revive && target.is_ko() {
                        target.hp = effect.hp_restore.min(target.max_hp);
                        heal_events.send(HealEvent {
                            source_id: actor_id,
                            target_id,
                            amount: target.hp,
                            revived: true,
                        });
                    } else if target.is_alive() {
                        let old_hp = target.hp;
                        target.hp = (target.hp + effect.hp_restore).min(target.max_hp);
                        target.pp = (target.pp + effect.pp_restore).min(target.max_pp);
                        let healed = target.hp - old_hp;
                        heal_events.send(HealEvent {
                            source_id: actor_id,
                            target_id,
                            amount: healed,
                            revived: false,
                        });
                    }
                }

                // Apply status removal
                if !effect.removes_status.is_empty()
                    && let Some(mut target) = units.iter_mut().find(|u| u.id == target_id)
                {
                    target.status_effects.retain(|se| {
                        let kind_str = match se.kind() {
                            StatusKind::Poison => "poison",
                            StatusKind::Burn => "burn",
                            StatusKind::Freeze => "freeze",
                            StatusKind::Paralyze => "paralyze",
                            StatusKind::Stun => "stun",
                            StatusKind::Blind => "blind",
                            _ => return true,
                        };
                        !effect.removes_status.iter().any(|r| r == kind_str)
                    });
                }

                // Apply damage (offensive items)
                if effect.damage_amount > 0
                    && let Some(mut target) = units.iter_mut().find(|u| u.id == target_id)
                {
                    let result =
                        damage::apply_damage_with_shields(&mut target, effect.damage_amount);
                    damage_events.send(DamageEvent {
                        attacker_id: actor_id,
                        target_id,
                        damage: result.actual_damage,
                        element: effect.damage_element,
                        was_blocked: result.was_blocked,
                    });
                    if target.is_ko() {
                        ko_events.send(UnitKoEvent {
                            unit_id: target.id,
                            unit_name: target.name.clone(),
                            side: target.side,
                        });
                    }
                }
            }
        }
    }
}

fn avg_speed(units: &Query<&mut BattleUnit>, side: UnitSide) -> f32 {
    let speeds: Vec<f32> = units
        .iter()
        .filter(|u| u.side == side && u.is_alive())
        .map(|u| u.spd as f32)
        .collect();
    if speeds.is_empty() {
        0.0
    } else {
        speeds.iter().sum::<f32>() / speeds.len() as f32
    }
}

fn execute_basic_attack(
    attacker_id: u32,
    target_id: u32,
    units: &mut Query<&mut BattleUnit>,
    rng: &mut ResMut<BattleRng>,
    damage_events: &mut EventWriter<DamageEvent>,
    ko_events: &mut EventWriter<UnitKoEvent>,
    battle_state: &ResMut<BattleStateRes>,
) {
    let attacker_data = units.iter().find(|u| u.id == attacker_id).cloned();
    let attacker = match attacker_data {
        Some(a) => a,
        None => return,
    };

    let target_defending = battle_state
        .actions
        .iter()
        .any(|(id, a)| *id == target_id && matches!(a, BattleAction::Defend));

    let basic_attack = AbilityDef {
        id: "basic_attack".into(),
        name: "Attack".into(),
        ability_type: AbilityType::Physical,
        element: None,
        mana_cost: 0,
        base_power: 0,
        targets: TargetKind::SingleEnemy,
        unlock_level: 1,
        description: String::new(),
        buff_effect: None,
        duration: None,
        status_effect: None,
        chain_damage: false,
        ignore_defense_percent: 0.0,
        damage_reduction_percent: 0.0,
        shield_charges: None,
        ai_hints: AiHints {
            priority: 1.0,
            target: AiTargetPref::Weakest,
            avoid_overkill: false,
            opener: false,
        },
    };

    if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
        // Accuracy check — miss if blind or unlucky
        if !damage::check_accuracy(&attacker, &target, &mut rng.0) {
            damage_events.send(DamageEvent {
                attacker_id,
                target_id,
                damage: 0,
                element: Some(attacker.element),
                was_blocked: false,
            });
            return;
        }

        let mut dmg = damage::calculate_physical_damage(
            &attacker,
            &target,
            &basic_attack,
            target_defending,
            &mut rng.0,
        );

        // Critical hit check
        if damage::calculate_crit(&attacker, &mut rng.0) {
            dmg = (dmg as f32 * 1.5) as i32;
        }

        let result = damage::apply_damage_with_shields(&mut target, dmg);
        damage_events.send(DamageEvent {
            attacker_id,
            target_id,
            damage: result.actual_damage,
            element: Some(attacker.element),
            was_blocked: result.was_blocked,
        });
        if target.is_ko() {
            ko_events.send(UnitKoEvent {
                unit_id: target.id,
                unit_name: target.name.clone(),
                side: target.side,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_ability(
    caster_id: u32,
    ability_id: &str,
    target_id: u32,
    units: &mut Query<&mut BattleUnit>,
    rng: &mut ResMut<BattleRng>,
    damage_events: &mut EventWriter<DamageEvent>,
    heal_events: &mut EventWriter<HealEvent>,
    ko_events: &mut EventWriter<UnitKoEvent>,
    battle_state: &ResMut<BattleStateRes>,
    game_data: &Res<GameData>,
) {
    let caster_data = units.iter().find(|u| u.id == caster_id).cloned();
    let caster = match caster_data {
        Some(c) => c,
        None => return,
    };

    let ability = match game_data.abilities.get(ability_id) {
        Some(a) => a.clone(),
        None => return,
    };

    // Deduct PP
    if let Some(mut caster_unit) = units.iter_mut().find(|u| u.id == caster_id) {
        caster_unit.pp = (caster_unit.pp - ability.mana_cost).max(0);
    }

    match ability.ability_type {
        AbilityType::Healing => {
            let heal_amount = damage::calculate_heal_amount(&caster, &ability);
            match ability.targets {
                TargetKind::SingleAlly | TargetKind::OneSelf => {
                    if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
                        let was_ko = target.is_ko();
                        damage::apply_healing(&mut target, heal_amount, false);
                        heal_events.send(HealEvent {
                            source_id: caster_id,
                            target_id,
                            amount: heal_amount,
                            revived: was_ko && target.is_alive(),
                        });
                    }
                }
                TargetKind::AllAllies => {
                    let ally_ids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == caster.side)
                        .map(|u| u.id)
                        .collect();
                    for aid in ally_ids {
                        if let Some(mut ally) = units.iter_mut().find(|u| u.id == aid) {
                            let was_ko = ally.is_ko();
                            damage::apply_healing(&mut ally, heal_amount, false);
                            heal_events.send(HealEvent {
                                source_id: caster_id,
                                target_id: aid,
                                amount: heal_amount,
                                revived: was_ko && ally.is_alive(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        AbilityType::Physical | AbilityType::Psynergy | AbilityType::Debuff => {
            let target_defending = battle_state
                .actions
                .iter()
                .any(|(id, a)| *id == target_id && matches!(a, BattleAction::Defend));

            match ability.targets {
                TargetKind::SingleEnemy | TargetKind::SingleAlly | TargetKind::OneSelf => {
                    if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
                        // Accuracy check for offensive abilities
                        if !damage::check_accuracy(&caster, &target, &mut rng.0) {
                            damage_events.send(DamageEvent {
                                attacker_id: caster_id,
                                target_id,
                                damage: 0,
                                element: ability.element,
                                was_blocked: false,
                            });
                        } else {
                            let mut dmg = damage::calculate_damage(
                                &caster,
                                &target,
                                &ability,
                                target_defending,
                                &mut rng.0,
                            );
                            if damage::calculate_crit(&caster, &mut rng.0) {
                                dmg = (dmg as f32 * 1.5) as i32;
                            }
                            let result = damage::apply_damage_with_shields(&mut target, dmg);
                            damage_events.send(DamageEvent {
                                attacker_id: caster_id,
                                target_id,
                                damage: result.actual_damage,
                                element: ability.element,
                                was_blocked: result.was_blocked,
                            });
                            // Apply status from ability
                            if let Some(ref se) = ability.status_effect
                                && let Some(battle_status) = convert_status_effect(se)
                            {
                                status::apply_status_to_unit(&mut target, battle_status);
                            }
                            if target.is_ko() {
                                ko_events.send(UnitKoEvent {
                                    unit_id: target.id,
                                    unit_name: target.name.clone(),
                                    side: target.side,
                                });
                            }
                        }
                    }
                }
                TargetKind::AllEnemies => {
                    let enemy_side = if caster.side == UnitSide::Player {
                        UnitSide::Enemy
                    } else {
                        UnitSide::Player
                    };
                    let eids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == enemy_side && u.is_alive())
                        .map(|u| u.id)
                        .collect();
                    for eid in eids {
                        let defending = battle_state
                            .actions
                            .iter()
                            .any(|(id, a)| *id == eid && matches!(a, BattleAction::Defend));
                        if let Some(mut target) = units.iter_mut().find(|u| u.id == eid) {
                            // Accuracy check per target
                            if !damage::check_accuracy(&caster, &target, &mut rng.0) {
                                damage_events.send(DamageEvent {
                                    attacker_id: caster_id,
                                    target_id: eid,
                                    damage: 0,
                                    element: ability.element,
                                    was_blocked: false,
                                });
                                continue;
                            }
                            let mut dmg = damage::calculate_damage(
                                &caster, &target, &ability, defending, &mut rng.0,
                            );
                            if damage::calculate_crit(&caster, &mut rng.0) {
                                dmg = (dmg as f32 * 1.5) as i32;
                            }
                            let result = damage::apply_damage_with_shields(&mut target, dmg);
                            damage_events.send(DamageEvent {
                                attacker_id: caster_id,
                                target_id: eid,
                                damage: result.actual_damage,
                                element: ability.element,
                                was_blocked: result.was_blocked,
                            });
                            if let Some(ref se) = ability.status_effect
                                && let Some(battle_status) = convert_status_effect(se)
                            {
                                status::apply_status_to_unit(&mut target, battle_status);
                            }
                            if target.is_ko() {
                                ko_events.send(UnitKoEvent {
                                    unit_id: target.id,
                                    unit_name: target.name.clone(),
                                    side: target.side,
                                });
                            }
                        }
                    }
                }
                TargetKind::AllAllies => {
                    // Buff/debuff all allies via status effect
                    if let Some(ref se) = ability.status_effect {
                        let aids: Vec<u32> = units
                            .iter()
                            .filter(|u| u.side == caster.side && u.is_alive())
                            .map(|u| u.id)
                            .collect();
                        for aid in aids {
                            if let Some(mut ally) = units.iter_mut().find(|u| u.id == aid)
                                && let Some(battle_status) = convert_status_effect(se)
                            {
                                status::apply_status_to_unit(&mut ally, battle_status);
                            }
                        }
                    }
                }
            }
        }
        AbilityType::Buff => {
            // Apply buff from ability's buff_effect as a BattleStatusEffect
            if let Some(ref buff) = ability.buff_effect {
                let duration = ability.duration.unwrap_or(3) as i32;
                let statuses = buff_to_status_effects(buff, duration);
                match ability.targets {
                    TargetKind::OneSelf | TargetKind::SingleAlly => {
                        if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
                            for s in statuses {
                                status::apply_status_to_unit(&mut target, s);
                            }
                        }
                    }
                    TargetKind::AllAllies => {
                        let aids: Vec<u32> = units
                            .iter()
                            .filter(|u| u.side == caster.side && u.is_alive())
                            .map(|u| u.id)
                            .collect();
                        for aid in aids {
                            if let Some(mut ally) = units.iter_mut().find(|u| u.id == aid) {
                                let sts = buff_to_status_effects(buff, duration);
                                for s in sts {
                                    status::apply_status_to_unit(&mut ally, s);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Convert a data::abilities::StatusEffectDef into a BattleStatusEffect.
fn convert_status_effect(
    se: &crate::data::abilities::StatusEffectDef,
) -> Option<BattleStatusEffect> {
    let dur = se.duration as i32;
    match se.effect_type.as_str() {
        "poison" => Some(BattleStatusEffect::Poison { duration: dur }),
        "burn" => Some(BattleStatusEffect::Burn { duration: dur }),
        "freeze" => Some(BattleStatusEffect::Freeze { duration: dur }),
        "paralyze" => Some(BattleStatusEffect::Paralyze { duration: dur }),
        "stun" => Some(BattleStatusEffect::Stun { duration: dur }),
        "blind" => Some(BattleStatusEffect::Blind { duration: dur }),
        _ => None,
    }
}

/// Convert a BuffEffect into one or more BattleStatusEffect::Buff entries.
fn buff_to_status_effects(
    buff: &crate::data::abilities::BuffEffect,
    duration: i32,
) -> Vec<BattleStatusEffect> {
    let mut effects = Vec::new();
    if buff.atk != 0 {
        let (kind, modifier) = if buff.atk > 0 {
            (true, buff.atk)
        } else {
            (false, buff.atk)
        };
        if kind {
            effects.push(BattleStatusEffect::Buff {
                stat: StatKind::Atk,
                modifier,
                duration,
            });
        } else {
            effects.push(BattleStatusEffect::Debuff {
                stat: StatKind::Atk,
                modifier,
                duration,
            });
        }
    }
    if buff.def != 0 {
        if buff.def > 0 {
            effects.push(BattleStatusEffect::Buff {
                stat: StatKind::Def,
                modifier: buff.def,
                duration,
            });
        } else {
            effects.push(BattleStatusEffect::Debuff {
                stat: StatKind::Def,
                modifier: buff.def,
                duration,
            });
        }
    }
    if buff.mag != 0 {
        if buff.mag > 0 {
            effects.push(BattleStatusEffect::Buff {
                stat: StatKind::Mag,
                modifier: buff.mag,
                duration,
            });
        } else {
            effects.push(BattleStatusEffect::Debuff {
                stat: StatKind::Mag,
                modifier: buff.mag,
                duration,
            });
        }
    }
    if buff.spd != 0 {
        if buff.spd > 0 {
            effects.push(BattleStatusEffect::Buff {
                stat: StatKind::Spd,
                modifier: buff.spd,
                duration,
            });
        } else {
            effects.push(BattleStatusEffect::Debuff {
                stat: StatKind::Spd,
                modifier: buff.spd,
                duration,
            });
        }
    }
    effects
}

// ---------------------------------------------------------------------------
// Victory / Defeat
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn victory_system(
    mut units: Query<&mut BattleUnit>,
    mut end_events: EventWriter<EndBattleEvent>,
    game_data: Res<GameData>,
    mut battle_rng: ResMut<BattleRng>,
    mut party: ResMut<Party>,
) {
    let enemy_xp_gold: Vec<(u32, u32)> = units
        .iter()
        .filter(|u| u.side == UnitSide::Enemy)
        .filter_map(|u| {
            game_data
                .enemies
                .values()
                .find(|e| e.name == u.name)
                .map(|e| (e.base_xp, e.base_gold))
        })
        .collect();

    let mut party_units: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player)
        .cloned()
        .collect();
    let party_size = party_units.len() as u32;
    let survivor_count = party_units.iter().filter(|u| u.is_alive()).count() as u32;

    let battle_rewards = rewards::calculate_battle_rewards(
        &enemy_xp_gold,
        party_size,
        survivor_count,
        &mut battle_rng.0,
    );

    // Build ability unlock map: unit_id -> [(unlock_level, ability_id)]
    let mut ability_unlocks = std::collections::HashMap::new();
    for unit in &party_units {
        let unlocks: Vec<(u8, String)> = game_data
            .units
            .values()
            .find(|def| def.name == unit.name)
            .map(|def| {
                def.abilities
                    .iter()
                    .map(|a| (a.unlock_level, a.ability_id.clone()))
                    .collect()
            })
            .unwrap_or_default();
        ability_unlocks.insert(unit.id, unlocks);
    }

    let level_ups =
        rewards::distribute_rewards(&mut party_units, &battle_rewards, &ability_unlocks);

    // Write updated stats back to ECS entities
    for updated in &party_units {
        if let Some(mut unit) = units.iter_mut().find(|u| u.id == updated.id) {
            unit.hp = updated.hp;
            unit.max_hp = updated.max_hp;
            unit.pp = updated.pp;
            unit.max_pp = updated.max_pp;
            unit.atk = updated.atk;
            unit.def = updated.def;
            unit.mag = updated.mag;
            unit.spd = updated.spd;
            unit.level = updated.level;
            unit.xp = updated.xp;
        }
    }

    // Persist levels/XP and HP/PP back to Party resource
    persist_party_state(&mut party, &party_units, &game_data);

    // Award gold
    party.gold += battle_rewards.total_gold;

    // Add dropped items to party inventory
    for item_id in &battle_rewards.item_drops {
        party.inventory.push(item_id.clone());
    }

    // Set first battle won story flag
    if !party.has_flag(story::FIRST_BATTLE_WON) {
        party.set_flag(story::FIRST_BATTLE_WON, true);
    }

    end_events.send(EndBattleEvent {
        victory: true,
        rewards: Some(battle_rewards),
        level_ups,
    });
}

pub fn defeat_system(
    mut end_events: EventWriter<EndBattleEvent>,
    units: Query<&BattleUnit>,
    mut party: ResMut<Party>,
    game_data: Res<GameData>,
) {
    // Even on defeat, persist current HP/PP/level state
    let party_units: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player)
        .cloned()
        .collect();
    persist_party_state(&mut party, &party_units, &game_data);

    end_events.send(EndBattleEvent {
        victory: false,
        rewards: None,
        level_ups: vec![],
    });
}

/// Detects a successful flee (battle_state.fled == true) after the battle phase
/// has transitioned to Inactive, and transitions the game state back to the
/// overworld so the player is not stranded on the battle screen.
pub fn handle_flee_system(
    mut battle_state: ResMut<BattleStateRes>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if battle_state.fled {
        battle_state.fled = false;
        next_game_state.set(GameState::Overworld);
    }
}

/// Write BattleUnit level/XP and HP/PP back to the Party resource for persistence.
fn persist_party_state(party: &mut Party, battle_units: &[BattleUnit], game_data: &GameData) {
    // Build name-to-unit-id mapping
    let name_to_unit_id: std::collections::HashMap<String, String> = game_data
        .units
        .iter()
        .map(|(id, def)| (def.name.clone(), id.clone()))
        .collect();

    for unit in battle_units {
        if let Some(unit_id) = name_to_unit_id.get(&unit.name) {
            party
                .unit_levels
                .insert(unit_id.clone(), (unit.level, unit.xp));
            party.unit_hp_pp.insert(unit_id.clone(), (unit.hp, unit.pp));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::abilities::{BuffEffect, StatusEffectDef};

    /// Helper to build a GameData for tests using the real registry builders.
    fn test_game_data() -> GameData {
        GameData {
            abilities: crate::data::abilities::build_ability_registry(),
            units: crate::data::units::build_unit_registry(),
            enemies: crate::data::enemies::build_enemy_registry(),
            items: crate::data::items::build_item_registry(),
            equipment: crate::data::items::build_equipment_registry(),
            djinn: crate::data::djinn::build_djinn_registry(),
        }
    }

    /// Helper to build a BattleUnit with sensible defaults for testing.
    fn make_battle_unit(id: u32, name: &str, level: u8, hp: i32, pp: i32, xp: u32) -> BattleUnit {
        BattleUnit {
            id,
            name: name.to_string(),
            side: UnitSide::Player,
            element: Element::Venus,
            level,
            hp,
            max_hp: hp,
            pp,
            max_pp: pp,
            atk: 20,
            def: 15,
            mag: 10,
            spd: 12,
            luck: 5,
            status_effects: Vec::new(),
            ability_ids: Vec::new(),
            djinn_ids: Vec::new(),
            damage_taken: 0,
            damage_dealt: 0,
            xp,
            growth_rates: GrowthRates {
                hp: 25,
                pp: 4,
                atk: 3,
                def: 4,
                mag: 2,
                spd: 1,
            },
        }
    }

    #[test]
    fn test_convert_status_effect_poison() {
        let se_def = StatusEffectDef {
            effect_type: "poison".to_string(),
            duration: 3,
            chance: 1.0,
        };
        let result = convert_status_effect(&se_def);
        assert!(
            result.is_some(),
            "poison should convert to a BattleStatusEffect"
        );
        let effect = result.unwrap();
        assert_eq!(effect, BattleStatusEffect::Poison { duration: 3 });
        assert_eq!(effect.kind(), StatusKind::Poison);
    }

    #[test]
    fn test_convert_status_effect_unknown() {
        let se_def = StatusEffectDef {
            effect_type: "petrify".to_string(),
            duration: 5,
            chance: 0.5,
        };
        let result = convert_status_effect(&se_def);
        assert!(result.is_none(), "unknown effect_type should return None");
    }

    #[test]
    fn test_buff_to_status_effects_mixed() {
        let buff = BuffEffect {
            atk: 10,
            def: -5,
            mag: 0,
            spd: 0,
        };
        let effects = buff_to_status_effects(&buff, 4);

        assert_eq!(effects.len(), 2, "should produce exactly 2 status effects");

        // First effect: positive atk -> Buff
        assert_eq!(
            effects[0],
            BattleStatusEffect::Buff {
                stat: StatKind::Atk,
                modifier: 10,
                duration: 4,
            }
        );

        // Second effect: negative def -> Debuff
        assert_eq!(
            effects[1],
            BattleStatusEffect::Debuff {
                stat: StatKind::Def,
                modifier: -5,
                duration: 4,
            }
        );
    }

    #[test]
    fn test_buff_to_status_effects_empty() {
        let buff = BuffEffect {
            atk: 0,
            def: 0,
            mag: 0,
            spd: 0,
        };
        let effects = buff_to_status_effects(&buff, 3);
        assert!(
            effects.is_empty(),
            "all-zero BuffEffect should produce no status effects"
        );
    }

    #[test]
    fn test_equipment_stat_bonuses_no_equipment() {
        let party = Party::default();
        let game_data = test_game_data();

        let (hp, pp, atk, def, mag, spd) = equipment_stat_bonuses("adept", &party, &game_data);

        assert_eq!(hp, 0);
        assert_eq!(pp, 0);
        assert_eq!(atk, 0);
        assert_eq!(def, 0);
        assert_eq!(mag, 0);
        assert_eq!(spd, 0);
    }

    #[test]
    fn test_persist_party_state_writes_levels() {
        let game_data = test_game_data();
        let mut party = Party::default();

        // Create BattleUnits whose names match units in the registry.
        // "adept" maps to the "Adept" unit definition.
        let units = vec![make_battle_unit(1, "Adept", 5, 80, 20, 1200)];

        persist_party_state(&mut party, &units, &game_data);

        // Verify level/XP was written
        let (level, xp) = party
            .unit_levels
            .get("adept")
            .expect("adept should have persisted level/XP");
        assert_eq!(*level, 5);
        assert_eq!(*xp, 1200);

        // Verify HP/PP was written
        let (hp, pp) = party
            .unit_hp_pp
            .get("adept")
            .expect("adept should have persisted HP/PP");
        assert_eq!(*hp, 80);
        assert_eq!(*pp, 20);
    }

    // -----------------------------------------------------------------------
    // Enhanced summon resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_enhanced_summon_damage_uses_real_registry() {
        // Verify that summon_djinn_enhanced returns the correct aggregated
        // damage from the real djinn registry for flint (80) + forge (120).
        let game_data = test_game_data();
        let mut djinn_state = DjinnBattleRes {
            trackers: vec![
                DjinnTracker {
                    djinn_id: "flint".into(),
                    state: DjinnBattleState::Standby,
                    owner_unit_id: 1,
                    last_activated_turn: 0,
                    recovery_turns_remaining: 0,
                },
                DjinnTracker {
                    djinn_id: "forge".into(),
                    state: DjinnBattleState::Standby,
                    owner_unit_id: 1,
                    last_activated_turn: 0,
                    recovery_turns_remaining: 0,
                },
            ],
        };

        let result = djinn::summon_djinn_enhanced(
            &["flint".into(), "forge".into()],
            3,
            &mut djinn_state,
            &game_data.djinn,
        )
        .expect("enhanced summon should succeed");

        // flint(80) + forge(120) = 200 total damage
        assert_eq!(result.total_damage, 200);
        assert_eq!(result.total_healing, 0);
        assert!(result.stat_buffs.is_empty());
        assert!(result.status_inflicts.is_empty());

        // Both djinn should now be in Recovery
        assert!(
            djinn_state
                .trackers
                .iter()
                .all(|t| t.state == DjinnBattleState::Recovery)
        );
    }

    #[test]
    fn test_enhanced_summon_heal_and_status_from_registry() {
        // Summon fizz (heal:100) and fever (status:burn/3) together.
        // Verify the SummonResult aggregates both healing and status effects.
        let game_data = test_game_data();
        let mut djinn_state = DjinnBattleRes {
            trackers: vec![
                DjinnTracker {
                    djinn_id: "fizz".into(),
                    state: DjinnBattleState::Standby,
                    owner_unit_id: 1,
                    last_activated_turn: 0,
                    recovery_turns_remaining: 0,
                },
                DjinnTracker {
                    djinn_id: "fever".into(),
                    state: DjinnBattleState::Standby,
                    owner_unit_id: 1,
                    last_activated_turn: 0,
                    recovery_turns_remaining: 0,
                },
            ],
        };

        let result = djinn::summon_djinn_enhanced(
            &["fizz".into(), "fever".into()],
            5,
            &mut djinn_state,
            &game_data.djinn,
        )
        .expect("enhanced summon should succeed");

        assert_eq!(result.total_damage, 0);
        assert_eq!(result.total_healing, 100);
        assert_eq!(result.status_inflicts.len(), 1);
        assert_eq!(result.status_inflicts[0].0, "burn");
        assert_eq!(result.status_inflicts[0].1, 3);
    }

    // -------------------------------------------------------------------
    // Integration tests: turn order, status effects, AI, flee, elements
    // -------------------------------------------------------------------

    #[test]
    fn test_turn_order_respects_speed() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut slow = make_battle_unit(1, "Slow", 5, 100, 50, 0);
        slow.spd = 5;
        slow.side = UnitSide::Player;

        let mut med = make_battle_unit(2, "Medium", 5, 100, 50, 0);
        med.spd = 15;
        med.side = UnitSide::Enemy;

        let mut fast = make_battle_unit(3, "Fast", 5, 100, 50, 0);
        fast.spd = 30;
        fast.side = UnitSide::Player;

        let units = vec![slow, med, fast];
        let mut rng = StdRng::seed_from_u64(42);
        let order = turn_order::calculate_turn_order(&units, &mut rng);

        assert_eq!(order.len(), 3);
        assert_eq!(order[0], 3, "fastest (spd=30) goes first");
        assert_eq!(order[1], 2, "medium (spd=15) goes second");
        assert_eq!(order[2], 1, "slowest (spd=5) goes last");
    }

    #[test]
    fn test_status_effect_tick() {
        use crate::battle::types::constants;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut unit = make_battle_unit(1, "Poisoned", 5, 200, 50, 0);
        unit.status_effects
            .push(BattleStatusEffect::Poison { duration: 3 });

        let initial_hp = unit.hp;
        let expected_dmg = (unit.max_hp as f32 * constants::POISON_PERCENT).floor() as i32;

        let mut rng = StdRng::seed_from_u64(42);
        let result = status::tick_status_effects(&mut unit, &mut rng);

        assert_eq!(result.damage, expected_dmg);
        assert_eq!(unit.hp, initial_hp - expected_dmg);
        assert!(
            !unit.status_effects.is_empty(),
            "poison (dur 3) should remain active after one tick"
        );
    }

    #[test]
    fn test_ai_targets_weakest() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        use std::collections::HashMap;

        let mut rng = StdRng::seed_from_u64(42);

        let mut enemy = make_battle_unit(1, "Enemy", 5, 100, 50, 0);
        enemy.side = UnitSide::Enemy;

        let mut strong = make_battle_unit(10, "Strong", 5, 90, 50, 0);
        strong.side = UnitSide::Player;
        let mut mid = make_battle_unit(11, "Mid", 5, 50, 50, 0);
        mid.side = UnitSide::Player;
        let mut weak = make_battle_unit(12, "Weak", 5, 15, 50, 0);
        weak.side = UnitSide::Player;

        let targets = vec![strong, mid, weak];
        let registry: HashMap<String, AbilityDef> = HashMap::new();

        let action = ai::enemy_choose_action(
            &enemy,
            std::slice::from_ref(&enemy),
            &targets,
            &registry,
            &mut rng,
        );

        match action {
            BattleAction::Attack { target_id } => {
                assert_eq!(target_id, 12, "should target weakest (id=12, hp=15)");
            }
            other => panic!("Expected Attack, got {:?}", other),
        }
    }

    #[test]
    fn test_flee_probability() {
        use crate::battle::types::constants;

        let equal = rewards::flee_chance(10.0, 10.0);
        assert!((equal - constants::BASE_FLEE_CHANCE).abs() < 0.001);

        let faster = rewards::flee_chance(20.0, 10.0);
        let expected = constants::BASE_FLEE_CHANCE + 10.0 * constants::SPEED_FLEE_BONUS;
        assert!((faster - expected).abs() < 0.001);

        let max = rewards::flee_chance(100.0, 10.0);
        assert!((max - 0.90).abs() < 0.001, "clamped to 0.90");

        let min = rewards::flee_chance(5.0, 100.0);
        assert!((min - 0.10).abs() < 0.001, "clamped to 0.10");
    }

    #[test]
    fn test_damage_with_element_advantage() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut atk_unit = make_battle_unit(1, "Venus", 5, 100, 50, 0);
        atk_unit.element = Element::Venus;
        atk_unit.mag = 25;
        atk_unit.luck = 0;

        let mut def_jup = make_battle_unit(2, "Jupiter", 5, 200, 50, 0);
        def_jup.element = Element::Jupiter;
        def_jup.def = 10;

        let mut def_ven = def_jup.clone();
        def_ven.element = Element::Venus;

        let ability = AbilityDef {
            id: "earth_strike".into(),
            name: "Earth Strike".into(),
            ability_type: AbilityType::Psynergy,
            element: Some(Element::Venus),
            mana_cost: 5,
            base_power: 40,
            targets: TargetKind::SingleEnemy,
            unlock_level: 1,
            description: String::new(),
            buff_effect: None,
            duration: None,
            status_effect: None,
            chain_damage: false,
            ignore_defense_percent: 0.0,
            damage_reduction_percent: 0.0,
            shield_charges: None,
            ai_hints: AiHints {
                priority: 1.0,
                target: AiTargetPref::Weakest,
                avoid_overkill: false,
                opener: false,
            },
        };

        let mut r1 = StdRng::seed_from_u64(42);
        let dmg_adv =
            damage::calculate_psynergy_damage(&atk_unit, &def_jup, &ability, false, &mut r1);
        let mut r2 = StdRng::seed_from_u64(42);
        let dmg_neu =
            damage::calculate_psynergy_damage(&atk_unit, &def_ven, &ability, false, &mut r2);

        assert!(
            dmg_adv > dmg_neu,
            "advantage damage ({}) should exceed neutral ({})",
            dmg_adv,
            dmg_neu
        );
        let m = damage::element_modifier(Element::Venus, Element::Jupiter);
        assert!((m - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_damage_with_element_disadvantage() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut atk_unit = make_battle_unit(1, "Venus", 5, 100, 50, 0);
        atk_unit.element = Element::Venus;
        atk_unit.mag = 25;
        atk_unit.luck = 0;

        let mut def_mars = make_battle_unit(2, "Mars", 5, 200, 50, 0);
        def_mars.element = Element::Mars;
        def_mars.def = 10;

        let mut def_ven = def_mars.clone();
        def_ven.element = Element::Venus;

        let ability = AbilityDef {
            id: "earth_strike".into(),
            name: "Earth Strike".into(),
            ability_type: AbilityType::Psynergy,
            element: Some(Element::Venus),
            mana_cost: 5,
            base_power: 40,
            targets: TargetKind::SingleEnemy,
            unlock_level: 1,
            description: String::new(),
            buff_effect: None,
            duration: None,
            status_effect: None,
            chain_damage: false,
            ignore_defense_percent: 0.0,
            damage_reduction_percent: 0.0,
            shield_charges: None,
            ai_hints: AiHints {
                priority: 1.0,
                target: AiTargetPref::Weakest,
                avoid_overkill: false,
                opener: false,
            },
        };

        let mut r1 = StdRng::seed_from_u64(42);
        let dmg_dis =
            damage::calculate_psynergy_damage(&atk_unit, &def_mars, &ability, false, &mut r1);
        let mut r2 = StdRng::seed_from_u64(42);
        let dmg_neu =
            damage::calculate_psynergy_damage(&atk_unit, &def_ven, &ability, false, &mut r2);

        assert!(
            dmg_dis < dmg_neu,
            "disadvantage damage ({}) should be less than neutral ({})",
            dmg_dis,
            dmg_neu
        );
        let m = damage::element_modifier(Element::Venus, Element::Mars);
        assert!((m - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_enhanced_summon_heals_all_party_members() {
        // Simulate the resolution system's healing path: summon fizz (heal:100),
        // then apply healing to multiple party members who are below max HP.
        let game_data = test_game_data();
        let mut djinn_state = DjinnBattleRes {
            trackers: vec![DjinnTracker {
                djinn_id: "fizz".into(),
                state: DjinnBattleState::Standby,
                owner_unit_id: 1,
                last_activated_turn: 0,
                recovery_turns_remaining: 0,
            }],
        };

        let result =
            djinn::summon_djinn_enhanced(&["fizz".into()], 5, &mut djinn_state, &game_data.djinn)
                .expect("enhanced summon should succeed");

        assert_eq!(result.total_healing, 100);

        // Create two party members both below max HP (simulating the all-ally heal).
        let mut ally1 = make_battle_unit(1, "Adept", 5, 50, 20, 0);
        ally1.max_hp = 120;
        let mut ally2 = make_battle_unit(2, "Mage", 5, 30, 20, 0);
        ally2.max_hp = 100;

        // Apply healing the same way the resolution system does for ALL allies.
        let old_hp1 = ally1.hp;
        damage::apply_healing(&mut ally1, result.total_healing, false);
        let healed1 = ally1.hp - old_hp1;

        let old_hp2 = ally2.hp;
        damage::apply_healing(&mut ally2, result.total_healing, false);
        let healed2 = ally2.hp - old_hp2;

        // Both allies should have been healed.
        assert!(healed1 > 0, "ally1 should have received healing");
        assert!(healed2 > 0, "ally2 should have received healing");
        // ally1: min(50+100, 120) = 120, healed 70
        assert_eq!(ally1.hp, 120);
        assert_eq!(healed1, 70);
        // ally2: min(30+100, 100) = 100, healed 70
        assert_eq!(ally2.hp, 100);
        assert_eq!(healed2, 70);
    }

    #[test]
    fn test_enhanced_summon_status_chance_roll() {
        // Simulate the resolution system's status application with a chance roll.
        // Summon fever (status:burn/3), then for each enemy roll against the chance.
        // Using a seeded RNG, verify that some enemies get the status and some don't.
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let game_data = test_game_data();
        let mut djinn_state = DjinnBattleRes {
            trackers: vec![DjinnTracker {
                djinn_id: "fever".into(),
                state: DjinnBattleState::Standby,
                owner_unit_id: 1,
                last_activated_turn: 0,
                recovery_turns_remaining: 0,
            }],
        };

        let result =
            djinn::summon_djinn_enhanced(&["fever".into()], 5, &mut djinn_state, &game_data.djinn)
                .expect("enhanced summon should succeed");

        assert_eq!(result.status_inflicts.len(), 1);
        assert_eq!(result.status_inflicts[0].0, "burn");
        assert_eq!(result.status_inflicts[0].1, 3);

        // Simulate applying status to 10 enemies with a 75% chance each,
        // using the same seeded RNG approach the resolution system uses.
        const SUMMON_STATUS_CHANCE: f32 = 0.75;
        let mut rng = StdRng::seed_from_u64(42);
        let mut enemies: Vec<BattleUnit> = (0..10)
            .map(|i| {
                let mut unit = make_battle_unit(100 + i, "Goblin", 3, 50, 10, 0);
                unit.side = UnitSide::Enemy;
                unit
            })
            .collect();

        let mut applied_count = 0;
        let mut skipped_count = 0;
        for enemy in enemies.iter_mut() {
            if rng.r#gen::<f32>() < SUMMON_STATUS_CHANCE {
                let battle_status = BattleStatusEffect::Burn { duration: 3 };
                let applied = status::apply_status_to_unit(enemy, battle_status);
                if applied {
                    applied_count += 1;
                }
            } else {
                skipped_count += 1;
            }
        }

        // With 75% chance and 10 enemies, we expect some to be hit and some to be skipped.
        // With seed 42, the exact distribution is deterministic.
        assert!(
            applied_count > 0,
            "at least some enemies should have status applied"
        );
        assert!(
            skipped_count > 0,
            "at least some enemies should have been skipped by the chance roll"
        );
        assert_eq!(applied_count + skipped_count, 10);

        // Verify that enemies who got the status actually have it.
        let enemies_with_burn = enemies
            .iter()
            .filter(|e| {
                e.status_effects
                    .iter()
                    .any(|s| matches!(s, BattleStatusEffect::Burn { .. }))
            })
            .count();
        assert_eq!(
            enemies_with_burn, applied_count,
            "burn count on units should match applied count"
        );
    }
}
