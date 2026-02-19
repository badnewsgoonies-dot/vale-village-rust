//! Djinn battle mechanics.
//!
//! Ported from TypeScript `djinn.ts` and `djinnAbilities.ts`.

use crate::battle::types::{
    constants, DjinnBattleRes, DjinnBattleState, Element,
};

// ---------------------------------------------------------------------------
// Element compatibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementCompatibility {
    Same,
    Counter,
    Neutral,
}

pub fn element_compatibility(unit_element: Element, djinn_element: Element) -> ElementCompatibility {
    if unit_element == djinn_element { return ElementCompatibility::Same; }
    let is_counter = matches!(
        (unit_element, djinn_element),
        (Element::Venus, Element::Jupiter) | (Element::Jupiter, Element::Venus)
        | (Element::Mars, Element::Mercury) | (Element::Mercury, Element::Mars)
    );
    if is_counter { ElementCompatibility::Counter } else { ElementCompatibility::Neutral }
}

// ---------------------------------------------------------------------------
// Synergy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DjinnSynergy {
    pub atk_bonus: i32,
    pub def_bonus: i32,
    pub spd_bonus: i32,
    pub class_name: String,
    pub abilities_unlocked: Vec<String>,
}

pub fn calculate_djinn_synergy(set_elements: &[Element]) -> DjinnSynergy {
    if set_elements.is_empty() {
        return DjinnSynergy { atk_bonus: 0, def_bonus: 0, spd_bonus: 0, class_name: "Base".into(), abilities_unlocked: vec![] };
    }

    let mut counts = std::collections::HashMap::new();
    for &e in set_elements { *counts.entry(e).or_insert(0u32) += 1; }
    let unique = counts.len();
    let max_count = *counts.values().max().unwrap_or(&0);
    let primary = counts.iter().max_by_key(|(_, c)| *c).map(|(e, _)| *e).unwrap_or(Element::Neutral);

    match (set_elements.len(), unique, max_count) {
        (1, _, _) => DjinnSynergy { atk_bonus: 4, def_bonus: 3, spd_bonus: 0, class_name: "Adept".into(), abilities_unlocked: vec![] },
        (2, 1, _) => DjinnSynergy { atk_bonus: 8, def_bonus: 5, spd_bonus: 0, class_name: format!("{:?} Warrior", primary), abilities_unlocked: vec![] },
        (2, 2, _) => DjinnSynergy { atk_bonus: 5, def_bonus: 5, spd_bonus: 0, class_name: "Hybrid".into(), abilities_unlocked: vec![] },
        (3, 1, _) => DjinnSynergy { atk_bonus: 12, def_bonus: 8, spd_bonus: 0, class_name: format!("{:?} Adept", primary), abilities_unlocked: vec![format!("{:?}-Ultimate", primary)] },
        (3, 2, 2) => DjinnSynergy { atk_bonus: 8, def_bonus: 6, spd_bonus: 0, class_name: format!("{:?} Knight", primary), abilities_unlocked: vec!["Hybrid-Spell".into()] },
        (3, _, _) => DjinnSynergy { atk_bonus: 4, def_bonus: 4, spd_bonus: 4, class_name: "Mystic".into(), abilities_unlocked: vec!["Elemental Harmony".into()] },
        _ => DjinnSynergy { atk_bonus: 0, def_bonus: 0, spd_bonus: 0, class_name: "Base".into(), abilities_unlocked: vec![] },
    }
}

// ---------------------------------------------------------------------------
// State transitions
// ---------------------------------------------------------------------------

pub fn unleash_djinn(djinn_id: &str, current_turn: u32, djinn_state: &mut DjinnBattleRes) -> Result<(), String> {
    let tracker = djinn_state.trackers.iter_mut()
        .find(|t| t.djinn_id == djinn_id)
        .ok_or_else(|| format!("Djinn '{}' not found in battle", djinn_id))?;

    if tracker.state != DjinnBattleState::Set {
        return Err(format!("Djinn '{}' is {:?}, must be Set", djinn_id, tracker.state));
    }

    tracker.state = DjinnBattleState::Standby;
    tracker.last_activated_turn = current_turn;
    Ok(())
}

pub fn summon_djinn(djinn_ids: &[String], current_turn: u32, djinn_state: &mut DjinnBattleRes) -> Result<i32, String> {
    let count = djinn_ids.len();
    if count == 0 || count > 3 { return Err(format!("Summon requires 1-3 Djinn, got {}", count)); }

    for id in djinn_ids {
        let t = djinn_state.trackers.iter().find(|t| t.djinn_id == *id)
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

    Ok(match count { 1 => constants::SUMMON_DAMAGE_1, 2 => constants::SUMMON_DAMAGE_2, 3 => constants::SUMMON_DAMAGE_3, _ => unreachable!() })
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

pub fn get_standby_djinn(unit_id: u32, djinn_state: &DjinnBattleRes) -> Vec<String> {
    djinn_state.trackers.iter()
        .filter(|t| t.owner_unit_id == unit_id && t.state == DjinnBattleState::Standby)
        .map(|t| t.djinn_id.clone()).collect()
}

pub fn get_set_djinn(unit_id: u32, djinn_state: &DjinnBattleRes) -> Vec<String> {
    djinn_state.trackers.iter()
        .filter(|t| t.owner_unit_id == unit_id && t.state == DjinnBattleState::Set)
        .map(|t| t.djinn_id.clone()).collect()
}

pub fn can_unleash(djinn_id: &str, djinn_state: &DjinnBattleRes) -> bool {
    djinn_state.trackers.iter().any(|t| t.djinn_id == djinn_id && t.state == DjinnBattleState::Set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::types::DjinnTracker;

    fn make_djinn_state(djinn: &[(&str, u32, DjinnBattleState)]) -> DjinnBattleRes {
        DjinnBattleRes {
            trackers: djinn.iter().map(|(id, owner, state)| DjinnTracker {
                djinn_id: id.to_string(), state: *state, owner_unit_id: *owner,
                last_activated_turn: 0, recovery_turns_remaining: 0,
            }).collect(),
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
        assert!(state.trackers.iter().all(|t| t.state == DjinnBattleState::Recovery));
    }

    #[test]
    fn test_recovery_tick() {
        let mut state = DjinnBattleRes {
            trackers: vec![DjinnTracker {
                djinn_id: "flint".into(), state: DjinnBattleState::Recovery,
                owner_unit_id: 1, last_activated_turn: 3, recovery_turns_remaining: 2,
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
}
