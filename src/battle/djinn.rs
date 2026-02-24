//! Djinn battle mechanics.
//!
//! Ported from TypeScript `djinn.ts` and `djinnAbilities.ts`.

use std::collections::HashMap;

use crate::battle::types::{DjinnBattleRes, DjinnBattleState, Element, constants};
use crate::data::djinn::{DjinnDefinition, SummonEffectKind};

// ---------------------------------------------------------------------------
// Element compatibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ElementCompatibility {
    Same,
    Counter,
    Neutral,
}

#[allow(dead_code)]
pub fn element_compatibility(
    unit_element: Element,
    djinn_element: Element,
) -> ElementCompatibility {
    if unit_element == djinn_element {
        return ElementCompatibility::Same;
    }
    let is_counter = matches!(
        (unit_element, djinn_element),
        (Element::Venus, Element::Jupiter)
            | (Element::Jupiter, Element::Venus)
            | (Element::Mars, Element::Mercury)
            | (Element::Mercury, Element::Mars)
    );
    if is_counter {
        ElementCompatibility::Counter
    } else {
        ElementCompatibility::Neutral
    }
}

// ---------------------------------------------------------------------------
// Synergy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DjinnSynergy {
    pub atk_bonus: i32,
    pub def_bonus: i32,
    pub spd_bonus: i32,
    pub class_name: String,
    pub abilities_unlocked: Vec<String>,
}

#[allow(dead_code)]
pub fn calculate_djinn_synergy(set_elements: &[Element]) -> DjinnSynergy {
    if set_elements.is_empty() {
        return DjinnSynergy {
            atk_bonus: 0,
            def_bonus: 0,
            spd_bonus: 0,
            class_name: "Base".into(),
            abilities_unlocked: vec![],
        };
    }

    let mut counts = std::collections::HashMap::new();
    for &e in set_elements {
        *counts.entry(e).or_insert(0u32) += 1;
    }
    let unique = counts.len();
    let max_count = *counts.values().max().unwrap_or(&0);
    let primary = counts
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(e, _)| *e)
        .unwrap_or(Element::Neutral);

    match (set_elements.len(), unique, max_count) {
        (1, _, _) => DjinnSynergy {
            atk_bonus: 4,
            def_bonus: 3,
            spd_bonus: 0,
            class_name: "Adept".into(),
            abilities_unlocked: vec![],
        },
        (2, 1, _) => DjinnSynergy {
            atk_bonus: 8,
            def_bonus: 5,
            spd_bonus: 0,
            class_name: format!("{:?} Warrior", primary),
            abilities_unlocked: vec![],
        },
        (2, 2, _) => DjinnSynergy {
            atk_bonus: 5,
            def_bonus: 5,
            spd_bonus: 0,
            class_name: "Hybrid".into(),
            abilities_unlocked: vec![],
        },
        (3, 1, _) => DjinnSynergy {
            atk_bonus: 12,
            def_bonus: 8,
            spd_bonus: 0,
            class_name: format!("{:?} Adept", primary),
            abilities_unlocked: vec![format!("{:?}-Ultimate", primary)],
        },
        (3, 2, 2) => DjinnSynergy {
            atk_bonus: 8,
            def_bonus: 6,
            spd_bonus: 0,
            class_name: format!("{:?} Knight", primary),
            abilities_unlocked: vec!["Hybrid-Spell".into()],
        },
        (3, _, _) => DjinnSynergy {
            atk_bonus: 4,
            def_bonus: 4,
            spd_bonus: 4,
            class_name: "Mystic".into(),
            abilities_unlocked: vec!["Elemental Harmony".into()],
        },
        _ => DjinnSynergy {
            atk_bonus: 0,
            def_bonus: 0,
            spd_bonus: 0,
            class_name: "Base".into(),
            abilities_unlocked: vec![],
        },
    }
}

// ---------------------------------------------------------------------------
// Summon result
// ---------------------------------------------------------------------------

/// Result of a summon using actual djinn definitions for damage/effects.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SummonResult {
    pub total_damage: i32,
    pub total_healing: i32,
    /// Status effects to inflict: (effect_type, duration).
    pub status_inflicts: Vec<(String, u8)>,
    /// Stat buffs for allies: (stat_name, amount).
    pub stat_buffs: Vec<(String, i32)>,
}

// ---------------------------------------------------------------------------
// Set djinn bonuses and granted abilities
// ---------------------------------------------------------------------------

/// Calculate the total stat bonuses from all Set djinn on a given unit.
/// Returns (atk_bonus, def_bonus, mag_bonus, spd_bonus, hp_bonus, pp_bonus).
#[allow(dead_code)]
pub fn calculate_set_bonuses(
    unit_id: u32,
    djinn_state: &DjinnBattleRes,
    djinn_registry: &HashMap<String, DjinnDefinition>,
) -> (i32, i32, i32, i32, i32, i32) {
    let mut atk = 0;
    let mut def = 0;
    let mut mag = 0;
    let mut spd = 0;
    let mut hp = 0;
    let mut pp = 0;

    for tracker in &djinn_state.trackers {
        if tracker.owner_unit_id == unit_id
            && tracker.state == DjinnBattleState::Set
            && let Some(djinn_def) = djinn_registry.get(&tracker.djinn_id)
        {
            atk += djinn_def.set_bonus.atk;
            def += djinn_def.set_bonus.def;
            mag += djinn_def.set_bonus.mag;
            spd += djinn_def.set_bonus.spd;
            hp += djinn_def.set_bonus.hp;
            pp += djinn_def.set_bonus.pp;
        }
    }

    (atk, def, mag, spd, hp, pp)
}

/// Get all ability IDs granted by Set djinn on a given unit.
#[allow(dead_code)]
pub fn get_granted_abilities(
    unit_id: u32,
    djinn_state: &DjinnBattleRes,
    djinn_registry: &HashMap<String, DjinnDefinition>,
) -> Vec<String> {
    let mut abilities = Vec::new();
    for tracker in &djinn_state.trackers {
        if tracker.owner_unit_id == unit_id
            && tracker.state == DjinnBattleState::Set
            && let Some(djinn_def) = djinn_registry.get(&tracker.djinn_id)
        {
            abilities.extend(djinn_def.granted_ability_ids.clone());
        }
    }
    abilities
}

// ---------------------------------------------------------------------------
// State transitions
// ---------------------------------------------------------------------------

pub fn unleash_djinn(
    djinn_id: &str,
    current_turn: u32,
    djinn_state: &mut DjinnBattleRes,
) -> Result<(), String> {
    let tracker = djinn_state
        .trackers
        .iter_mut()
        .find(|t| t.djinn_id == djinn_id)
        .ok_or_else(|| format!("Djinn '{}' not found in battle", djinn_id))?;

    if tracker.state != DjinnBattleState::Set {
        return Err(format!(
            "Djinn '{}' is {:?}, must be Set",
            djinn_id, tracker.state
        ));
    }

    tracker.state = DjinnBattleState::Standby;
    tracker.last_activated_turn = current_turn;
    Ok(())
}

pub fn summon_djinn(
    djinn_ids: &[String],
    current_turn: u32,
    djinn_state: &mut DjinnBattleRes,
) -> Result<i32, String> {
    let count = djinn_ids.len();
    if count == 0 || count > 3 {
        return Err(format!("Summon requires 1-3 Djinn, got {}", count));
    }

    for id in djinn_ids {
        let t = djinn_state
            .trackers
            .iter()
            .find(|t| t.djinn_id == *id)
            .ok_or_else(|| format!("Djinn '{}' not found", id))?;
        if t.state != DjinnBattleState::Standby {
            return Err(format!("Djinn '{}' is {:?}, must be Standby", id, t.state));
        }
    }

    for id in djinn_ids {
        if let Some(t) = djinn_state.trackers.iter_mut().find(|t| t.djinn_id == *id) {
            t.state = DjinnBattleState::Recovery;
            t.last_activated_turn = current_turn;
            t.recovery_turns_remaining = constants::DJINN_RECOVERY_TURNS;
        }
    }

    Ok(match count {
        1 => constants::SUMMON_DAMAGE_1,
        2 => constants::SUMMON_DAMAGE_2,
        3 => constants::SUMMON_DAMAGE_3,
        _ => unreachable!(),
    })
}

/// Enhanced summon that uses actual djinn definitions for damage/effects.
///
/// Validates that all djinn are in Standby, then transitions them to Recovery
/// using their per-definition recovery turns. Aggregates effects from all
/// participating djinn into a single `SummonResult`.
#[allow(dead_code)]
pub fn summon_djinn_enhanced(
    djinn_ids: &[String],
    current_turn: u32,
    djinn_state: &mut DjinnBattleRes,
    djinn_registry: &HashMap<String, DjinnDefinition>,
) -> Result<SummonResult, String> {
    let count = djinn_ids.len();
    if count == 0 || count > 3 {
        return Err(format!("Summon requires 1-3 Djinn, got {}", count));
    }

    // Validate all djinn exist and are in Standby.
    for id in djinn_ids {
        let t = djinn_state
            .trackers
            .iter()
            .find(|t| t.djinn_id == *id)
            .ok_or_else(|| format!("Djinn '{}' not found", id))?;
        if t.state != DjinnBattleState::Standby {
            return Err(format!("Djinn '{}' is {:?}, must be Standby", id, t.state));
        }
        if !djinn_registry.contains_key(id.as_str()) {
            return Err(format!("Djinn '{}' has no definition in registry", id));
        }
    }

    // Aggregate effects from all djinn definitions.
    let mut result = SummonResult {
        total_damage: 0,
        total_healing: 0,
        status_inflicts: Vec::new(),
        stat_buffs: Vec::new(),
    };

    for id in djinn_ids {
        if let Some(djinn_def) = djinn_registry.get(id.as_str()) {
            match &djinn_def.summon_effect.kind {
                SummonEffectKind::Damage { amount } => {
                    result.total_damage += amount;
                }
                SummonEffectKind::Heal { amount } => {
                    result.total_healing += amount;
                }
                SummonEffectKind::Buff { stat_bonus } => {
                    if stat_bonus.atk != 0 {
                        result.stat_buffs.push(("atk".into(), stat_bonus.atk));
                    }
                    if stat_bonus.def != 0 {
                        result.stat_buffs.push(("def".into(), stat_bonus.def));
                    }
                    if stat_bonus.mag != 0 {
                        result.stat_buffs.push(("mag".into(), stat_bonus.mag));
                    }
                    if stat_bonus.spd != 0 {
                        result.stat_buffs.push(("spd".into(), stat_bonus.spd));
                    }
                    if stat_bonus.hp != 0 {
                        result.stat_buffs.push(("hp".into(), stat_bonus.hp));
                    }
                    if stat_bonus.pp != 0 {
                        result.stat_buffs.push(("pp".into(), stat_bonus.pp));
                    }
                }
                SummonEffectKind::StatusInflict {
                    effect_type,
                    duration,
                } => {
                    result
                        .status_inflicts
                        .push((effect_type.clone(), *duration));
                }
            }
        }
    }

    // Transition all djinn to Recovery using per-definition recovery turns.
    for id in djinn_ids {
        if let Some(t) = djinn_state.trackers.iter_mut().find(|t| t.djinn_id == *id) {
            t.state = DjinnBattleState::Recovery;
            t.last_activated_turn = current_turn;
            // Use the djinn definition's recovery_turns if available; fall back to constant.
            t.recovery_turns_remaining = djinn_registry
                .get(id.as_str())
                .map(|d| d.recovery_turns as u32)
                .unwrap_or(constants::DJINN_RECOVERY_TURNS);
        }
    }

    Ok(result)
}

pub fn tick_djinn_recovery(djinn_state: &mut DjinnBattleRes) -> Vec<String> {
    let mut recovered = Vec::new();
    for t in djinn_state.trackers.iter_mut() {
        if t.state == DjinnBattleState::Recovery {
            if t.recovery_turns_remaining <= 1 {
                t.state = DjinnBattleState::Set;
                t.recovery_turns_remaining = 0;
                recovered.push(t.djinn_id.clone());
            } else {
                t.recovery_turns_remaining -= 1;
            }
        }
    }
    recovered
}

#[allow(dead_code)]
pub fn get_standby_djinn(unit_id: u32, djinn_state: &DjinnBattleRes) -> Vec<String> {
    djinn_state
        .trackers
        .iter()
        .filter(|t| t.owner_unit_id == unit_id && t.state == DjinnBattleState::Standby)
        .map(|t| t.djinn_id.clone())
        .collect()
}

#[allow(dead_code)]
pub fn get_set_djinn(unit_id: u32, djinn_state: &DjinnBattleRes) -> Vec<String> {
    djinn_state
        .trackers
        .iter()
        .filter(|t| t.owner_unit_id == unit_id && t.state == DjinnBattleState::Set)
        .map(|t| t.djinn_id.clone())
        .collect()
}

#[allow(dead_code)]
pub fn can_unleash(djinn_id: &str, djinn_state: &DjinnBattleRes) -> bool {
    djinn_state
        .trackers
        .iter()
        .any(|t| t.djinn_id == djinn_id && t.state == DjinnBattleState::Set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::types::DjinnTracker;

    fn make_djinn_state(djinn: &[(&str, u32, DjinnBattleState)]) -> DjinnBattleRes {
        DjinnBattleRes {
            trackers: djinn
                .iter()
                .map(|(id, owner, state)| DjinnTracker {
                    djinn_id: id.to_string(),
                    state: *state,
                    owner_unit_id: *owner,
                    last_activated_turn: 0,
                    recovery_turns_remaining: 0,
                })
                .collect(),
        }
    }

    #[test]
    fn test_unleash() {
        let mut state = make_djinn_state(&[("flint", 1, DjinnBattleState::Set)]);
        assert!(unleash_djinn("flint", 3, &mut state).is_ok());
        assert_eq!(state.trackers[0].state, DjinnBattleState::Standby);
    }

    #[test]
    fn test_summon() {
        let mut state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Standby),
            ("forge", 1, DjinnBattleState::Standby),
        ]);
        let dmg = summon_djinn(&["flint".into(), "forge".into()], 5, &mut state);
        assert_eq!(dmg, Ok(constants::SUMMON_DAMAGE_2));
        assert!(
            state
                .trackers
                .iter()
                .all(|t| t.state == DjinnBattleState::Recovery)
        );
    }

    #[test]
    fn test_recovery_tick() {
        let mut state = DjinnBattleRes {
            trackers: vec![DjinnTracker {
                djinn_id: "flint".into(),
                state: DjinnBattleState::Recovery,
                owner_unit_id: 1,
                last_activated_turn: 3,
                recovery_turns_remaining: 2,
            }],
        };
        assert!(tick_djinn_recovery(&mut state).is_empty());
        assert_eq!(state.trackers[0].recovery_turns_remaining, 1);
        let recovered = tick_djinn_recovery(&mut state);
        assert_eq!(recovered, vec!["flint"]);
        assert_eq!(state.trackers[0].state, DjinnBattleState::Set);
    }

    #[test]
    fn test_synergy_three_same() {
        let s = calculate_djinn_synergy(&[Element::Mars, Element::Mars, Element::Mars]);
        assert_eq!(s.atk_bonus, 12);
        assert!(s.class_name.contains("Adept"));
    }

    // -----------------------------------------------------------------------
    // Element compatibility tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_element_compatibility_same() {
        assert_eq!(
            element_compatibility(Element::Venus, Element::Venus),
            ElementCompatibility::Same
        );
        assert_eq!(
            element_compatibility(Element::Mars, Element::Mars),
            ElementCompatibility::Same
        );
        assert_eq!(
            element_compatibility(Element::Mercury, Element::Mercury),
            ElementCompatibility::Same
        );
        assert_eq!(
            element_compatibility(Element::Jupiter, Element::Jupiter),
            ElementCompatibility::Same
        );
    }

    #[test]
    fn test_element_compatibility_counter() {
        assert_eq!(
            element_compatibility(Element::Venus, Element::Jupiter),
            ElementCompatibility::Counter
        );
        assert_eq!(
            element_compatibility(Element::Jupiter, Element::Venus),
            ElementCompatibility::Counter
        );
        assert_eq!(
            element_compatibility(Element::Mars, Element::Mercury),
            ElementCompatibility::Counter
        );
        assert_eq!(
            element_compatibility(Element::Mercury, Element::Mars),
            ElementCompatibility::Counter
        );
    }

    #[test]
    fn test_element_compatibility_neutral() {
        assert_eq!(
            element_compatibility(Element::Venus, Element::Mars),
            ElementCompatibility::Neutral
        );
        assert_eq!(
            element_compatibility(Element::Venus, Element::Mercury),
            ElementCompatibility::Neutral
        );
        assert_eq!(
            element_compatibility(Element::Jupiter, Element::Mars),
            ElementCompatibility::Neutral
        );
        assert_eq!(
            element_compatibility(Element::Jupiter, Element::Mercury),
            ElementCompatibility::Neutral
        );
    }

    // -----------------------------------------------------------------------
    // Helper: build a small djinn registry for tests
    // -----------------------------------------------------------------------

    use crate::data::djinn::{
        DjinnDefinition, DjinnTier, StatModifier, SummonEffect, SummonEffectKind,
    };

    fn make_test_registry() -> HashMap<String, DjinnDefinition> {
        let mut reg = HashMap::new();
        reg.insert(
            "flint".into(),
            DjinnDefinition {
                id: "flint".into(),
                name: "Flint".into(),
                element: Element::Venus,
                tier: DjinnTier::Tier1,
                set_bonus: StatModifier {
                    atk: 3,
                    def: 2,
                    ..Default::default()
                },
                summon_effect: SummonEffect {
                    kind: SummonEffectKind::Damage { amount: 80 },
                    description: "Stone Barrage".into(),
                },
                granted_ability_ids: vec!["earth-spike-damage".into()],
                recovery_turns: 2,
                description: "A steadfast earth djinn.".into(),
            },
        );
        reg.insert(
            "forge".into(),
            DjinnDefinition {
                id: "forge".into(),
                name: "Forge".into(),
                element: Element::Mars,
                tier: DjinnTier::Tier1,
                set_bonus: StatModifier {
                    atk: 4,
                    mag: 2,
                    ..Default::default()
                },
                summon_effect: SummonEffect {
                    kind: SummonEffectKind::Damage { amount: 120 },
                    description: "Firebolt barrage".into(),
                },
                granted_ability_ids: vec!["fireball".into()],
                recovery_turns: 2,
                description: "A fiery mars djinn.".into(),
            },
        );
        reg.insert(
            "granite".into(),
            DjinnDefinition {
                id: "granite".into(),
                name: "Granite".into(),
                element: Element::Venus,
                tier: DjinnTier::Tier2,
                set_bonus: StatModifier {
                    def: 5,
                    hp: 10,
                    ..Default::default()
                },
                summon_effect: SummonEffect {
                    kind: SummonEffectKind::Buff {
                        stat_bonus: StatModifier {
                            def: 10,
                            ..Default::default()
                        },
                    },
                    description: "Terra Wall".into(),
                },
                granted_ability_ids: vec!["stone-skin-utility".into()],
                recovery_turns: 3,
                description: "A fortifying earth djinn.".into(),
            },
        );
        reg.insert(
            "fever".into(),
            DjinnDefinition {
                id: "fever".into(),
                name: "Fever".into(),
                element: Element::Mars,
                tier: DjinnTier::Tier2,
                set_bonus: StatModifier {
                    atk: 3,
                    spd: 3,
                    ..Default::default()
                },
                summon_effect: SummonEffect {
                    kind: SummonEffectKind::StatusInflict {
                        effect_type: "burn".into(),
                        duration: 3,
                    },
                    description: "Inflames enemies".into(),
                },
                granted_ability_ids: vec!["burn-touch".into()],
                recovery_turns: 3,
                description: "A feverish mars djinn.".into(),
            },
        );
        reg.insert(
            "fizz".into(),
            DjinnDefinition {
                id: "fizz".into(),
                name: "Fizz".into(),
                element: Element::Mercury,
                tier: DjinnTier::Tier1,
                set_bonus: StatModifier {
                    def: 2,
                    mag: 3,
                    ..Default::default()
                },
                summon_effect: SummonEffect {
                    kind: SummonEffectKind::Heal { amount: 100 },
                    description: "Healing waters".into(),
                },
                granted_ability_ids: vec!["heal".into()],
                recovery_turns: 2,
                description: "A healing mercury djinn.".into(),
            },
        );
        reg
    }

    // -----------------------------------------------------------------------
    // calculate_set_bonuses tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_calculate_set_bonuses_two_set_djinn() {
        // Unit 1 has flint (atk:3, def:2) and granite (def:5, hp:10) both Set.
        let state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Set),
            ("granite", 1, DjinnBattleState::Set),
        ]);
        let registry = make_test_registry();
        let (atk, def, mag, spd, hp, pp) = calculate_set_bonuses(1, &state, &registry);
        assert_eq!(atk, 3); // only flint contributes atk
        assert_eq!(def, 2 + 5); // flint(2) + granite(5)
        assert_eq!(mag, 0);
        assert_eq!(spd, 0);
        assert_eq!(hp, 10); // only granite contributes hp
        assert_eq!(pp, 0);
    }

    #[test]
    fn test_calculate_set_bonuses_ignores_non_set_djinn() {
        // Unit 1 has flint Set and forge Standby — only flint counted.
        let state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Set),
            ("forge", 1, DjinnBattleState::Standby),
        ]);
        let registry = make_test_registry();
        let (atk, def, mag, spd, hp, pp) = calculate_set_bonuses(1, &state, &registry);
        assert_eq!(atk, 3);
        assert_eq!(def, 2);
        assert_eq!(mag, 0);
        assert_eq!(spd, 0);
        assert_eq!(hp, 0);
        assert_eq!(pp, 0);
    }

    #[test]
    fn test_calculate_set_bonuses_ignores_other_unit() {
        // Flint belongs to unit 2, not unit 1.
        let state = make_djinn_state(&[("flint", 2, DjinnBattleState::Set)]);
        let registry = make_test_registry();
        let (atk, def, mag, spd, hp, pp) = calculate_set_bonuses(1, &state, &registry);
        assert_eq!(atk, 0);
        assert_eq!(def, 0);
        assert_eq!(mag, 0);
        assert_eq!(spd, 0);
        assert_eq!(hp, 0);
        assert_eq!(pp, 0);
    }

    #[test]
    fn test_calculate_set_bonuses_empty() {
        let state = make_djinn_state(&[]);
        let registry = make_test_registry();
        let (atk, def, mag, spd, hp, pp) = calculate_set_bonuses(1, &state, &registry);
        assert_eq!((atk, def, mag, spd, hp, pp), (0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn test_calculate_set_bonuses_unknown_djinn_id() {
        // A tracker with an ID not in the registry is silently skipped.
        let state = make_djinn_state(&[("unknown-djinn", 1, DjinnBattleState::Set)]);
        let registry = make_test_registry();
        let (atk, def, mag, spd, hp, pp) = calculate_set_bonuses(1, &state, &registry);
        assert_eq!((atk, def, mag, spd, hp, pp), (0, 0, 0, 0, 0, 0));
    }

    // -----------------------------------------------------------------------
    // get_granted_abilities tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_granted_abilities() {
        let state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Set),
            ("forge", 1, DjinnBattleState::Set),
        ]);
        let registry = make_test_registry();
        let abilities = get_granted_abilities(1, &state, &registry);
        assert_eq!(abilities.len(), 2);
        assert!(abilities.contains(&"earth-spike-damage".to_string()));
        assert!(abilities.contains(&"fireball".to_string()));
    }

    #[test]
    fn test_get_granted_abilities_ignores_standby() {
        let state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Set),
            ("forge", 1, DjinnBattleState::Standby),
        ]);
        let registry = make_test_registry();
        let abilities = get_granted_abilities(1, &state, &registry);
        assert_eq!(abilities.len(), 1);
        assert!(abilities.contains(&"earth-spike-damage".to_string()));
    }

    #[test]
    fn test_get_granted_abilities_empty_when_no_set() {
        let state = make_djinn_state(&[("flint", 1, DjinnBattleState::Standby)]);
        let registry = make_test_registry();
        let abilities = get_granted_abilities(1, &state, &registry);
        assert!(abilities.is_empty());
    }

    // -----------------------------------------------------------------------
    // summon_djinn_enhanced tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_summon_enhanced_single_damage() {
        let mut state = make_djinn_state(&[("flint", 1, DjinnBattleState::Standby)]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&["flint".into()], 5, &mut state, &registry).unwrap();
        assert_eq!(result.total_damage, 80);
        assert_eq!(result.total_healing, 0);
        assert!(result.status_inflicts.is_empty());
        assert!(result.stat_buffs.is_empty());
        // Djinn should be in Recovery with definition-based recovery turns.
        assert_eq!(state.trackers[0].state, DjinnBattleState::Recovery);
        assert_eq!(state.trackers[0].recovery_turns_remaining, 2);
    }

    #[test]
    fn test_summon_enhanced_aggregates_damage() {
        let mut state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Standby),
            ("forge", 1, DjinnBattleState::Standby),
        ]);
        let registry = make_test_registry();
        let result =
            summon_djinn_enhanced(&["flint".into(), "forge".into()], 5, &mut state, &registry)
                .unwrap();
        assert_eq!(result.total_damage, 80 + 120); // flint(80) + forge(120)
        assert_eq!(result.total_healing, 0);
    }

    #[test]
    fn test_summon_enhanced_heal_effect() {
        let mut state = make_djinn_state(&[("fizz", 1, DjinnBattleState::Standby)]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&["fizz".into()], 5, &mut state, &registry).unwrap();
        assert_eq!(result.total_damage, 0);
        assert_eq!(result.total_healing, 100);
    }

    #[test]
    fn test_summon_enhanced_buff_effect() {
        let mut state = make_djinn_state(&[("granite", 1, DjinnBattleState::Standby)]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&["granite".into()], 5, &mut state, &registry).unwrap();
        assert_eq!(result.total_damage, 0);
        assert_eq!(result.total_healing, 0);
        assert_eq!(result.stat_buffs.len(), 1);
        assert_eq!(result.stat_buffs[0], ("def".into(), 10));
    }

    #[test]
    fn test_summon_enhanced_status_inflict() {
        let mut state = make_djinn_state(&[("fever", 1, DjinnBattleState::Standby)]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&["fever".into()], 5, &mut state, &registry).unwrap();
        assert_eq!(result.total_damage, 0);
        assert_eq!(result.status_inflicts.len(), 1);
        assert_eq!(result.status_inflicts[0], ("burn".into(), 3));
    }

    #[test]
    fn test_summon_enhanced_mixed_effects() {
        // Summon flint (damage:80), fizz (heal:100), fever (status:burn/3).
        let mut state = make_djinn_state(&[
            ("flint", 1, DjinnBattleState::Standby),
            ("fizz", 1, DjinnBattleState::Standby),
            ("fever", 1, DjinnBattleState::Standby),
        ]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(
            &["flint".into(), "fizz".into(), "fever".into()],
            5,
            &mut state,
            &registry,
        )
        .unwrap();
        assert_eq!(result.total_damage, 80);
        assert_eq!(result.total_healing, 100);
        assert_eq!(result.status_inflicts.len(), 1);
        assert_eq!(result.status_inflicts[0], ("burn".into(), 3));
        // All three should be in Recovery.
        assert!(
            state
                .trackers
                .iter()
                .all(|t| t.state == DjinnBattleState::Recovery)
        );
    }

    #[test]
    fn test_summon_enhanced_uses_definition_recovery_turns() {
        // granite has recovery_turns: 3, forge has recovery_turns: 2.
        let mut state = make_djinn_state(&[
            ("granite", 1, DjinnBattleState::Standby),
            ("forge", 1, DjinnBattleState::Standby),
        ]);
        let registry = make_test_registry();
        let _result = summon_djinn_enhanced(
            &["granite".into(), "forge".into()],
            5,
            &mut state,
            &registry,
        )
        .unwrap();
        let granite_tracker = state
            .trackers
            .iter()
            .find(|t| t.djinn_id == "granite")
            .unwrap();
        let forge_tracker = state
            .trackers
            .iter()
            .find(|t| t.djinn_id == "forge")
            .unwrap();
        assert_eq!(granite_tracker.recovery_turns_remaining, 3);
        assert_eq!(forge_tracker.recovery_turns_remaining, 2);
    }

    #[test]
    fn test_summon_enhanced_rejects_empty() {
        let mut state = make_djinn_state(&[]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&[], 5, &mut state, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1-3 Djinn"));
    }

    #[test]
    fn test_summon_enhanced_rejects_set_state() {
        let mut state = make_djinn_state(&[("flint", 1, DjinnBattleState::Set)]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&["flint".into()], 5, &mut state, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be Standby"));
    }

    #[test]
    fn test_summon_enhanced_rejects_missing_definition() {
        let mut state = make_djinn_state(&[("unknown-djinn", 1, DjinnBattleState::Standby)]);
        let registry = make_test_registry();
        let result = summon_djinn_enhanced(&["unknown-djinn".into()], 5, &mut state, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no definition"));
    }
}
