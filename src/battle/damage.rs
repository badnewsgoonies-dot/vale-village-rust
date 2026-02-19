//! Damage and healing calculation algorithms.
//!
//! Ported from TypeScript `damage.ts`. All functions are pure (no side effects, no ECS).
//! Randomness is injected via an `&mut impl Rng` parameter for determinism in tests.

use crate::battle::types::{
    AbilityDef, AbilityType, BattleStatusEffect, BattleUnit, Element, constants,
};
use rand::Rng;

// ---------------------------------------------------------------------------
// Element advantage
// ---------------------------------------------------------------------------

/// Returns the element modifier for an attack.
///
/// Uses `Element::modifier_against` from shared types, which implements:
/// Advantage cycle: Venus > Jupiter > Mercury > Mars > Venus (1.25x / 0.75x / 1.0x)
pub fn element_modifier(atk_element: Element, def_element: Element) -> f32 {
    atk_element.modifier_against(&def_element)
}

// ---------------------------------------------------------------------------
// Damage modifiers from status effects
// ---------------------------------------------------------------------------

/// Sum damage-reduction percentages from status effects (clamped to [0, 1]).
fn total_damage_reduction(defender: &BattleUnit) -> f32 {
    let sum: f32 = defender
        .status_effects
        .iter()
        .filter_map(|s| match s {
            BattleStatusEffect::DamageReduction { percent, .. } => Some(*percent),
            _ => None,
        })
        .sum();
    sum.clamp(0.0, 1.0)
}

/// Check if the defender is currently invulnerable.
pub fn is_invulnerable(unit: &BattleUnit) -> bool {
    unit.status_effects
        .iter()
        .any(|s| matches!(s, BattleStatusEffect::Invulnerable { .. }))
}

/// Check if the defender has active shield charges.
pub fn has_shield_charges(unit: &BattleUnit) -> bool {
    unit.status_effects.iter().any(
        |s| matches!(s, BattleStatusEffect::Shield { remaining_charges, .. } if *remaining_charges > 0),
    )
}

/// Consume one shield charge from the first available shield.
pub fn consume_shield_charge(unit: &mut BattleUnit) {
    let mut consumed = false;
    for effect in unit.status_effects.iter_mut() {
        if let BattleStatusEffect::Shield {
            remaining_charges, ..
        } = effect
        {
            if *remaining_charges > 0 && !consumed {
                *remaining_charges -= 1;
                consumed = true;
            }
        }
    }
    unit.status_effects.retain(|s| {
        !matches!(s, BattleStatusEffect::Shield { remaining_charges, .. } if *remaining_charges <= 0)
    });
}

// ---------------------------------------------------------------------------
// Physical damage
// ---------------------------------------------------------------------------

/// Calculate physical damage.
///
/// Formula: `(base_power + atk) - (def * 0.5 * (1 - ignore_def_pct))`
pub fn calculate_physical_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    rng: &mut impl Rng,
) -> i32 {
    let base_power = if ability.base_power > 0 {
        ability.base_power as f32
    } else {
        attacker.atk as f32
    };
    let atk_power = attacker.atk as f32;
    let ignore_pct = ability.ignore_defense_percent.clamp(0.0, 1.0);
    let effective_def = defender.def as f32 * (1.0 - ignore_pct);
    let raw = base_power + atk_power - (effective_def * constants::DEFENSE_MULTIPLIER);

    let variance = rng.gen_range(constants::DAMAGE_VARIANCE_MIN..=constants::DAMAGE_VARIANCE_MAX);
    let mut damage = raw * variance;

    let dr = total_damage_reduction(defender);
    damage *= 1.0 - dr;

    if defender_is_defending {
        damage *= 1.0 - constants::DEFEND_DAMAGE_REDUCTION;
    }

    (damage.floor() as i32).max(constants::MINIMUM_DAMAGE)
}

// ---------------------------------------------------------------------------
// Psynergy (magic) damage
// ---------------------------------------------------------------------------

/// Calculate psynergy (magic) damage.
///
/// Formula: `(base_power + mag - def * 0.3 * (1 - ignore_def_pct)) * element_mod`
pub fn calculate_psynergy_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    rng: &mut impl Rng,
) -> i32 {
    let base_power = ability.base_power as f32;
    let mag = attacker.mag as f32;
    let ignore_pct = ability.ignore_defense_percent.clamp(0.0, 1.0);
    let effective_def = defender.def as f32 * (1.0 - ignore_pct);
    let mag_def = effective_def * constants::PSYNERGY_DEFENSE_MULTIPLIER;

    let elem_mod = match ability.element {
        Some(e) => element_modifier(e, defender.element),
        None => 1.0,
    };

    let raw = (base_power + mag - mag_def) * elem_mod;
    let variance = rng.gen_range(constants::DAMAGE_VARIANCE_MIN..=constants::DAMAGE_VARIANCE_MAX);
    let mut damage = raw * variance;

    let dr = total_damage_reduction(defender);
    damage *= 1.0 - dr;

    if defender_is_defending {
        damage *= 1.0 - constants::DEFEND_DAMAGE_REDUCTION;
    }

    (damage.floor() as i32).max(constants::MINIMUM_DAMAGE)
}

// ---------------------------------------------------------------------------
// Unified damage dispatcher
// ---------------------------------------------------------------------------

/// Calculate damage for any ability, dispatching on its kind.
pub fn calculate_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    rng: &mut impl Rng,
) -> i32 {
    match ability.ability_type {
        AbilityType::Physical => {
            calculate_physical_damage(attacker, defender, ability, defender_is_defending, rng)
        }
        AbilityType::Psynergy | AbilityType::Debuff => {
            calculate_psynergy_damage(attacker, defender, ability, defender_is_defending, rng)
        }
        AbilityType::Healing | AbilityType::Buff => 0,
    }
}

// ---------------------------------------------------------------------------
// Healing
// ---------------------------------------------------------------------------

/// Calculate heal amount. Formula: `base_power + mag`.
pub fn calculate_heal_amount(caster: &BattleUnit, ability: &AbilityDef) -> i32 {
    let base_heal = ability.base_power;
    if base_heal <= 0 {
        return 0;
    }
    (base_heal + caster.mag).max(constants::MINIMUM_HEALING)
}

/// Apply healing. No-op if KO'd without revive. Never exceeds max HP.
pub fn apply_healing(unit: &mut BattleUnit, amount: i32, revives_fallen: bool) {
    if amount <= 0 {
        return;
    }
    if unit.is_ko() && !revives_fallen {
        return;
    }
    unit.hp = (unit.hp + amount).min(unit.max_hp);
}

// ---------------------------------------------------------------------------
// Damage application with shields / invuln / auto-revive
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DamageResult {
    pub actual_damage: i32,
    pub was_blocked: bool,
    #[allow(dead_code)]
    pub auto_revived: bool,
}

/// Apply damage with full shield / invulnerability / auto-revive pipeline.
pub fn apply_damage_with_shields(unit: &mut BattleUnit, damage: i32) -> DamageResult {
    if damage <= 0 {
        return DamageResult {
            actual_damage: 0,
            was_blocked: false,
            auto_revived: false,
        };
    }

    if is_invulnerable(unit) {
        return DamageResult {
            actual_damage: 0,
            was_blocked: true,
            auto_revived: false,
        };
    }

    if has_shield_charges(unit) {
        consume_shield_charge(unit);
        return DamageResult {
            actual_damage: 0,
            was_blocked: true,
            auto_revived: false,
        };
    }

    unit.hp = (unit.hp - damage).max(0);
    unit.damage_taken += damage;

    let auto_revived = if unit.is_ko() {
        try_auto_revive(unit)
    } else {
        false
    };

    DamageResult {
        actual_damage: damage,
        was_blocked: false,
        auto_revived,
    }
}

fn try_auto_revive(unit: &mut BattleUnit) -> bool {
    let idx = unit.status_effects.iter().position(|s| {
        matches!(s, BattleStatusEffect::AutoRevive { uses_remaining, .. } if *uses_remaining > 0)
    });

    if let Some(i) = idx {
        if let BattleStatusEffect::AutoRevive {
            hp_percent,
            uses_remaining,
        } = &mut unit.status_effects[i]
        {
            let revive_hp = (unit.max_hp as f32 * *hp_percent).floor() as i32;
            unit.hp = revive_hp.max(1);
            *uses_remaining -= 1;
        }
        unit.status_effects.retain(|s| {
            !matches!(s, BattleStatusEffect::AutoRevive { uses_remaining, .. } if *uses_remaining <= 0)
        });
        true
    } else {
        unit.status_effects.retain(|s| {
            !matches!(s, BattleStatusEffect::AutoRevive { uses_remaining, .. } if *uses_remaining <= 0)
        });
        false
    }
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

    fn make_unit(atk: i32, def: i32, mag: i32, spd: i32, element: Element) -> BattleUnit {
        BattleUnit {
            id: 1,
            name: "Test".into(),
            side: UnitSide::Player,
            element,
            level: 5,
            hp: 100,
            max_hp: 100,
            pp: 50,
            max_pp: 50,
            atk,
            def,
            mag,
            spd,
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

    fn basic_physical() -> AbilityDef {
        AbilityDef {
            id: "test".into(),
            name: "Test".into(),
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
        }
    }

    #[test]
    fn test_element_advantage() {
        assert_eq!(element_modifier(Element::Venus, Element::Jupiter), 1.25);
        assert_eq!(element_modifier(Element::Jupiter, Element::Venus), 0.75);
        assert_eq!(element_modifier(Element::Mars, Element::Mars), 1.0);
        assert_eq!(element_modifier(Element::Neutral, Element::Venus), 1.0);
    }

    #[test]
    fn test_physical_damage_minimum() {
        let attacker = make_unit(1, 10, 10, 10, Element::Venus);
        let defender = make_unit(10, 999, 10, 10, Element::Venus);
        let ability = basic_physical();
        let mut rng = StdRng::seed_from_u64(42);
        let dmg = calculate_physical_damage(&attacker, &defender, &ability, false, &mut rng);
        assert!(dmg >= constants::MINIMUM_DAMAGE);
    }

    #[test]
    fn test_healing_ko_unit() {
        let mut unit = make_unit(10, 10, 10, 10, Element::Venus);
        unit.hp = 0;
        apply_healing(&mut unit, 50, false);
        assert_eq!(unit.hp, 0);
        apply_healing(&mut unit, 50, true);
        assert_eq!(unit.hp, 50);
    }

    #[test]
    fn test_healing_never_exceeds_max() {
        let mut unit = make_unit(10, 10, 10, 10, Element::Venus);
        unit.hp = 90;
        apply_healing(&mut unit, 50, false);
        assert_eq!(unit.hp, 100);
    }

    #[test]
    fn test_shield_blocks_damage() {
        let mut unit = make_unit(10, 10, 10, 10, Element::Venus);
        unit.status_effects.push(BattleStatusEffect::Shield {
            remaining_charges: 1,
            duration: 3,
        });
        let result = apply_damage_with_shields(&mut unit, 50);
        assert!(result.was_blocked);
        assert_eq!(result.actual_damage, 0);
        assert_eq!(unit.hp, 100);
    }

    #[test]
    fn test_auto_revive() {
        let mut unit = make_unit(10, 10, 10, 10, Element::Venus);
        unit.status_effects.push(BattleStatusEffect::AutoRevive {
            hp_percent: 0.5,
            uses_remaining: 1,
        });
        let result = apply_damage_with_shields(&mut unit, 200);
        assert!(result.auto_revived);
        assert_eq!(unit.hp, 50);
    }
}
