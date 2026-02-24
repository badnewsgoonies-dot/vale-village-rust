//! Damage and healing calculation algorithms.
//!
//! Ported from TypeScript `damage.ts`. All functions are pure (no side effects, no ECS).
//! Randomness is injected via an `&mut impl Rng` parameter for determinism in tests.

use crate::battle::types::{
    AbilityDef, AbilityType, BattleStatusEffect, BattleUnit, Element, StatKind, constants,
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
// Effective stat with buff/debuff modifiers
// ---------------------------------------------------------------------------

/// Calculate a unit's effective stat by summing buff/debuff modifiers from status effects.
///
/// The result is clamped to a minimum of 1 so that no stat can drop to zero or below.
pub fn effective_stat(unit: &BattleUnit, stat: StatKind) -> i32 {
    let base = match stat {
        StatKind::Atk => unit.atk,
        StatKind::Def => unit.def,
        StatKind::Mag => unit.mag,
        StatKind::Spd => unit.spd,
        StatKind::Luck => unit.luck,
    };
    let modifier: i32 = unit
        .status_effects
        .iter()
        .filter_map(|s| match s {
            BattleStatusEffect::Buff {
                stat: s, modifier, ..
            } if *s == stat => Some(*modifier),
            BattleStatusEffect::Debuff {
                stat: s, modifier, ..
            } if *s == stat => Some(*modifier),
            _ => None,
        })
        .sum();
    (base + modifier).max(1) // never goes below 1
}

// ---------------------------------------------------------------------------
// Critical hit calculation
// ---------------------------------------------------------------------------

/// Determine whether an attack is a critical hit.
///
/// Base crit chance is `effective_luck / 200.0`, so luck=10 gives 5% and luck=50 gives 25%.
/// Uses effective luck (including buffs/debuffs).
pub fn calculate_crit(attacker: &BattleUnit, rng: &mut impl Rng) -> bool {
    let luck = effective_stat(attacker, StatKind::Luck) as f32;
    let crit_chance = (luck / 200.0).clamp(0.0, 1.0);
    rng.r#gen::<f32>() < crit_chance
}

// ---------------------------------------------------------------------------
// Accuracy / miss check
// ---------------------------------------------------------------------------

/// Check whether an attack hits its target.
///
/// Base hit rate is 95%. If the attacker has the Blind status, hit rate is reduced by 30%
/// (to 65%). Returns `true` if the attack lands.
#[allow(dead_code)]
pub fn check_accuracy(attacker: &BattleUnit, _defender: &BattleUnit, rng: &mut impl Rng) -> bool {
    let mut hit_rate: f32 = 0.95;

    let is_blind = attacker
        .status_effects
        .iter()
        .any(|s| matches!(s, BattleStatusEffect::Blind { .. }));
    if is_blind {
        hit_rate -= 0.30;
    }

    rng.r#gen::<f32>() < hit_rate
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
            && *remaining_charges > 0
            && !consumed
        {
            *remaining_charges -= 1;
            consumed = true;
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
/// Buff/debuff modifiers are applied to ATK and DEF via `effective_stat()`.
/// Crit chance is rolled; crits multiply final damage by 1.5x.
pub fn calculate_physical_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    rng: &mut impl Rng,
) -> i32 {
    let eff_atk = effective_stat(attacker, StatKind::Atk) as f32;
    let eff_def = effective_stat(defender, StatKind::Def) as f32;

    let base_power = if ability.base_power > 0 {
        ability.base_power as f32
    } else {
        eff_atk
    };
    let atk_power = eff_atk;
    let ignore_pct = ability.ignore_defense_percent.clamp(0.0, 1.0);
    let adjusted_def = eff_def * (1.0 - ignore_pct);
    let raw = base_power + atk_power - (adjusted_def * constants::DEFENSE_MULTIPLIER);

    let variance = rng.gen_range(constants::DAMAGE_VARIANCE_MIN..=constants::DAMAGE_VARIANCE_MAX);
    let mut damage = raw * variance;

    let dr = total_damage_reduction(defender);
    damage *= 1.0 - dr;

    if defender_is_defending {
        damage *= 1.0 - constants::DEFEND_DAMAGE_REDUCTION;
    }

    let is_crit = calculate_crit(attacker, rng);
    if is_crit {
        damage *= 1.5;
    }

    (damage.floor() as i32).max(constants::MINIMUM_DAMAGE)
}

// ---------------------------------------------------------------------------
// Psynergy (magic) damage
// ---------------------------------------------------------------------------

/// Calculate psynergy (magic) damage with a weather multiplier.
///
/// Formula: `(base_power + mag - def * 0.3 * (1 - ignore_def_pct)) * element_mod * weather_multiplier`
/// Buff/debuff modifiers are applied to MAG and DEF via `effective_stat()`.
/// Crit chance is rolled; crits multiply final damage by 1.5x.
///
/// The `weather_multiplier` scales final damage based on current weather conditions
/// (e.g. 1.10 for a 10% boost, 0.90 for a 10% reduction, 1.0 for no effect).
pub fn calculate_psynergy_damage_with_weather(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    weather_multiplier: f32,
    rng: &mut impl Rng,
) -> i32 {
    let eff_mag = effective_stat(attacker, StatKind::Mag) as f32;
    let eff_def = effective_stat(defender, StatKind::Def) as f32;

    let base_power = ability.base_power as f32;
    let ignore_pct = ability.ignore_defense_percent.clamp(0.0, 1.0);
    let adjusted_def = eff_def * (1.0 - ignore_pct);
    let mag_def = adjusted_def * constants::PSYNERGY_DEFENSE_MULTIPLIER;

    let elem_mod = match ability.element {
        Some(e) => element_modifier(e, defender.element),
        None => 1.0,
    };

    let raw = (base_power + eff_mag - mag_def) * elem_mod;
    let variance = rng.gen_range(constants::DAMAGE_VARIANCE_MIN..=constants::DAMAGE_VARIANCE_MAX);
    let mut damage = raw * variance;

    let dr = total_damage_reduction(defender);
    damage *= 1.0 - dr;

    if defender_is_defending {
        damage *= 1.0 - constants::DEFEND_DAMAGE_REDUCTION;
    }

    let is_crit = calculate_crit(attacker, rng);
    if is_crit {
        damage *= 1.5;
    }

    // Apply weather multiplier to final damage
    damage *= weather_multiplier;

    (damage.floor() as i32).max(constants::MINIMUM_DAMAGE)
}

/// Calculate psynergy (magic) damage without weather effects.
///
/// This is a backwards-compatible wrapper around [`calculate_psynergy_damage_with_weather`]
/// that passes a neutral weather multiplier of `1.0`.
#[allow(dead_code)]
pub fn calculate_psynergy_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    rng: &mut impl Rng,
) -> i32 {
    calculate_psynergy_damage_with_weather(
        attacker,
        defender,
        ability,
        defender_is_defending,
        1.0,
        rng,
    )
}

// ---------------------------------------------------------------------------
// Unified damage dispatcher
// ---------------------------------------------------------------------------

/// Calculate damage for any ability, dispatching on its kind, with weather effects.
///
/// For psynergy and debuff abilities, the `weather_multiplier` is applied.
/// Physical abilities are not affected by weather.
pub fn calculate_damage_with_weather(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    weather_multiplier: f32,
    rng: &mut impl Rng,
) -> i32 {
    match ability.ability_type {
        AbilityType::Physical => {
            calculate_physical_damage(attacker, defender, ability, defender_is_defending, rng)
        }
        AbilityType::Psynergy | AbilityType::Debuff => calculate_psynergy_damage_with_weather(
            attacker,
            defender,
            ability,
            defender_is_defending,
            weather_multiplier,
            rng,
        ),
        AbilityType::Healing | AbilityType::Buff => 0,
    }
}

/// Calculate damage for any ability, dispatching on its kind.
///
/// Backwards-compatible wrapper that passes a neutral weather multiplier of `1.0`.
pub fn calculate_damage(
    attacker: &BattleUnit,
    defender: &BattleUnit,
    ability: &AbilityDef,
    defender_is_defending: bool,
    rng: &mut impl Rng,
) -> i32 {
    calculate_damage_with_weather(attacker, defender, ability, defender_is_defending, 1.0, rng)
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

    // -------------------------------------------------------------------
    // effective_stat tests
    // -------------------------------------------------------------------

    #[test]
    fn test_effective_stat_with_buffs() {
        let mut unit = make_unit(20, 10, 10, 10, Element::Venus);
        unit.status_effects.push(BattleStatusEffect::Buff {
            stat: StatKind::Atk,
            modifier: 5,
            duration: 3,
        });
        assert_eq!(effective_stat(&unit, StatKind::Atk), 25); // 20 + 5
    }

    #[test]
    fn test_effective_stat_with_debuffs() {
        let mut unit = make_unit(10, 4, 10, 10, Element::Venus);
        // Apply a -3 debuff to DEF (base 4 - 3 = 1, stays at 1)
        unit.status_effects.push(BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -3,
            duration: 3,
        });
        assert_eq!(effective_stat(&unit, StatKind::Def), 1); // 4 + (-3) = 1, clamped to min 1

        // Apply an even larger debuff to test floor at 1
        let mut unit2 = make_unit(10, 4, 10, 10, Element::Venus);
        unit2.status_effects.push(BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -30,
            duration: 3,
        });
        assert_eq!(effective_stat(&unit2, StatKind::Def), 1); // 4 + (-30) = -26, clamped to 1
    }

    // -------------------------------------------------------------------
    // critical hit tests
    // -------------------------------------------------------------------

    #[test]
    fn test_crit_with_high_luck() {
        // luck=200 => crit chance = 200/200 = 1.0, always crits
        let mut unit = make_unit(10, 10, 10, 10, Element::Venus);
        unit.luck = 200;
        let mut rng = StdRng::seed_from_u64(42);
        // Test 20 times — all should crit
        for _ in 0..20 {
            assert!(calculate_crit(&unit, &mut rng));
        }
    }

    // -------------------------------------------------------------------
    // accuracy / blind tests
    // -------------------------------------------------------------------

    #[test]
    fn test_accuracy_blind() {
        let mut attacker = make_unit(10, 10, 10, 10, Element::Venus);
        attacker
            .status_effects
            .push(BattleStatusEffect::Blind { duration: 3 });
        let defender = make_unit(10, 10, 10, 10, Element::Venus);
        let mut rng = StdRng::seed_from_u64(42);

        let trials = 1000;
        let hits: usize = (0..trials)
            .filter(|_| check_accuracy(&attacker, &defender, &mut rng))
            .count();

        // Expected hit rate: 65%. Allow generous range [55%, 75%] for RNG variance.
        let hit_rate = hits as f64 / trials as f64;
        assert!(
            hit_rate > 0.55 && hit_rate < 0.75,
            "Blind hit rate {hit_rate:.2} outside expected range [0.55, 0.75]"
        );
    }

    // -------------------------------------------------------------------
    // buff affects physical damage test
    // -------------------------------------------------------------------

    #[test]
    fn test_buff_affects_physical_damage() {
        let ability = basic_physical();

        // Unbuffed attacker
        let attacker_no_buff = make_unit(20, 10, 10, 10, Element::Venus);
        let defender = make_unit(10, 10, 10, 10, Element::Venus);
        let mut rng1 = StdRng::seed_from_u64(99);
        let dmg_no_buff =
            calculate_physical_damage(&attacker_no_buff, &defender, &ability, false, &mut rng1);

        // Buffed attacker (+15 ATK)
        let mut attacker_buffed = make_unit(20, 10, 10, 10, Element::Venus);
        attacker_buffed
            .status_effects
            .push(BattleStatusEffect::Buff {
                stat: StatKind::Atk,
                modifier: 15,
                duration: 3,
            });
        let mut rng2 = StdRng::seed_from_u64(99);
        let dmg_buffed =
            calculate_physical_damage(&attacker_buffed, &defender, &ability, false, &mut rng2);

        assert!(
            dmg_buffed > dmg_no_buff,
            "Buffed damage ({dmg_buffed}) should be greater than unbuffed ({dmg_no_buff})"
        );
    }

    // -------------------------------------------------------------------
    // weather multiplier tests
    // -------------------------------------------------------------------

    /// Helper: create a psynergy ability with the given element.
    fn psynergy_ability(element: Element) -> AbilityDef {
        AbilityDef {
            id: "psy_test".into(),
            name: "Psy Test".into(),
            ability_type: AbilityType::Psynergy,
            element: Some(element),
            mana_cost: 5,
            base_power: 50,
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
    fn test_weather_rain_boosts_mercury_damage() {
        // Rain boosts Mercury by 10% => weather_multiplier = 1.10
        let attacker = make_unit(10, 10, 30, 10, Element::Mercury);
        let defender = make_unit(10, 10, 10, 10, Element::Neutral);
        let ability = psynergy_ability(Element::Mercury);

        let mut rng_base = StdRng::seed_from_u64(42);
        let dmg_no_weather = calculate_psynergy_damage_with_weather(
            &attacker,
            &defender,
            &ability,
            false,
            1.0,
            &mut rng_base,
        );

        let mut rng_rain = StdRng::seed_from_u64(42);
        let dmg_rain = calculate_psynergy_damage_with_weather(
            &attacker,
            &defender,
            &ability,
            false,
            1.10,
            &mut rng_rain,
        );

        assert!(
            dmg_rain > dmg_no_weather,
            "Rain + Mercury should boost damage: rain={dmg_rain}, base={dmg_no_weather}"
        );
    }

    #[test]
    fn test_weather_rain_reduces_mars_damage() {
        // Rain reduces Mars by 10% => weather_multiplier = 0.90
        let attacker = make_unit(10, 10, 30, 10, Element::Mars);
        let defender = make_unit(10, 10, 10, 10, Element::Neutral);
        let ability = psynergy_ability(Element::Mars);

        let mut rng_base = StdRng::seed_from_u64(42);
        let dmg_no_weather = calculate_psynergy_damage_with_weather(
            &attacker,
            &defender,
            &ability,
            false,
            1.0,
            &mut rng_base,
        );

        let mut rng_rain = StdRng::seed_from_u64(42);
        let dmg_rain = calculate_psynergy_damage_with_weather(
            &attacker,
            &defender,
            &ability,
            false,
            0.90,
            &mut rng_rain,
        );

        assert!(
            dmg_rain < dmg_no_weather,
            "Rain + Mars should reduce damage: rain={dmg_rain}, base={dmg_no_weather}"
        );
    }

    #[test]
    fn test_weather_clear_no_effect() {
        // Clear weather => weather_multiplier = 1.0, no change
        let attacker = make_unit(10, 10, 30, 10, Element::Venus);
        let defender = make_unit(10, 10, 10, 10, Element::Neutral);
        let ability = psynergy_ability(Element::Venus);

        let mut rng1 = StdRng::seed_from_u64(42);
        let dmg_clear = calculate_psynergy_damage_with_weather(
            &attacker, &defender, &ability, false, 1.0, &mut rng1,
        );

        let mut rng2 = StdRng::seed_from_u64(42);
        let dmg_wrapper =
            calculate_psynergy_damage(&attacker, &defender, &ability, false, &mut rng2);

        assert_eq!(
            dmg_clear, dmg_wrapper,
            "Clear weather (1.0) should produce identical damage to the no-weather wrapper: \
             clear={dmg_clear}, wrapper={dmg_wrapper}"
        );
    }
}
