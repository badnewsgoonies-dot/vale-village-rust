//! Enemy AI action selection.
//!
//! Pure functions for choosing enemy actions. Uses AI hints from ability data.

use crate::battle::types::{
    AbilityDef, AbilityType, AiTargetPref, BattleAction, BattleUnit, StatusKind, TargetKind,
};
use rand::Rng;
use std::collections::HashMap;

/// Choose an action for an enemy unit.
///
/// Strategy:
/// 1. If HP < 30% and has healing, heal self or weakest ally
/// 2. Consider djinn unleash (30% chance when HP > 50%)
/// 3. Pick highest-priority affordable ability (with debuff boost)
/// 4. Select best target (respecting avoid_overkill)
/// 5. Fallback: basic attack lowest-HP opponent
pub fn enemy_choose_action(
    enemy: &BattleUnit,
    allies: &[BattleUnit],
    targets: &[BattleUnit],
    ability_registry: &HashMap<String, AbilityDef>,
    rng: &mut impl Rng,
) -> BattleAction {
    let alive_targets: Vec<&BattleUnit> = targets.iter().filter(|u| u.is_alive()).collect();
    let alive_allies: Vec<&BattleUnit> = allies.iter().filter(|u| u.is_alive()).collect();

    if alive_targets.is_empty() {
        return BattleAction::Defend;
    }

    // Resolve abilities from registry
    let abilities: Vec<&AbilityDef> = enemy
        .ability_ids
        .iter()
        .filter_map(|id| ability_registry.get(id))
        .collect();

    // 1. Emergency healing
    let hp_pct = enemy.hp as f32 / enemy.max_hp.max(1) as f32;
    if hp_pct < 0.30
        && let Some(action) = try_heal(enemy, &alive_allies, &abilities)
    {
        return action;
    }

    // 2. Djinn unleash consideration
    if !enemy.djinn_ids.is_empty() && hp_pct > 0.50 {
        let roll: f32 = rng.r#gen();
        if roll < 0.30 {
            // Pick the first djinn available
            let djinn_id = enemy.djinn_ids[0].clone();
            // Target using Weakest preference
            let target = alive_targets.iter().min_by_key(|u| u.hp).unwrap();
            return BattleAction::DjinnUnleash {
                djinn_id,
                target_id: target.id,
            };
        }
    }

    // 3. Usable abilities (affordable, have valid targets)
    let mut usable: Vec<&AbilityDef> = abilities
        .iter()
        .filter(|a| a.mana_cost <= enemy.pp)
        .filter(|a| has_valid_targets(a, &alive_targets, &alive_allies))
        .copied()
        .collect();

    usable.sort_by(|a, b| {
        let priority_a = effective_priority(a, &alive_targets);
        let priority_b = effective_priority(b, &alive_targets);
        priority_b
            .partial_cmp(&priority_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 4. Weighted pick
    if let Some(ability) = pick_weighted_ability(&usable, &alive_targets, rng) {
        let target_id = select_target(
            ability,
            &alive_targets,
            &alive_allies,
            ability_registry,
            rng,
        );
        return BattleAction::Ability {
            ability_id: ability.id.clone(),
            target_id,
        };
    }

    // 5. Basic attack
    let target = alive_targets.iter().min_by_key(|u| u.hp).unwrap();
    BattleAction::Attack {
        target_id: target.id,
    }
}

/// Compute effective priority for an ability, boosting debuffs that haven't
/// been applied to any target yet.
fn effective_priority(ability: &AbilityDef, targets: &[&BattleUnit]) -> f32 {
    let base = ability.ai_hints.priority;
    if ability.ability_type == AbilityType::Debuff {
        // Check if any target already has the Debuff status kind
        let any_has_debuff = targets.iter().any(|u| {
            u.status_effects
                .iter()
                .any(|s| s.kind() == StatusKind::Debuff)
        });
        if !any_has_debuff {
            return base * 1.5;
        }
    }
    base
}

fn try_heal(
    enemy: &BattleUnit,
    allies: &[&BattleUnit],
    abilities: &[&AbilityDef],
) -> Option<BattleAction> {
    let heals: Vec<&&AbilityDef> = abilities
        .iter()
        .filter(|a| a.ability_type == AbilityType::Healing && a.mana_cost <= enemy.pp)
        .collect();

    let best = heals.iter().max_by_key(|a| a.base_power)?;
    let weakest = allies
        .iter()
        .filter(|u| u.is_alive())
        .min_by_key(|u| u.hp)?;
    let target_id = if enemy.hp < weakest.hp {
        enemy.id
    } else {
        weakest.id
    };

    Some(BattleAction::Ability {
        ability_id: best.id.clone(),
        target_id,
    })
}

fn has_valid_targets(
    ability: &AbilityDef,
    opponents: &[&BattleUnit],
    allies: &[&BattleUnit],
) -> bool {
    match ability.targets {
        TargetKind::SingleEnemy | TargetKind::AllEnemies => !opponents.is_empty(),
        TargetKind::SingleAlly | TargetKind::AllAllies => !allies.is_empty(),
        TargetKind::OneSelf => true,
    }
}

fn pick_weighted_ability<'a>(
    abilities: &[&'a AbilityDef],
    targets: &[&BattleUnit],
    rng: &mut impl Rng,
) -> Option<&'a AbilityDef> {
    if abilities.is_empty() {
        return None;
    }

    let weights: Vec<f32> = abilities
        .iter()
        .map(|a| effective_priority(a, targets).max(0.1))
        .collect();
    let total: f32 = weights.iter().sum();
    let mut roll = rng.r#gen::<f32>() * total;

    for (i, w) in weights.iter().enumerate() {
        roll -= w;
        if roll <= 0.0 {
            return Some(abilities[i]);
        }
    }
    Some(abilities[0])
}

/// Returns true if the unit has any healing ability in the registry.
fn unit_has_healing(unit: &BattleUnit, ability_registry: &HashMap<String, AbilityDef>) -> bool {
    unit.ability_ids.iter().any(|aid| {
        ability_registry
            .get(aid)
            .is_some_and(|a| a.ability_type == AbilityType::Healing)
    })
}

fn select_target(
    ability: &AbilityDef,
    opponents: &[&BattleUnit],
    allies: &[&BattleUnit],
    ability_registry: &HashMap<String, AbilityDef>,
    rng: &mut impl Rng,
) -> u32 {
    match ability.targets {
        TargetKind::SingleEnemy => {
            // Filter for avoid_overkill: skip targets below 20% max HP
            let candidates: Vec<&&BattleUnit> = if ability.ai_hints.avoid_overkill {
                let filtered: Vec<&&BattleUnit> = opponents
                    .iter()
                    .filter(|u| u.hp as f32 / u.max_hp.max(1) as f32 >= 0.20)
                    .collect();
                if filtered.is_empty() {
                    // All targets are nearly dead, fall back to all opponents
                    opponents.iter().collect()
                } else {
                    filtered
                }
            } else {
                opponents.iter().collect()
            };

            match ability.ai_hints.target {
                AiTargetPref::Weakest => candidates
                    .iter()
                    .min_by_key(|u| u.hp)
                    .map(|u| u.id)
                    .unwrap_or(0),
                AiTargetPref::HighestDef => candidates
                    .iter()
                    .max_by_key(|u| u.def)
                    .map(|u| u.id)
                    .unwrap_or(0),
                AiTargetPref::HealerFirst => {
                    // Target opponent that has healing abilities (checked via registry)
                    let healer = candidates
                        .iter()
                        .find(|u| unit_has_healing(u, ability_registry));
                    if let Some(h) = healer {
                        h.id
                    } else {
                        // Fallback: lowest HP
                        candidates
                            .iter()
                            .min_by_key(|u| u.hp)
                            .map(|u| u.id)
                            .unwrap_or(0)
                    }
                }
                AiTargetPref::LowestRes => {
                    // Target opponent with lowest mag (magical resistance proxy)
                    candidates
                        .iter()
                        .min_by_key(|u| u.mag)
                        .map(|u| u.id)
                        .unwrap_or(0)
                }
                AiTargetPref::Random => {
                    let idx = rng.gen_range(0..candidates.len());
                    candidates[idx].id
                }
            }
        }
        TargetKind::AllEnemies => opponents.first().map(|u| u.id).unwrap_or(0),
        TargetKind::SingleAlly => {
            if ability.ability_type == AbilityType::Healing {
                allies
                    .iter()
                    .filter(|u| u.is_alive())
                    .min_by_key(|u| u.hp)
                    .map(|u| u.id)
                    .unwrap_or(0)
            } else {
                let idx = rng.gen_range(0..allies.len().max(1));
                allies.get(idx).map(|u| u.id).unwrap_or(0)
            }
        }
        TargetKind::AllAllies => allies.first().map(|u| u.id).unwrap_or(0),
        TargetKind::OneSelf => allies.first().map(|u| u.id).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::types::{GrowthRates, UnitSide};
    use crate::components::stats::Element;
    use crate::data::abilities::AiHints;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_unit(id: u32, side: UnitSide, hp: i32, max_hp: i32) -> BattleUnit {
        BattleUnit {
            id,
            name: format!("Unit{}", id),
            side,
            element: Element::Venus,
            level: 5,
            hp,
            max_hp,
            pp: 50,
            max_pp: 50,
            atk: 10,
            def: 10,
            mag: 10,
            spd: 10,
            luck: 5,
            status_effects: vec![],
            ability_ids: vec![],
            djinn_ids: vec![],
            damage_taken: 0,
            damage_dealt: 0,
            xp: 0,
            growth_rates: GrowthRates::default(),
        }
    }

    fn make_ability(id: &str, ability_type: AbilityType, targets: TargetKind) -> AbilityDef {
        AbilityDef {
            id: id.into(),
            name: id.into(),
            ability_type,
            element: None,
            mana_cost: 2,
            base_power: 30,
            targets,
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
                priority: 2.0,
                target: AiTargetPref::Weakest,
                avoid_overkill: false,
                opener: false,
            },
        }
    }

    #[test]
    fn test_basic_attack_fallback() {
        let mut rng = StdRng::seed_from_u64(42);
        let enemy = make_unit(1, UnitSide::Enemy, 100, 100);
        let targets = vec![
            make_unit(10, UnitSide::Player, 50, 100),
            make_unit(11, UnitSide::Player, 80, 100),
        ];
        let registry: HashMap<String, AbilityDef> = HashMap::new();

        let action = enemy_choose_action(
            &enemy,
            std::slice::from_ref(&enemy),
            &targets,
            &registry,
            &mut rng,
        );

        match action {
            BattleAction::Attack { target_id } => {
                // Should target the lowest HP unit (id=10, hp=50)
                assert_eq!(target_id, 10);
            }
            _ => panic!("Expected basic Attack fallback, got {:?}", action),
        }
    }

    #[test]
    fn test_emergency_heal() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut enemy = make_unit(1, UnitSide::Enemy, 20, 100); // 20% HP
        enemy.ability_ids = vec!["heal".into()];

        let heal = make_ability("heal", AbilityType::Healing, TargetKind::SingleAlly);
        let mut registry = HashMap::new();
        registry.insert("heal".into(), heal);

        let targets = vec![make_unit(10, UnitSide::Player, 80, 100)];
        let allies = vec![enemy.clone()];

        let action = enemy_choose_action(&enemy, &allies, &targets, &registry, &mut rng);

        match action {
            BattleAction::Ability {
                ability_id,
                target_id,
            } => {
                assert_eq!(ability_id, "heal");
                // Enemy is the weakest ally (20 HP), should heal self
                assert_eq!(target_id, 1);
            }
            _ => panic!("Expected healing Ability, got {:?}", action),
        }
    }

    #[test]
    fn test_healer_first_targeting() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut enemy = make_unit(1, UnitSide::Enemy, 100, 100);
        enemy.ability_ids = vec!["attack_ability".into()];

        let mut attack_ability = make_ability(
            "attack_ability",
            AbilityType::Psynergy,
            TargetKind::SingleEnemy,
        );
        attack_ability.ai_hints.target = AiTargetPref::HealerFirst;

        // Also register the heal ability so the registry lookup works
        let heal = make_ability("heal", AbilityType::Healing, TargetKind::SingleAlly);

        let mut registry = HashMap::new();
        registry.insert("attack_ability".into(), attack_ability);
        registry.insert("heal".into(), heal);

        // Target 10 has no healing, target 11 has a heal ability
        let target_no_heal = make_unit(10, UnitSide::Player, 80, 100);
        let mut target_healer = make_unit(11, UnitSide::Player, 90, 100);
        target_healer.ability_ids = vec!["heal".into()];
        let targets = vec![target_no_heal, target_healer];

        let action = enemy_choose_action(&enemy, &[enemy.clone()], &targets, &registry, &mut rng);

        match action {
            BattleAction::Ability { target_id, .. } => {
                // Should target the healer (id=11)
                assert_eq!(target_id, 11);
            }
            _ => panic!("Expected Ability targeting healer, got {:?}", action),
        }
    }

    #[test]
    fn test_lowest_res_targeting() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut enemy = make_unit(1, UnitSide::Enemy, 100, 100);
        enemy.ability_ids = vec!["magic_attack".into()];

        let mut magic_attack = make_ability(
            "magic_attack",
            AbilityType::Psynergy,
            TargetKind::SingleEnemy,
        );
        magic_attack.ai_hints.target = AiTargetPref::LowestRes;

        let mut registry = HashMap::new();
        registry.insert("magic_attack".into(), magic_attack);

        let mut target_high_mag = make_unit(10, UnitSide::Player, 80, 100);
        target_high_mag.mag = 30;
        let mut target_low_mag = make_unit(11, UnitSide::Player, 80, 100);
        target_low_mag.mag = 5;
        let targets = vec![target_high_mag, target_low_mag];

        let action = enemy_choose_action(&enemy, &[enemy.clone()], &targets, &registry, &mut rng);

        match action {
            BattleAction::Ability { target_id, .. } => {
                // Should target the unit with lowest mag (id=11, mag=5)
                assert_eq!(target_id, 11);
            }
            _ => panic!("Expected Ability targeting lowest res, got {:?}", action),
        }
    }

    #[test]
    fn test_avoid_overkill() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut enemy = make_unit(1, UnitSide::Enemy, 100, 100);
        enemy.ability_ids = vec!["big_hit".into()];

        let mut big_hit = make_ability("big_hit", AbilityType::Physical, TargetKind::SingleEnemy);
        big_hit.ai_hints.avoid_overkill = true;
        big_hit.ai_hints.target = AiTargetPref::Weakest;

        let mut registry = HashMap::new();
        registry.insert("big_hit".into(), big_hit);

        // Target 10: nearly dead (10/100 = 10% < 20%), should be skipped
        // Target 11: healthy (80/100 = 80%), should be chosen
        let target_nearly_dead = make_unit(10, UnitSide::Player, 10, 100);
        let target_healthy = make_unit(11, UnitSide::Player, 80, 100);
        let targets = vec![target_nearly_dead, target_healthy];

        let action = enemy_choose_action(&enemy, &[enemy.clone()], &targets, &registry, &mut rng);

        match action {
            BattleAction::Ability { target_id, .. } => {
                // Should skip the nearly dead target (id=10) and target id=11
                assert_eq!(target_id, 11);
            }
            _ => panic!("Expected Ability with avoid_overkill, got {:?}", action),
        }
    }
}
