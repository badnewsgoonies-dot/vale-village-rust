//! Bevy ECS systems for the battle flow.
//!
//! These systems drive the battle loop:
//! CommandSelect -> AiSelect -> Resolution -> Victory/Defeat
//!
//! Pure logic lives in sibling modules (damage, status, turn_order, ai, rewards, djinn).
//! Systems here handle ECS queries, events, state transitions, and input.

use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::battle::{
    ai, damage, djinn, rewards, status, turn_order,
    types::*,
};

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Seeded RNG resource for deterministic battles (save-replay friendly).
#[derive(Resource)]
pub struct BattleRng(pub StdRng);

impl Default for BattleRng {
    fn default() -> Self {
        Self(StdRng::from_entropy())
    }
}

// ---------------------------------------------------------------------------
// Battle transition: enter / exit
// ---------------------------------------------------------------------------

/// System that initializes battle when entering `BattlePhase::CommandSelect`.
///
/// Reads [`StartBattleEvent`], spawns [`BattleUnit`] entities, computes initial
/// turn order, and sets up [`BattleState`] + [`CommandSelectState`].
pub fn battle_enter_system(
    mut commands: Commands,
    mut start_events: EventReader<StartBattleEvent>,
    mut battle_state: ResMut<BattleState>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut rng: ResMut<BattleRng>,
    party_query: Query<&BattleUnit, With<BattleUnit>>,
) {
    for event in start_events.read() {
        // Reset state
        *battle_state = BattleState {
            turn_number: 1,
            encounter_id: event.encounter_id.clone(),
            ..Default::default()
        };

        // Spawn enemy entities
        for enemy in &event.enemy_units {
            commands.spawn(enemy.clone());
        }

        // Collect all units for turn order
        let all_units: Vec<BattleUnit> = party_query
            .iter()
            .cloned()
            .chain(event.enemy_units.iter().cloned())
            .collect();

        battle_state.turn_order = turn_order::calculate_turn_order(&all_units, &mut rng.0);

        // Initialize command select for player units
        let player_count = all_units
            .iter()
            .filter(|u| u.side == UnitSide::Player && u.is_alive())
            .count();
        *cmd_state = CommandSelectState {
            menu: CommandMenu::TopLevel,
            cursor_index: 0,
            pending_actions: vec![None; player_count],
            selected_ability: None,
            selected_djinn: None,
        };
    }
}

/// System that cleans up battle entities when exiting battle.
pub fn battle_exit_system(
    mut commands: Commands,
    enemy_query: Query<(Entity, &BattleUnit)>,
) {
    for (entity, unit) in enemy_query.iter() {
        if unit.side == UnitSide::Enemy {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Command Select phase (player input)
// ---------------------------------------------------------------------------

/// System that handles player input during command selection.
///
/// Controls:
/// - Up/Down arrows: navigate menu
/// - Enter: confirm selection
/// - Escape: go back
pub fn command_select_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut battle_state: ResMut<BattleState>,
    mut next_phase: ResMut<NextState<BattlePhase>>,
    units: Query<&BattleUnit>,
) {
    let player_units: Vec<&BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player && u.is_alive())
        .collect();

    if player_units.is_empty() {
        return;
    }

    let current_idx = cmd_state.selecting_unit_index;
    if current_idx >= player_units.len() {
        // All player units have selected actions — move to AI phase
        next_phase.set(BattlePhase::AiSelect);
        return;
    }

    match cmd_state.menu {
        CommandMenu::TopLevel => {
            handle_top_level_input(&keyboard, &mut cmd_state, &player_units, &units);
        }
        CommandMenu::AbilitySelect => {
            handle_ability_select_input(&keyboard, &mut cmd_state, &player_units);
        }
        CommandMenu::TargetSelect => {
            handle_target_select_input(
                &keyboard,
                &mut cmd_state,
                &mut battle_state,
                &player_units,
                &units,
                &mut next_phase,
            );
        }
        CommandMenu::ItemSelect => {
            // Stub: items not yet implemented, go back
            if keyboard.just_pressed(KeyCode::Escape) {
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
            }
        }
        CommandMenu::DjinnSelect => {
            handle_djinn_select_input(&keyboard, &mut cmd_state, &player_units);
        }
    }
}

fn handle_top_level_input(
    keyboard: &Res<ButtonInput<KeyCode>>,
    cmd_state: &mut ResMut<CommandSelectState>,
    _player_units: &[&BattleUnit],
    _units: &Query<&BattleUnit>,
) {
    // Top level: Fight(0), Ability(1), Djinn(2), Item(3), Defend(4), Flee(5)
    let menu_size = 6;

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        if cmd_state.cursor_index > 0 {
            cmd_state.cursor_index -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        if cmd_state.cursor_index < menu_size - 1 {
            cmd_state.cursor_index += 1;
        }
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        match cmd_state.cursor_index {
            0 => {
                // Fight -> target select
                cmd_state.menu = CommandMenu::TargetSelect;
                cmd_state.cursor_index = 0;
                cmd_state.selected_ability = None; // basic attack
            }
            1 => {
                // Ability select
                cmd_state.menu = CommandMenu::AbilitySelect;
                cmd_state.cursor_index = 0;
            }
            2 => {
                // Djinn select
                cmd_state.menu = CommandMenu::DjinnSelect;
                cmd_state.cursor_index = 0;
            }
            3 => {
                // Item select (stub)
                cmd_state.menu = CommandMenu::ItemSelect;
                cmd_state.cursor_index = 0;
            }
            4 => {
                // Defend
                let idx = cmd_state.selecting_unit_index;
                if idx < cmd_state.pending_actions.len() {
                    cmd_state.pending_actions[idx] = Some(BattleAction::Defend);
                    cmd_state.selecting_unit_index += 1;
                    cmd_state.menu = CommandMenu::TopLevel;
                    cmd_state.cursor_index = 0;
                }
            }
            5 => {
                // Flee
                let idx = cmd_state.selecting_unit_index;
                if idx < cmd_state.pending_actions.len() {
                    cmd_state.pending_actions[idx] = Some(BattleAction::Flee);
                    cmd_state.selecting_unit_index += 1;
                    cmd_state.menu = CommandMenu::TopLevel;
                    cmd_state.cursor_index = 0;
                }
            }
            _ => {}
        }
    }
}

fn handle_ability_select_input(
    keyboard: &Res<ButtonInput<KeyCode>>,
    cmd_state: &mut ResMut<CommandSelectState>,
    player_units: &[&BattleUnit],
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        cmd_state.menu = CommandMenu::TopLevel;
        cmd_state.cursor_index = 0;
        return;
    }

    let unit_idx = cmd_state.selecting_unit_index;
    if unit_idx >= player_units.len() {
        return;
    }

    let unit = player_units[unit_idx];
    let affordable: Vec<&AbilityDef> = unit
        .abilities
        .iter()
        .filter(|a| a.pp_cost <= unit.stats.pp && a.unlock_level <= unit.level)
        .collect();

    if affordable.is_empty() {
        // No affordable abilities, go back
        cmd_state.menu = CommandMenu::TopLevel;
        cmd_state.cursor_index = 0;
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
        cmd_state.cursor_index -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && cmd_state.cursor_index < affordable.len() - 1 {
        cmd_state.cursor_index += 1;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some(ability) = affordable.get(cmd_state.cursor_index) {
            cmd_state.selected_ability = Some(ability.id.clone());
            cmd_state.menu = CommandMenu::TargetSelect;
            cmd_state.cursor_index = 0;
        }
    }
}

fn handle_target_select_input(
    keyboard: &Res<ButtonInput<KeyCode>>,
    cmd_state: &mut ResMut<CommandSelectState>,
    battle_state: &mut ResMut<BattleState>,
    _player_units: &[&BattleUnit],
    units: &Query<&BattleUnit>,
    next_phase: &mut ResMut<NextState<BattlePhase>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        cmd_state.menu = CommandMenu::TopLevel;
        cmd_state.cursor_index = 0;
        cmd_state.selected_ability = None;
        return;
    }

    // Build target list based on ability
    let targets: Vec<&BattleUnit> = if cmd_state.selected_ability.is_some() {
        // For now, all alive enemies (simplified targeting)
        units.iter().filter(|u| u.side == UnitSide::Enemy && u.is_alive()).collect()
    } else {
        // Basic attack: all alive enemies
        units.iter().filter(|u| u.side == UnitSide::Enemy && u.is_alive()).collect()
    };

    if targets.is_empty() {
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
        cmd_state.cursor_index -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && cmd_state.cursor_index < targets.len() - 1 {
        cmd_state.cursor_index += 1;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some(target) = targets.get(cmd_state.cursor_index) {
            let target_id = target.id;
            let unit_idx = cmd_state.selecting_unit_index;

            let action = if let Some(ref ability_id) = cmd_state.selected_ability {
                BattleAction::Ability {
                    ability_id: ability_id.clone(),
                    target_id,
                }
            } else {
                BattleAction::Attack { target_id }
            };

            if unit_idx < cmd_state.pending_actions.len() {
                cmd_state.pending_actions[unit_idx] = Some(action);
                cmd_state.selecting_unit_index += 1;
                cmd_state.menu = CommandMenu::TopLevel;
                cmd_state.cursor_index = 0;
                cmd_state.selected_ability = None;

                // Check if all player units have selected
                if cmd_state.selecting_unit_index >= cmd_state.pending_actions.len() {
                    // Transfer pending actions to battle state
                    let player_units: Vec<&BattleUnit> = units
                        .iter()
                        .filter(|u| u.side == UnitSide::Player && u.is_alive())
                        .collect();

                    for (i, action) in cmd_state.pending_actions.iter().enumerate() {
                        if let (Some(action), Some(unit)) = (action, player_units.get(i)) {
                            battle_state.actions.push((unit.id, action.clone()));
                        }
                    }

                    next_phase.set(BattlePhase::AiSelect);
                }
            }
        }
    }
}

fn handle_djinn_select_input(
    keyboard: &Res<ButtonInput<KeyCode>>,
    cmd_state: &mut ResMut<CommandSelectState>,
    player_units: &[&BattleUnit],
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        cmd_state.menu = CommandMenu::TopLevel;
        cmd_state.cursor_index = 0;
        return;
    }

    let unit_idx = cmd_state.selecting_unit_index;
    if unit_idx >= player_units.len() {
        return;
    }

    let unit = player_units[unit_idx];
    let djinn_count = unit.djinn_ids.len();

    if djinn_count == 0 {
        cmd_state.menu = CommandMenu::TopLevel;
        cmd_state.cursor_index = 0;
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) && cmd_state.cursor_index > 0 {
        cmd_state.cursor_index -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && cmd_state.cursor_index < djinn_count - 1 {
        cmd_state.cursor_index += 1;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some(djinn_id) = unit.djinn_ids.get(cmd_state.cursor_index) {
            cmd_state.selected_djinn = Some(djinn_id.clone());
            // Go to target select for Djinn unleash
            cmd_state.menu = CommandMenu::TargetSelect;
            cmd_state.cursor_index = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// AI Select phase
// ---------------------------------------------------------------------------

/// System that selects actions for all enemy units.
pub fn ai_select_system(
    mut battle_state: ResMut<BattleState>,
    mut rng: ResMut<BattleRng>,
    mut next_phase: ResMut<NextState<BattlePhase>>,
    units: Query<&BattleUnit>,
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
        let action = ai::enemy_choose_action(enemy, &enemies, &players, &mut rng.0);
        battle_state.actions.push((enemy.id, action));
    }

    // Transition to resolution
    next_phase.set(BattlePhase::Resolution);
}

// ---------------------------------------------------------------------------
// Resolution phase
// ---------------------------------------------------------------------------

/// System that executes all queued actions in turn order.
///
/// Processes one action per frame for visual pacing. When all actions
/// are resolved, transitions to victory check.
pub fn resolution_system(
    mut battle_state: ResMut<BattleState>,
    mut rng: ResMut<BattleRng>,
    mut next_phase: ResMut<NextState<BattlePhase>>,
    mut units: Query<&mut BattleUnit>,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
    mut ko_events: EventWriter<UnitKoEvent>,
    mut djinn_state: ResMut<DjinnBattleState>,
) {
    let idx = battle_state.current_actor_index;

    if idx >= battle_state.turn_order.len() {
        // All actions resolved — advance turn
        battle_state.turn_number += 1;
        battle_state.current_actor_index = 0;
        battle_state.actions.clear();

        // Tick Djinn recovery
        let _recovered = djinn::tick_djinn_recovery(&mut djinn_state);

        // Recalculate turn order for next round
        let all_units: Vec<BattleUnit> = units.iter().cloned().collect();
        battle_state.turn_order = turn_order::calculate_turn_order(&all_units, &mut rng.0);

        // Check for victory/defeat before starting next command phase
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

    // Find the action for this actor
    let action = battle_state
        .actions
        .iter()
        .find(|(id, _)| *id == actor_id)
        .map(|(_, a)| a.clone());

    let action = match action {
        Some(a) => a,
        None => return, // No action queued (shouldn't happen)
    };

    // Check if actor is still alive
    let actor_alive = units.iter().any(|u| u.id == actor_id && u.is_alive());
    if !actor_alive {
        return; // Skip dead units
    }

    // Status tick for the actor
    {
        if let Some(mut actor) = units.iter_mut().find(|u| u.id == actor_id) {
            let _tick_result = status::tick_status_effects(&mut actor, &mut rng.0);

            // Check if actor died from DOT
            if actor.is_ko() {
                ko_events.send(UnitKoEvent {
                    unit_id: actor.id,
                    unit_name: actor.name.clone(),
                    side: actor.side,
                });
                return;
            }

            // Check freeze/stun
            if status::is_frozen_or_stunned(&actor) {
                return; // Skip action
            }

            // Check paralyze
            if status::check_paralyze_failure(&actor, &mut rng.0) {
                return; // Action fails
            }
        }
    }

    // Execute the action
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
            );
        }
        BattleAction::Defend => {
            // Defend is handled passively during damage calculation
        }
        BattleAction::Flee => {
            // Flee attempt
            let avg_player_spd = {
                let player_speeds: Vec<f32> = units
                    .iter()
                    .filter(|u| u.side == UnitSide::Player && u.is_alive())
                    .map(|u| u.stats.spd as f32)
                    .collect();
                if player_speeds.is_empty() {
                    0.0
                } else {
                    player_speeds.iter().sum::<f32>() / player_speeds.len() as f32
                }
            };
            let avg_enemy_spd = {
                let enemy_speeds: Vec<f32> = units
                    .iter()
                    .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
                    .map(|u| u.stats.spd as f32)
                    .collect();
                if enemy_speeds.is_empty() {
                    0.0
                } else {
                    enemy_speeds.iter().sum::<f32>() / enemy_speeds.len() as f32
                }
            };
            let chance = rewards::flee_chance(avg_player_spd, avg_enemy_spd);
            if rng.0.gen::<f32>() < chance {
                battle_state.fled = true;
                next_phase.set(BattlePhase::Inactive);
                return;
            }
            // Flee failed — turn wasted
        }
        BattleAction::DjinnUnleash {
            djinn_id,
            target_id,
        } => {
            // Unleash: move to Standby, then apply Djinn ability
            let _ = djinn::unleash_djinn(&djinn_id, battle_state.turn_number, &mut djinn_state);
            // Djinn abilities would be looked up from data; for now treat as physical attack
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
            // Summon: move Djinn to Recovery, deal AoE damage
            if let Ok(summon_damage) =
                djinn::summon_djinn(&djinn_ids, battle_state.turn_number, &mut djinn_state)
            {
                // Apply to all enemies
                let enemy_ids: Vec<u32> = units
                    .iter()
                    .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
                    .map(|u| u.id)
                    .collect();

                for enemy_id in enemy_ids {
                    if let Some(mut target) = units.iter_mut().find(|u| u.id == enemy_id) {
                        let result =
                            damage::apply_damage_with_shields(&mut target, summon_damage);
                        damage_events.send(DamageEvent {
                            attacker_id: actor_id,
                            target_id: enemy_id,
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
        }
        BattleAction::Item { .. } => {
            // Item usage stub — not yet implemented
        }
    }
}

fn execute_basic_attack(
    attacker_id: u32,
    target_id: u32,
    units: &mut Query<&mut BattleUnit>,
    rng: &mut ResMut<BattleRng>,
    damage_events: &mut EventWriter<DamageEvent>,
    ko_events: &mut EventWriter<UnitKoEvent>,
    battle_state: &ResMut<BattleState>,
) {
    // Get attacker stats (immutable read)
    let attacker_data = units
        .iter()
        .find(|u| u.id == attacker_id)
        .cloned();
    let attacker = match attacker_data {
        Some(a) => a,
        None => return,
    };

    // Check if target is defending
    let target_defending = battle_state
        .actions
        .iter()
        .any(|(id, action)| *id == target_id && matches!(action, BattleAction::Defend));

    // Create a basic attack ability
    let basic_attack = AbilityDef {
        id: "basic_attack".into(),
        name: "Attack".into(),
        kind: AbilityKind::Physical,
        base_power: 0, // uses ATK as base
        ..Default::default()
    };

    // Get target
    if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
        let dmg =
            damage::calculate_physical_damage(&attacker, &target, &basic_attack, target_defending, &mut rng.0);

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

fn execute_ability(
    caster_id: u32,
    ability_id: &str,
    target_id: u32,
    units: &mut Query<&mut BattleUnit>,
    rng: &mut ResMut<BattleRng>,
    damage_events: &mut EventWriter<DamageEvent>,
    heal_events: &mut EventWriter<HealEvent>,
    ko_events: &mut EventWriter<UnitKoEvent>,
    battle_state: &ResMut<BattleState>,
) {
    // Get caster data
    let caster_data = units.iter().find(|u| u.id == caster_id).cloned();
    let caster = match caster_data {
        Some(c) => c,
        None => return,
    };

    // Find ability
    let ability = caster.abilities.iter().find(|a| a.id == ability_id).cloned();
    let ability = match ability {
        Some(a) => a,
        None => return,
    };

    // Deduct PP
    if let Some(mut caster_unit) = units.iter_mut().find(|u| u.id == caster_id) {
        caster_unit.stats.pp = (caster_unit.stats.pp - ability.pp_cost).max(0);
    }

    match ability.kind {
        AbilityKind::Healing => {
            let heal_amount = damage::calculate_heal_amount(&caster, &ability);

            // Resolve targets
            match ability.targets {
                TargetKind::SingleAlly | TargetKind::OneSelf => {
                    if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
                        let was_ko = target.is_ko();
                        damage::apply_healing(&mut target, heal_amount, ability.revives_fallen);
                        let revived = was_ko && target.is_alive();
                        heal_events.send(HealEvent {
                            source_id: caster_id,
                            target_id,
                            amount: heal_amount,
                            revived,
                        });
                    }
                }
                TargetKind::AllAllies => {
                    let ally_side = caster.side;
                    let ally_ids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == ally_side)
                        .map(|u| u.id)
                        .collect();
                    for ally_id in ally_ids {
                        if let Some(mut ally) = units.iter_mut().find(|u| u.id == ally_id) {
                            let was_ko = ally.is_ko();
                            damage::apply_healing(&mut ally, heal_amount, ability.revives_fallen);
                            let revived = was_ko && ally.is_alive();
                            heal_events.send(HealEvent {
                                source_id: caster_id,
                                target_id: ally_id,
                                amount: heal_amount,
                                revived,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        AbilityKind::Physical | AbilityKind::Psynergy | AbilityKind::Debuff | AbilityKind::StatusInflict => {
            let target_defending = battle_state
                .actions
                .iter()
                .any(|(id, action)| *id == target_id && matches!(action, BattleAction::Defend));

            match ability.targets {
                TargetKind::SingleEnemy | TargetKind::SingleAlly | TargetKind::OneSelf => {
                    if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
                        let dmg = damage::calculate_damage(
                            &caster,
                            &target,
                            &ability,
                            target_defending,
                            &mut rng.0,
                        );
                        let result = damage::apply_damage_with_shields(&mut target, dmg);

                        damage_events.send(DamageEvent {
                            attacker_id: caster_id,
                            target_id,
                            damage: result.actual_damage,
                            element: ability.element,
                            was_blocked: result.was_blocked,
                        });

                        // Apply status effect if any
                        if let Some(ref status_effect) = ability.inflicts_status {
                            status::apply_status_to_unit(&mut target, status_effect.clone());
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
                TargetKind::AllEnemies => {
                    let enemy_side = if caster.side == UnitSide::Player {
                        UnitSide::Enemy
                    } else {
                        UnitSide::Player
                    };
                    let enemy_ids: Vec<u32> = units
                        .iter()
                        .filter(|u| u.side == enemy_side && u.is_alive())
                        .map(|u| u.id)
                        .collect();

                    for eid in enemy_ids {
                        let defending = battle_state
                            .actions
                            .iter()
                            .any(|(id, action)| *id == eid && matches!(action, BattleAction::Defend));

                        if let Some(mut target) = units.iter_mut().find(|u| u.id == eid) {
                            let dmg = damage::calculate_damage(
                                &caster,
                                &target,
                                &ability,
                                defending,
                                &mut rng.0,
                            );
                            let result = damage::apply_damage_with_shields(&mut target, dmg);

                            damage_events.send(DamageEvent {
                                attacker_id: caster_id,
                                target_id: eid,
                                damage: result.actual_damage,
                                element: ability.element,
                                was_blocked: result.was_blocked,
                            });

                            if let Some(ref status_effect) = ability.inflicts_status {
                                status::apply_status_to_unit(
                                    &mut target,
                                    status_effect.clone(),
                                );
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
                    // Buff/debuff all allies — delegated to status
                    if let Some(ref status_effect) = ability.inflicts_status {
                        let ally_ids: Vec<u32> = units
                            .iter()
                            .filter(|u| u.side == caster.side && u.is_alive())
                            .map(|u| u.id)
                            .collect();
                        for aid in ally_ids {
                            if let Some(mut ally) = units.iter_mut().find(|u| u.id == aid) {
                                status::apply_status_to_unit(&mut ally, status_effect.clone());
                            }
                        }
                    }
                }
            }
        }

        AbilityKind::Buff => {
            // Apply buff status
            if let Some(ref status_effect) = ability.inflicts_status {
                match ability.targets {
                    TargetKind::OneSelf | TargetKind::SingleAlly => {
                        if let Some(mut target) = units.iter_mut().find(|u| u.id == target_id) {
                            status::apply_status_to_unit(&mut target, status_effect.clone());
                        }
                    }
                    TargetKind::AllAllies => {
                        let ally_ids: Vec<u32> = units
                            .iter()
                            .filter(|u| u.side == caster.side && u.is_alive())
                            .map(|u| u.id)
                            .collect();
                        for aid in ally_ids {
                            if let Some(mut ally) = units.iter_mut().find(|u| u.id == aid) {
                                status::apply_status_to_unit(&mut ally, status_effect.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Victory / Defeat
// ---------------------------------------------------------------------------

/// Victory check happens at end of resolution (in resolution_system).
/// This system handles the victory screen logic.
pub fn victory_system(
    mut battle_state: ResMut<BattleState>,
    mut units: Query<&mut BattleUnit>,
    mut end_events: EventWriter<EndBattleEvent>,
) {
    let enemies: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Enemy)
        .cloned()
        .collect();

    let mut party: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player)
        .cloned()
        .collect();

    let party_size = party.len() as u32;
    let survivor_count = party.iter().filter(|u| u.is_alive()).count() as u32;

    let battle_rewards = rewards::calculate_battle_rewards(&enemies, party_size, survivor_count);
    let level_ups = rewards::distribute_rewards(&mut party, &battle_rewards);

    // Apply updated stats back to entities
    for updated in &party {
        if let Some(mut unit) = units.iter_mut().find(|u| u.id == updated.id) {
            unit.stats = updated.stats;
            unit.level = updated.level;
            unit.xp = updated.xp;
        }
    }

    end_events.send(EndBattleEvent {
        victory: true,
        rewards: Some(battle_rewards),
        level_ups,
    });
}

/// Defeat system — game over handling.
pub fn defeat_system(mut end_events: EventWriter<EndBattleEvent>) {
    end_events.send(EndBattleEvent {
        victory: false,
        rewards: None,
        level_ups: vec![],
    });
}
