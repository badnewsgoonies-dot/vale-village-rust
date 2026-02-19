//! Enemy AI action selection.
//!
//! Pure functions for choosing enemy actions. Uses AI hints from ability data.

use crate::battle::types::{
    AbilityDef, AbilityType, AiTargetPref, BattleAction, BattleUnit, TargetKind,
};
use rand::Rng;
use std::collections::HashMap;

/// Choose an action for an enemy unit.
///
/// Strategy:
/// 1. If HP < 30% and has healing, heal self or weakest ally
/// 2. Pick highest-priority affordable ability
/// 3. Select best target
/// 4. Fallback: basic attack lowest-HP opponent
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
    if hp_pct < 0.30 {
        if let Some(action) = try_heal(enemy, &alive_allies, &abilities) {
            return action;
        }
    }

    // 2. Usable abilities (affordable, have valid targets)
    let mut usable: Vec<&AbilityDef> = abilities
        .iter()
        .filter(|a| a.mana_cost <= enemy.pp)
        .filter(|a| has_valid_targets(a, &alive_targets, &alive_allies))
        .copied()
        .collect();

    usable.sort_by(|a, b| b.ai_hints.priority.partial_cmp(&a.ai_hints.priority).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Weighted pick
    if let Some(ability) = pick_weighted_ability(&usable, rng) {
        let target_id = select_target(ability, &alive_targets, &alive_allies, rng);
        return BattleAction::Ability {
            ability_id: ability.id.clone(),
            target_id,
        };
    }

    // 4. Basic attack
    let target = alive_targets.iter().min_by_key(|u| u.hp).unwrap();
    BattleAction::Attack { target_id: target.id }
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
    let weakest = allies.iter().filter(|u| u.is_alive()).min_by_key(|u| u.hp)?;
    let target_id = if enemy.hp < weakest.hp { enemy.id } else { weakest.id };

    Some(BattleAction::Ability { ability_id: best.id.clone(), target_id })
}

fn has_valid_targets(ability: &AbilityDef, opponents: &[&BattleUnit], allies: &[&BattleUnit]) -> bool {
    match ability.targets {
        TargetKind::SingleEnemy | TargetKind::AllEnemies => !opponents.is_empty(),
        TargetKind::SingleAlly | TargetKind::AllAllies => !allies.is_empty(),
        TargetKind::OneSelf => true,
    }
}

fn pick_weighted_ability<'a>(abilities: &[&'a AbilityDef], rng: &mut impl Rng) -> Option<&'a AbilityDef> {
    if abilities.is_empty() { return None; }

    let weights: Vec<f32> = abilities.iter().map(|a| a.ai_hints.priority.max(0.1)).collect();
    let total: f32 = weights.iter().sum();
    let mut roll = rng.r#gen::<f32>() * total;

    for (i, w) in weights.iter().enumerate() {
        roll -= w;
        if roll <= 0.0 { return Some(abilities[i]); }
    }
    Some(abilities[0])
}

fn select_target(ability: &AbilityDef, opponents: &[&BattleUnit], allies: &[&BattleUnit], rng: &mut impl Rng) -> u32 {
    match ability.targets {
        TargetKind::SingleEnemy => {
            match ability.ai_hints.target {
                AiTargetPref::Weakest => opponents.iter().min_by_key(|u| u.hp).map(|u| u.id).unwrap_or(0),
                AiTargetPref::HighestDef => opponents.iter().max_by_key(|u| u.def).map(|u| u.id).unwrap_or(0),
                AiTargetPref::HealerFirst | AiTargetPref::LowestRes => {
                    // Simplified: target lowest HP
                    opponents.iter().min_by_key(|u| u.hp).map(|u| u.id).unwrap_or(0)
                }
                AiTargetPref::Random => {
                    let idx = rng.gen_range(0..opponents.len());
                    opponents[idx].id
                }
            }
        }
        TargetKind::AllEnemies => opponents.first().map(|u| u.id).unwrap_or(0),
        TargetKind::SingleAlly => {
            if ability.ability_type == AbilityType::Healing {
                allies.iter().filter(|u| u.is_alive()).min_by_key(|u| u.hp).map(|u| u.id).unwrap_or(0)
            } else {
                let idx = rng.gen_range(0..allies.len().max(1));
                allies.get(idx).map(|u| u.id).unwrap_or(0)
            }
        }
        TargetKind::AllAllies => allies.first().map(|u| u.id).unwrap_or(0),
        TargetKind::OneSelf => allies.first().map(|u| u.id).unwrap_or(0),
    }
}
