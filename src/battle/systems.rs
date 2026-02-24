//! Bevy ECS systems for the battle flow.
//!
//! Systems drive the battle loop: CommandSelect -> AiSelect -> Resolution -> Victory/Defeat.
//! Pure logic lives in sibling modules; systems bridge ECS queries to pure functions.

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::battle::{ai, damage, djinn, rewards, status, turn_order, types::*};
use crate::plugins::core_plugin::{GameData, Party};

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
// Battle enter / exit
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn battle_enter_system(
    mut commands: Commands,
    mut start_events: EventReader<StartBattleEvent>,
    mut battle_state: ResMut<BattleStateRes>,
    mut cmd_state: ResMut<CommandSelectState>,
    mut rng: ResMut<BattleRng>,
    party: Res<Party>,
    game_data: Res<GameData>,
    existing_party_units: Query<(Entity, &BattleUnit)>,
) {
    for event in start_events.read() {
        *battle_state = BattleStateRes {
            turn_number: 1,
            encounter_id: event.encounter_id.clone(),
            ..Default::default()
        };

        // Despawn any leftover party battle units from previous battles
        for (entity, unit) in existing_party_units.iter() {
            if unit.side == UnitSide::Player {
                commands.entity(entity).despawn();
            }
        }

        // Spawn enemy units
        for enemy in &event.enemy_units {
            commands.spawn(enemy.clone());
        }

        // Spawn party units from Party resource + GameData
        let mut party_units = Vec::new();
        for (idx, unit_id) in party.active.iter().enumerate() {
            if let Some(def) = game_data.units.get(unit_id) {
                let battle_unit = BattleUnit {
                    id: idx as u32 + 1,
                    name: def.name.clone(),
                    side: UnitSide::Player,
                    element: def.element,
                    level: 1,
                    hp: def.base_hp,
                    max_hp: def.base_hp,
                    pp: def.base_pp,
                    max_pp: def.base_pp,
                    atk: def.base_atk,
                    def: def.base_def,
                    mag: def.base_mag,
                    spd: def.base_spd,
                    luck: 5,
                    status_effects: Vec::new(),
                    ability_ids: def
                        .abilities
                        .iter()
                        .filter(|a| a.unlock_level <= 1)
                        .map(|a| a.ability_id.clone())
                        .collect(),
                    djinn_ids: Vec::new(),
                    damage_taken: 0,
                    damage_dealt: 0,
                    xp: 0,
                    growth_rates: GrowthRates {
                        hp: def.growth.hp,
                        pp: def.growth.pp,
                        atk: def.growth.atk,
                        def: def.growth.def,
                        mag: def.growth.mag,
                        spd: def.growth.spd,
                    },
                };
                commands.spawn(battle_unit.clone());
                party_units.push(battle_unit);
            }
        }

        let all_units: Vec<BattleUnit> = party_units
            .iter()
            .cloned()
            .chain(event.enemy_units.iter().cloned())
            .collect();

        battle_state.turn_order = turn_order::calculate_turn_order(&all_units, &mut rng.0);

        let player_count = party_units.iter().filter(|u| u.is_alive()).count();
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
                let action = if let Some(ref djinn_id) = cmd_state.selected_djinn {
                    BattleAction::DjinnUnleash {
                        djinn_id: djinn_id.clone(),
                        target_id: target.id,
                    }
                } else if let Some(ref aid) = cmd_state.selected_ability {
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
                cmd_state.selected_djinn = None;
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
            }
            // Item selection is handled by the battle UI layer
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
    mut djinn_state: ResMut<DjinnBattleRes>,
    mut end_events: EventWriter<EndBattleEvent>,
    game_data: Res<GameData>,
) {
    // Check for flee
    if battle_state.fled {
        end_events.send(EndBattleEvent {
            victory: false,
            rewards: None,
            level_ups: vec![],
        });
        return;
    }

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
            if let Ok(summon_damage) =
                djinn::summon_djinn(&djinn_ids, battle_state.turn_number, &mut djinn_state)
            {
                let enemy_ids: Vec<u32> = units
                    .iter()
                    .filter(|u| u.side == UnitSide::Enemy && u.is_alive())
                    .map(|u| u.id)
                    .collect();
                for eid in enemy_ids {
                    if let Some(mut target) = units.iter_mut().find(|u| u.id == eid) {
                        let result = damage::apply_damage_with_shields(&mut target, summon_damage);
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
        }
        BattleAction::Item {
            item_id, target_id, ..
        } => {
            execute_item(
                actor_id,
                &item_id,
                target_id,
                &mut units,
                &mut heal_events,
                &mut damage_events,
                &mut ko_events,
                &game_data,
            );
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
        let dmg = damage::calculate_physical_damage(
            &attacker,
            &target,
            &basic_attack,
            target_defending,
            &mut rng.0,
        );
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
                            let dmg = damage::calculate_damage(
                                &caster, &target, &ability, defending, &mut rng.0,
                            );
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

/// Execute an item usage in battle.
#[allow(clippy::too_many_arguments)]
fn execute_item(
    user_id: u32,
    item_id: &str,
    target_id: u32,
    units: &mut Query<&mut BattleUnit>,
    heal_events: &mut EventWriter<HealEvent>,
    damage_events: &mut EventWriter<DamageEvent>,
    ko_events: &mut EventWriter<UnitKoEvent>,
    game_data: &Res<GameData>,
) {
    let item = match game_data.items.get(item_id) {
        Some(i) => i.clone(),
        None => return,
    };

    let effect = &item.effect;

    // Healing / Revive
    if (effect.hp_restore > 0 || effect.pp_restore > 0 || effect.revive)
        && let Some(mut target) = units.iter_mut().find(|u| u.id == target_id)
    {
        let was_ko = target.is_ko();

        // Only allow using on KO targets if item revives
        if was_ko && !effect.revive {
            return;
        }

        if effect.revive && was_ko {
            target.hp = 1; // Bring back to life first
        }

        if effect.hp_restore > 0 {
            let heal = effect.hp_restore.min(target.max_hp - target.hp);
            target.hp = (target.hp + effect.hp_restore).min(target.max_hp);
            heal_events.send(HealEvent {
                source_id: user_id,
                target_id,
                amount: heal,
                revived: was_ko && target.is_alive(),
            });
        }

        if effect.pp_restore > 0 {
            target.pp = (target.pp + effect.pp_restore).min(target.max_pp);
        }
    }

    // Status removal
    if !effect.removes_status.is_empty()
        && let Some(mut target) = units.iter_mut().find(|u| u.id == target_id)
    {
        for status_name in &effect.removes_status {
            target.status_effects.retain(|se| {
                let kind_name = format!("{:?}", se.kind()).to_lowercase();
                kind_name != *status_name
            });
        }
    }

    // Damage items
    if effect.damage_amount > 0
        && let Some(mut target) = units.iter_mut().find(|u| u.id == target_id)
    {
        let result = damage::apply_damage_with_shields(&mut target, effect.damage_amount);
        damage_events.send(DamageEvent {
            attacker_id: user_id,
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

pub fn victory_system(
    mut units: Query<&mut BattleUnit>,
    mut end_events: EventWriter<EndBattleEvent>,
    game_data: Res<GameData>,
) {
    let enemy_xp_gold: Vec<(u32, u32)> = units
        .iter()
        .filter(|u| u.side == UnitSide::Enemy)
        .filter_map(|u| {
            // Look up base_xp and base_gold from enemy definitions by name
            game_data
                .enemies
                .values()
                .find(|e| e.name == u.name)
                .map(|e| (e.base_xp, e.base_gold))
        })
        .collect();

    let mut party: Vec<BattleUnit> = units
        .iter()
        .filter(|u| u.side == UnitSide::Player)
        .cloned()
        .collect();
    let party_size = party.len() as u32;
    let survivor_count = party.iter().filter(|u| u.is_alive()).count() as u32;

    let battle_rewards =
        rewards::calculate_battle_rewards(&enemy_xp_gold, party_size, survivor_count);
    let level_ups = rewards::distribute_rewards(&mut party, &battle_rewards);

    for updated in &party {
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

    end_events.send(EndBattleEvent {
        victory: true,
        rewards: Some(battle_rewards),
        level_ups,
    });
}

pub fn defeat_system(mut end_events: EventWriter<EndBattleEvent>) {
    end_events.send(EndBattleEvent {
        victory: false,
        rewards: None,
        level_ups: vec![],
    });
}
