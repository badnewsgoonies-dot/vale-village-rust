use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::components::stats::Element;

// ---------------------------------------------------------------------------
// Growth rates — how much each stat increases per level
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthRates {
    pub hp: i32,
    pub pp: i32,
    pub atk: i32,
    pub def: i32,
    pub mag: i32,
    pub spd: i32,
}

// ---------------------------------------------------------------------------
// Unit definition — a template for spawning player characters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityUnlock {
    pub ability_id: String,
    pub unlock_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitDefinition {
    pub id: String,
    pub name: String,
    pub element: Element,
    pub role: String,
    pub base_hp: i32,
    pub base_pp: i32,
    pub base_atk: i32,
    pub base_def: i32,
    pub base_mag: i32,
    pub base_spd: i32,
    pub growth: GrowthRates,
    pub abilities: Vec<AbilityUnlock>,
    pub mana_contribution: u8,
    pub description: String,
}

/// Build the registry of all 10 playable unit definitions.
pub fn build_unit_registry() -> HashMap<String, UnitDefinition> {
    let mut m = HashMap::new();

    // ----- Starter: Adept (Venus, Defensive Tank) -----
    m.insert(
        "adept".into(),
        UnitDefinition {
            id: "adept".into(),
            name: "Adept".into(),
            element: Element::Venus,
            role: "Defensive Tank".into(),
            base_hp: 120,
            base_pp: 15,
            base_atk: 14,
            base_def: 16,
            base_mag: 8,
            base_spd: 10,
            growth: GrowthRates {
                hp: 25,
                pp: 4,
                atk: 3,
                def: 4,
                mag: 2,
                spd: 1,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "earth-spike-damage".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "focus-strike-neutral".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "stone-skin-utility".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "ice-lance-damage".into(),
                    unlock_level: 5,
                },
                AbilityUnlock {
                    ability_id: "aqua-heal-utility".into(),
                    unlock_level: 6,
                },
                AbilityUnlock {
                    ability_id: "quake".into(),
                    unlock_level: 11,
                },
            ],
            mana_contribution: 1,
            description: "A sturdy Earth adept who breaks through enemy defenses".into(),
        },
    );

    // ----- War Mage (Mars, Elemental Mage) -----
    m.insert(
        "war-mage".into(),
        UnitDefinition {
            id: "war-mage".into(),
            name: "War Mage".into(),
            element: Element::Mars,
            role: "Elemental Mage".into(),
            base_hp: 80,
            base_pp: 25,
            base_atk: 10,
            base_def: 8,
            base_mag: 18,
            base_spd: 12,
            growth: GrowthRates {
                hp: 15,
                pp: 6,
                atk: 2,
                def: 2,
                mag: 5,
                spd: 2,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "flame-burst-damage".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "fire-ward-utility".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "gale-force-damage".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "wind-barrier-utility".into(),
                    unlock_level: 4,
                },
                AbilityUnlock {
                    ability_id: "focus-strike-neutral".into(),
                    unlock_level: 5,
                },
                AbilityUnlock {
                    ability_id: "fireball".into(),
                    unlock_level: 7,
                },
                AbilityUnlock {
                    ability_id: "flare".into(),
                    unlock_level: 11,
                },
            ],
            mana_contribution: 2,
            description: "A fire mage who burns enemies with powerful psynergy".into(),
        },
    );

    // ----- Mystic (Mercury, Healer) -----
    m.insert(
        "mystic".into(),
        UnitDefinition {
            id: "mystic".into(),
            name: "Mystic".into(),
            element: Element::Mercury,
            role: "Healer".into(),
            base_hp: 90,
            base_pp: 30,
            base_atk: 8,
            base_def: 10,
            base_mag: 16,
            base_spd: 11,
            growth: GrowthRates {
                hp: 18,
                pp: 7,
                atk: 1,
                def: 2,
                mag: 4,
                spd: 2,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "ice-lance-damage".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "aqua-heal-utility".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "earth-spike-damage".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "stone-skin-utility".into(),
                    unlock_level: 4,
                },
                AbilityUnlock {
                    ability_id: "focus-strike-neutral".into(),
                    unlock_level: 5,
                },
                AbilityUnlock {
                    ability_id: "heal".into(),
                    unlock_level: 7,
                },
                AbilityUnlock {
                    ability_id: "freeze-blast".into(),
                    unlock_level: 8,
                },
                AbilityUnlock {
                    ability_id: "party-heal".into(),
                    unlock_level: 11,
                },
            ],
            mana_contribution: 2,
            description: "A water mystic who heals allies and freezes enemies".into(),
        },
    );

    // ----- Ranger (Jupiter, Rogue Assassin) -----
    m.insert(
        "ranger".into(),
        UnitDefinition {
            id: "ranger".into(),
            name: "Ranger".into(),
            element: Element::Jupiter,
            role: "Rogue Assassin".into(),
            base_hp: 85,
            base_pp: 20,
            base_atk: 16,
            base_def: 9,
            base_mag: 10,
            base_spd: 18,
            growth: GrowthRates {
                hp: 16,
                pp: 5,
                atk: 4,
                def: 2,
                mag: 2,
                spd: 4,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "gale-force-damage".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "wind-barrier-utility".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "flame-burst-damage".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "fire-ward-utility".into(),
                    unlock_level: 4,
                },
                AbilityUnlock {
                    ability_id: "focus-strike-neutral".into(),
                    unlock_level: 5,
                },
                AbilityUnlock {
                    ability_id: "gust".into(),
                    unlock_level: 7,
                },
                AbilityUnlock {
                    ability_id: "blind".into(),
                    unlock_level: 8,
                },
                AbilityUnlock {
                    ability_id: "chain-lightning".into(),
                    unlock_level: 12,
                },
            ],
            mana_contribution: 1,
            description: "A swift wind ranger who strikes with precision and blinds foes".into(),
        },
    );

    // ----- Sentinel (Venus, Support Buffer) -----
    m.insert(
        "sentinel".into(),
        UnitDefinition {
            id: "sentinel".into(),
            name: "Sentinel".into(),
            element: Element::Venus,
            role: "Support Buffer".into(),
            base_hp: 110,
            base_pp: 18,
            base_atk: 12,
            base_def: 18,
            base_mag: 9,
            base_spd: 9,
            growth: GrowthRates {
                hp: 22,
                pp: 4,
                atk: 2,
                def: 5,
                mag: 2,
                spd: 1,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "boost-def".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "guard-break".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "quake".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "heavy-strike".into(),
                    unlock_level: 5,
                },
            ],
            mana_contribution: 1,
            description: "A defensive sentinel who protects allies and breaks enemy guards".into(),
        },
    );

    // ----- Stormcaller (Jupiter, AoE Mage) -----
    m.insert(
        "stormcaller".into(),
        UnitDefinition {
            id: "stormcaller".into(),
            name: "Stormcaller".into(),
            element: Element::Jupiter,
            role: "AoE Mage".into(),
            base_hp: 75,
            base_pp: 28,
            base_atk: 9,
            base_def: 7,
            base_mag: 20,
            base_spd: 15,
            growth: GrowthRates {
                hp: 14,
                pp: 7,
                atk: 1,
                def: 1,
                mag: 6,
                spd: 3,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "gust".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "chain-lightning".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "blind".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "paralyze-shock".into(),
                    unlock_level: 8,
                },
            ],
            mana_contribution: 3,
            description: "A storm caller who unleashes chain lightning on all enemies".into(),
        },
    );

    // ----- Blaze (Mars, Balanced Warrior) -----
    m.insert(
        "blaze".into(),
        UnitDefinition {
            id: "blaze".into(),
            name: "Blaze".into(),
            element: Element::Mars,
            role: "Balanced Warrior".into(),
            base_hp: 95,
            base_pp: 22,
            base_atk: 15,
            base_def: 11,
            base_mag: 14,
            base_spd: 13,
            growth: GrowthRates {
                hp: 18,
                pp: 5,
                atk: 3,
                def: 2,
                mag: 4,
                spd: 3,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "heavy-strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "fireball".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "burn-touch".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "boost-atk".into(),
                    unlock_level: 5,
                },
            ],
            mana_contribution: 2,
            description: "A versatile Mars warrior who balances physical and magical combat".into(),
        },
    );

    // ----- Karis (Mercury, Versatile Scholar) -----
    m.insert(
        "karis".into(),
        UnitDefinition {
            id: "karis".into(),
            name: "Karis".into(),
            element: Element::Mercury,
            role: "Versatile Scholar".into(),
            base_hp: 88,
            base_pp: 28,
            base_atk: 7,
            base_def: 9,
            base_mag: 17,
            base_spd: 12,
            growth: GrowthRates {
                hp: 17,
                pp: 6,
                atk: 1,
                def: 2,
                mag: 5,
                spd: 2,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "ice-shard".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "heal".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "freeze-blast".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "party-heal".into(),
                    unlock_level: 4,
                },
            ],
            mana_contribution: 2,
            description: "A scholarly mage with mastery of ice magic and healing arts".into(),
        },
    );

    // ----- Tyrell (Mars, Pure DPS) -----
    m.insert(
        "tyrell".into(),
        UnitDefinition {
            id: "tyrell".into(),
            name: "Tyrell".into(),
            element: Element::Mars,
            role: "Pure DPS".into(),
            base_hp: 92,
            base_pp: 18,
            base_atk: 18,
            base_def: 10,
            base_mag: 12,
            base_spd: 16,
            growth: GrowthRates {
                hp: 17,
                pp: 4,
                atk: 5,
                def: 2,
                mag: 3,
                spd: 4,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "precise-jab".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "heavy-strike".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "fireball".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "burn-touch".into(),
                    unlock_level: 4,
                },
            ],
            mana_contribution: 1,
            description: "A relentless damage dealer who excels in both physical and fire attacks"
                .into(),
        },
    );

    // ----- Felix (Venus, Master Warrior) -----
    m.insert(
        "felix".into(),
        UnitDefinition {
            id: "felix".into(),
            name: "Felix".into(),
            element: Element::Venus,
            role: "Master Warrior".into(),
            base_hp: 125,
            base_pp: 16,
            base_atk: 16,
            base_def: 18,
            base_mag: 9,
            base_spd: 11,
            growth: GrowthRates {
                hp: 26,
                pp: 4,
                atk: 4,
                def: 5,
                mag: 2,
                spd: 2,
            },
            abilities: vec![
                AbilityUnlock {
                    ability_id: "strike".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "guard-break".into(),
                    unlock_level: 1,
                },
                AbilityUnlock {
                    ability_id: "heavy-strike".into(),
                    unlock_level: 2,
                },
                AbilityUnlock {
                    ability_id: "quake".into(),
                    unlock_level: 3,
                },
                AbilityUnlock {
                    ability_id: "boost-def".into(),
                    unlock_level: 4,
                },
            ],
            mana_contribution: 1,
            description: "A legendary earth warrior with unmatched physical prowess".into(),
        },
    );

    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_unit_count() {
        let registry = build_unit_registry();
        assert_eq!(
            registry.len(),
            10,
            "Expected exactly 10 units, got {}",
            registry.len()
        );
    }

    #[test]
    fn test_all_units_have_abilities() {
        let registry = build_unit_registry();
        for (id, unit) in &registry {
            assert!(
                !unit.abilities.is_empty(),
                "Unit '{}' should have at least 1 ability unlock",
                id
            );
        }
    }

    #[test]
    fn test_adept_is_venus() {
        let registry = build_unit_registry();
        let adept = registry
            .get("adept")
            .expect("Adept unit should exist in registry");
        assert_eq!(
            adept.element,
            Element::Venus,
            "Adept should be Venus element"
        );
    }

    #[test]
    fn test_growth_rates_positive() {
        let registry = build_unit_registry();
        for (id, unit) in &registry {
            let g = &unit.growth;
            assert!(g.hp >= 0, "Unit '{}' has negative hp growth: {}", id, g.hp);
            assert!(g.pp >= 0, "Unit '{}' has negative pp growth: {}", id, g.pp);
            assert!(
                g.atk >= 0,
                "Unit '{}' has negative atk growth: {}",
                id,
                g.atk
            );
            assert!(
                g.def >= 0,
                "Unit '{}' has negative def growth: {}",
                id,
                g.def
            );
            assert!(
                g.mag >= 0,
                "Unit '{}' has negative mag growth: {}",
                id,
                g.mag
            );
            assert!(
                g.spd >= 0,
                "Unit '{}' has negative spd growth: {}",
                id,
                g.spd
            );
        }
    }

    #[test]
    fn test_unique_unit_ids() {
        let registry = build_unit_registry();
        let mut seen = HashSet::new();
        for (id, unit) in &registry {
            assert_eq!(
                id, &unit.id,
                "Registry key '{}' should match unit id '{}'",
                id, unit.id
            );
            assert!(
                seen.insert(unit.id.clone()),
                "Duplicate unit id found: '{}'",
                unit.id
            );
        }
    }
}
