//! Status effect processing algorithms.
//!
//! Ported from TypeScript `status.ts`. Pure functions.

use crate::battle::types::{constants, BattleStatusEffect, BattleUnit, StatusKind};
use rand::Rng;

/// Result of processing status ticks for a single unit.
#[derive(Debug, Clone)]
pub struct StatusTickResult {
    #[allow(dead_code)]
    pub damage: i32,
    #[allow(dead_code)]
    pub healing: i32,
    #[allow(dead_code)]
    pub messages: Vec<String>,
}

/// Process all status effects at the start of a unit's turn.
pub fn tick_status_effects(unit: &mut BattleUnit, rng: &mut impl Rng) -> StatusTickResult {
    let mut total_damage: i32 = 0;
    let mut total_healing: i32 = 0;
    let mut messages: Vec<String> = Vec::new();
    let max_hp = unit.max_hp;

    let updated: Vec<BattleStatusEffect> = unit
        .status_effects
        .iter()
        .filter_map(|effect| {
            match effect {
                BattleStatusEffect::Poison { duration } if *duration <= 0 => None,
                BattleStatusEffect::Burn { duration } if *duration <= 0 => None,
                BattleStatusEffect::HealOverTime { duration, .. } if *duration <= 0 => None,

                BattleStatusEffect::Poison { duration } => {
                    let dmg = (max_hp as f32 * constants::POISON_PERCENT).floor() as i32;
                    total_damage += dmg;
                    messages.push(format!("{} takes {} poison damage!", unit.name, dmg));
                    let new_dur = duration - 1;
                    if new_dur > 0 { Some(BattleStatusEffect::Poison { duration: new_dur }) } else { None }
                }
                BattleStatusEffect::Burn { duration } => {
                    let dmg = (max_hp as f32 * constants::BURN_PERCENT).floor() as i32;
                    total_damage += dmg;
                    messages.push(format!("{} takes {} burn damage!", unit.name, dmg));
                    let new_dur = duration - 1;
                    if new_dur > 0 { Some(BattleStatusEffect::Burn { duration: new_dur }) } else { None }
                }
                BattleStatusEffect::HealOverTime { heal_per_turn, duration } => {
                    total_healing += *heal_per_turn;
                    messages.push(format!("{} recovers {} HP!", unit.name, heal_per_turn));
                    let new_dur = duration - 1;
                    if new_dur > 0 {
                        Some(BattleStatusEffect::HealOverTime { heal_per_turn: *heal_per_turn, duration: new_dur })
                    } else { None }
                }
                BattleStatusEffect::Freeze { duration } => {
                    if *duration <= 0 { return None; }
                    if rng.r#gen::<f32>() < constants::FREEZE_BREAK_CHANCE {
                        messages.push(format!("{} broke free from freeze!", unit.name));
                        None
                    } else {
                        messages.push(format!("{} is frozen and cannot act!", unit.name));
                        Some(BattleStatusEffect::Freeze { duration: duration - 1 })
                    }
                }
                BattleStatusEffect::Stun { duration } => {
                    if *duration <= 0 { return None; }
                    messages.push(format!("{} is stunned and cannot act!", unit.name));
                    Some(BattleStatusEffect::Stun { duration: duration - 1 })
                }
                BattleStatusEffect::Paralyze { duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Paralyze { duration: duration - 1 })
                }
                BattleStatusEffect::Blind { duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Blind { duration: duration - 1 })
                }
                BattleStatusEffect::Buff { stat, modifier, duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Buff { stat: *stat, modifier: *modifier, duration: duration - 1 })
                }
                BattleStatusEffect::Debuff { stat, modifier, duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Debuff { stat: *stat, modifier: *modifier, duration: duration - 1 })
                }
                BattleStatusEffect::Shield { remaining_charges, duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Shield { remaining_charges: *remaining_charges, duration: duration - 1 })
                }
                BattleStatusEffect::Invulnerable { duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Invulnerable { duration: duration - 1 })
                }
                BattleStatusEffect::DamageReduction { percent, duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::DamageReduction { percent: *percent, duration: duration - 1 })
                }
                BattleStatusEffect::AutoRevive { .. } => Some(effect.clone()),
                BattleStatusEffect::Immunity { types, all_negative, duration } => {
                    if *duration <= 0 { return None; }
                    Some(BattleStatusEffect::Immunity { types: types.clone(), all_negative: *all_negative, duration: duration - 1 })
                }
            }
        })
        .collect();

    if total_damage > 0 {
        unit.hp = (unit.hp - total_damage).max(0);
        unit.damage_taken += total_damage;
    }
    if total_healing > 0 {
        unit.hp = (unit.hp + total_healing).min(max_hp);
    }
    unit.status_effects = updated;

    StatusTickResult { damage: total_damage, healing: total_healing, messages }
}

/// Returns true if the unit is frozen or stunned.
pub fn is_frozen_or_stunned(unit: &BattleUnit) -> bool {
    unit.status_effects.iter().any(|s| {
        matches!(s, BattleStatusEffect::Freeze { duration } if *duration >= 0)
            || matches!(s, BattleStatusEffect::Stun { duration } if *duration >= 0)
    })
}

/// Returns true if the unit's action fails due to paralysis (25% chance).
pub fn check_paralyze_failure(unit: &BattleUnit, rng: &mut impl Rng) -> bool {
    let paralyzed = unit.status_effects.iter().any(
        |s| matches!(s, BattleStatusEffect::Paralyze { duration } if *duration >= 0),
    );
    paralyzed && rng.r#gen::<f32>() < constants::PARALYZE_FAIL_CHANCE
}

/// Check if a unit is immune to a given status kind.
pub fn is_immune_to_status(unit: &BattleUnit, status_kind: StatusKind) -> bool {
    unit.status_effects.iter().any(|s| match s {
        BattleStatusEffect::Immunity { types, all_negative, .. } => {
            if *all_negative && is_negative_kind(status_kind) { return true; }
            types.contains(&status_kind)
        }
        _ => false,
    })
}

fn is_negative_kind(kind: StatusKind) -> bool {
    matches!(kind,
        StatusKind::Poison | StatusKind::Burn | StatusKind::Freeze
        | StatusKind::Stun | StatusKind::Paralyze | StatusKind::Blind
        | StatusKind::Debuff
    )
}

/// Attempt to apply a status effect to a unit. Returns true if applied.
pub fn apply_status_to_unit(unit: &mut BattleUnit, new_status: BattleStatusEffect) -> bool {
    let kind = new_status.kind();

    if kind != StatusKind::Immunity && is_immune_to_status(unit, kind) {
        return false;
    }

    if kind == StatusKind::Immunity {
        unit.status_effects.retain(|s| !matches!(s, BattleStatusEffect::Immunity { .. }));
    }

    // Stack limit for buffs/debuffs
    match &new_status {
        BattleStatusEffect::Buff { stat, .. } | BattleStatusEffect::Debuff { stat, .. } => {
            let count = unit.status_effects.iter().filter(|s| match (s, &new_status) {
                (BattleStatusEffect::Buff { stat: es, .. }, BattleStatusEffect::Buff { .. }) => es == stat,
                (BattleStatusEffect::Debuff { stat: es, .. }, BattleStatusEffect::Debuff { .. }) => es == stat,
                _ => false,
            }).count();
            if count >= constants::BUFF_DEBUFF_STACK_LIMIT {
                return false;
            }
        }
        _ => {}
    }

    unit.status_effects.push(new_status);
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::types::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_unit(hp: i32, max_hp: i32) -> BattleUnit {
        BattleUnit {
            id: 1, name: "TestUnit".into(), side: UnitSide::Player,
            element: Element::Venus, level: 5,
            hp, max_hp, pp: 50, max_pp: 50,
            atk: 10, def: 10, mag: 10, spd: 10, luck: 5,
            status_effects: vec![], ability_ids: vec![], djinn_ids: vec![],
            damage_taken: 0, damage_dealt: 0, xp: 0,
            growth_rates: GrowthRates::default(),
        }
    }

    #[test]
    fn test_poison_tick() {
        let mut unit = make_unit(100, 100);
        unit.status_effects.push(BattleStatusEffect::Poison { duration: 3 });
        let mut rng = StdRng::seed_from_u64(42);
        let result = tick_status_effects(&mut unit, &mut rng);
        assert_eq!(result.damage, 8);
        assert_eq!(unit.hp, 92);
    }

    #[test]
    fn test_immunity_blocks_status() {
        let mut unit = make_unit(100, 100);
        unit.status_effects.push(BattleStatusEffect::Immunity {
            types: vec![StatusKind::Poison], all_negative: false, duration: 5,
        });
        let applied = apply_status_to_unit(&mut unit, BattleStatusEffect::Poison { duration: 3 });
        assert!(!applied);
    }

    #[test]
    fn test_buff_stack_limit() {
        let mut unit = make_unit(100, 100);
        for _ in 0..constants::BUFF_DEBUFF_STACK_LIMIT {
            assert!(apply_status_to_unit(&mut unit,
                BattleStatusEffect::Buff { stat: StatKind::Atk, modifier: 5, duration: 3 }));
        }
        assert!(!apply_status_to_unit(&mut unit,
            BattleStatusEffect::Buff { stat: StatKind::Atk, modifier: 5, duration: 3 }));
    }
}
