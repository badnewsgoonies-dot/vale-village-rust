use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::components::stats::Element;

// ---------------------------------------------------------------------------
// Enemy definition — a template for spawning enemies
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyAbility {
    pub ability_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyDefinition {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub element: Element,
    pub hp: i32,
    pub pp: i32,
    pub atk: i32,
    pub def: i32,
    pub mag: i32,
    pub spd: i32,
    pub abilities: Vec<EnemyAbility>,
    pub base_xp: u32,
    pub base_gold: u32,
    /// Encounter tier: 1 = early game, 2 = mid game, 3 = late game, 4 = boss.
    pub tier: u8,
    /// Item drops: Vec of (item_id, drop_chance) where drop_chance is 0.0–1.0.
    pub drop_table: Vec<(String, f32)>,
}

/// Helper to create an enemy ability entry.
fn ea(id: &str) -> EnemyAbility {
    EnemyAbility {
        ability_id: id.into(),
    }
}

/// Helper to create a drop table entry (item_id, drop_chance).
fn drop(id: &str, chance: f32) -> (String, f32) {
    (id.into(), chance)
}

/// Build the registry of all enemy definitions (65 types).
pub fn build_enemy_registry() -> HashMap<String, EnemyDefinition> {
    let mut m = HashMap::new();

    let enemies = vec![
        // ===== ENSLAVED BEASTS - Tier 1 (5) =====
        EnemyDefinition {
            id: "mercury-slime".into(),
            name: "Mercury Slime".into(),
            level: 1,
            element: Element::Mercury,
            hp: 40,
            pp: 8,
            atk: 4,
            def: 5,
            mag: 6,
            spd: 5,
            abilities: vec![ea("strike"), ea("ice-shard")],
            base_xp: 12,
            base_gold: 6,
            tier: 1,
            drop_table: vec![drop("herb", 0.25)],
        },
        EnemyDefinition {
            id: "venus-wolf".into(),
            name: "Earthbound Wolf".into(),
            level: 1,
            element: Element::Venus,
            hp: 55,
            pp: 8,
            atk: 11,
            def: 7,
            mag: 3,
            spd: 11,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("earth-spike-damage")],
            base_xp: 16,
            base_gold: 8,
            tier: 1,
            drop_table: vec![drop("herb", 0.20)],
        },
        EnemyDefinition {
            id: "mars-bandit".into(),
            name: "Flame Bandit".into(),
            level: 2,
            element: Element::Mars,
            hp: 60,
            pp: 12,
            atk: 13,
            def: 6,
            mag: 8,
            spd: 10,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("fireball")],
            base_xp: 20,
            base_gold: 12,
            tier: 1,
            drop_table: vec![drop("herb", 0.20), drop("antidote", 0.10)],
        },
        EnemyDefinition {
            id: "jupiter-sprite".into(),
            name: "Wind Sprite".into(),
            level: 2,
            element: Element::Jupiter,
            hp: 45,
            pp: 15,
            atk: 5,
            def: 5,
            mag: 14,
            spd: 17,
            abilities: vec![ea("gust"), ea("blind"), ea("paralyze-shock")],
            base_xp: 18,
            base_gold: 10,
            tier: 1,
            drop_table: vec![drop("jupiter-zephyr-scroll", 0.08)],
        },
        EnemyDefinition {
            id: "venus-beetle".into(),
            name: "Stone Beetle".into(),
            level: 2,
            element: Element::Venus,
            hp: 80,
            pp: 8,
            atk: 8,
            def: 15,
            mag: 3,
            spd: 6,
            abilities: vec![ea("strike"), ea("guard-break"), ea("earth-spike-damage")],
            base_xp: 22,
            base_gold: 12,
            tier: 1,
            drop_table: Vec::new(),
        },
        // ===== ENSLAVED BEASTS - Wolf Pack (3) =====
        EnemyDefinition {
            id: "mars-wolf".into(),
            name: "Flame Wolf".into(),
            level: 2,
            element: Element::Mars,
            hp: 58,
            pp: 10,
            atk: 12,
            def: 6,
            mag: 5,
            spd: 13,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("burn-touch")],
            base_xp: 18,
            base_gold: 9,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "mercury-wolf".into(),
            name: "Frost Wolf".into(),
            level: 2,
            element: Element::Mercury,
            hp: 56,
            pp: 12,
            atk: 10,
            def: 7,
            mag: 6,
            spd: 14,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("freeze-blast")],
            base_xp: 18,
            base_gold: 9,
            tier: 1,
            drop_table: vec![drop("mercury-vial", 0.08)],
        },
        EnemyDefinition {
            id: "jupiter-wolf".into(),
            name: "Storm Wolf".into(),
            level: 2,
            element: Element::Jupiter,
            hp: 52,
            pp: 11,
            atk: 11,
            def: 6,
            mag: 7,
            spd: 16,
            abilities: vec![ea("strike"), ea("precise-jab"), ea("gust")],
            base_xp: 18,
            base_gold: 9,
            tier: 1,
            drop_table: Vec::new(),
        },
        // ===== ENSLAVED BEASTS - Bears (4) =====
        EnemyDefinition {
            id: "venus-bear".into(),
            name: "Mountain Bear".into(),
            level: 4,
            element: Element::Venus,
            hp: 110,
            pp: 12,
            atk: 14,
            def: 18,
            mag: 6,
            spd: 8,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("quake")],
            base_xp: 35,
            base_gold: 18,
            tier: 1,
            drop_table: vec![drop("herb", 0.15), drop("power_bread", 0.03)],
        },
        EnemyDefinition {
            id: "mars-bear".into(),
            name: "Inferno Bear".into(),
            level: 4,
            element: Element::Mars,
            hp: 105,
            pp: 14,
            atk: 16,
            def: 16,
            mag: 8,
            spd: 9,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("fireball")],
            base_xp: 35,
            base_gold: 18,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "mercury-bear".into(),
            name: "Glacier Bear".into(),
            level: 4,
            element: Element::Mercury,
            hp: 115,
            pp: 13,
            atk: 13,
            def: 19,
            mag: 7,
            spd: 7,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("ice-shard")],
            base_xp: 35,
            base_gold: 18,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "jupiter-bear".into(),
            name: "Thunder Bear".into(),
            level: 4,
            element: Element::Jupiter,
            hp: 100,
            pp: 15,
            atk: 15,
            def: 15,
            mag: 10,
            spd: 12,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("gust")],
            base_xp: 35,
            base_gold: 18,
            tier: 1,
            drop_table: Vec::new(),
        },
        // ===== COUNTER-STRATEGY ENEMIES - Support Roles (7) =====
        EnemyDefinition {
            id: "frost-mystic".into(),
            name: "Frost Mystic".into(),
            level: 2,
            element: Element::Mercury,
            hp: 200,
            pp: 20,
            atk: 10,
            def: 8,
            mag: 12,
            spd: 11,
            abilities: vec![ea("strike"), ea("ice-shard"), ea("heal")],
            base_xp: 22,
            base_gold: 12,
            tier: 1,
            drop_table: vec![drop("mercury-mist-elixir", 0.10)],
        },
        EnemyDefinition {
            id: "gale-priest".into(),
            name: "Gale Priest".into(),
            level: 2,
            element: Element::Jupiter,
            hp: 180,
            pp: 22,
            atk: 8,
            def: 7,
            mag: 14,
            spd: 13,
            abilities: vec![ea("gust"), ea("heal"), ea("blind")],
            base_xp: 24,
            base_gold: 14,
            tier: 1,
            drop_table: vec![drop("antidote", 0.15)],
        },
        EnemyDefinition {
            id: "stone-guardian".into(),
            name: "Stone Guardian".into(),
            level: 3,
            element: Element::Venus,
            hp: 350,
            pp: 10,
            atk: 12,
            def: 20,
            mag: 5,
            spd: 6,
            abilities: vec![
                ea("strike"),
                ea("guard-break"),
                ea("earth-spike-damage"),
                ea("boost-def"),
            ],
            base_xp: 30,
            base_gold: 16,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "ember-cleric".into(),
            name: "Ember Cleric".into(),
            level: 3,
            element: Element::Mars,
            hp: 190,
            pp: 18,
            atk: 9,
            def: 8,
            mag: 11,
            spd: 10,
            abilities: vec![ea("strike"), ea("fireball"), ea("heal")],
            base_xp: 26,
            base_gold: 14,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "earth-shaman".into(),
            name: "Earth Shaman".into(),
            level: 4,
            element: Element::Venus,
            hp: 220,
            pp: 25,
            atk: 10,
            def: 14,
            mag: 16,
            spd: 9,
            abilities: vec![
                ea("quake"),
                ea("earth-spike-damage"),
                ea("boost-def"),
                ea("heal"),
            ],
            base_xp: 45,
            base_gold: 22,
            tier: 1,
            drop_table: vec![drop("potion", 0.12)],
        },
        EnemyDefinition {
            id: "tide-enchanter".into(),
            name: "Tide Enchanter".into(),
            level: 4,
            element: Element::Mercury,
            hp: 240,
            pp: 30,
            atk: 11,
            def: 13,
            mag: 18,
            spd: 10,
            abilities: vec![
                ea("ice-shard"),
                ea("freeze-blast"),
                ea("boost-def"),
                ea("heal"),
            ],
            base_xp: 50,
            base_gold: 24,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "frost-oracle".into(),
            name: "Frost Oracle".into(),
            level: 5,
            element: Element::Mercury,
            hp: 200,
            pp: 35,
            atk: 10,
            def: 12,
            mag: 20,
            spd: 11,
            abilities: vec![
                ea("freeze-blast"),
                ea("ice-shard"),
                ea("heal"),
                ea("party-heal"),
            ],
            base_xp: 55,
            base_gold: 26,
            tier: 1,
            drop_table: vec![drop("mercury-mist-elixir", 0.12), drop("lucky_medal", 0.02)],
        },
        // ===== NEW BEASTS - Tier 1 Basics (4) =====
        EnemyDefinition {
            id: "mushroom".into(),
            name: "Mushroom".into(),
            level: 1,
            element: Element::Venus,
            hp: 35,
            pp: 5,
            atk: 5,
            def: 4,
            mag: 3,
            spd: 4,
            abilities: vec![ea("strike"), ea("poison-strike")],
            base_xp: 10,
            base_gold: 8,
            tier: 1,
            drop_table: vec![drop("herb", 0.30)],
        },
        EnemyDefinition {
            id: "cave-bat".into(),
            name: "Cave Bat".into(),
            level: 1,
            element: Element::Jupiter,
            hp: 30,
            pp: 6,
            atk: 6,
            def: 3,
            mag: 4,
            spd: 14,
            abilities: vec![ea("strike"), ea("drain-kiss")],
            base_xp: 11,
            base_gold: 7,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "rat-king".into(),
            name: "Rat King".into(),
            level: 2,
            element: Element::Venus,
            hp: 65,
            pp: 8,
            atk: 10,
            def: 6,
            mag: 4,
            spd: 12,
            abilities: vec![ea("strike"), ea("venom-bite"), ea("precise-jab")],
            base_xp: 18,
            base_gold: 14,
            tier: 1,
            drop_table: vec![drop("antidote", 0.20)],
        },
        EnemyDefinition {
            id: "wild-boar".into(),
            name: "Wild Boar".into(),
            level: 3,
            element: Element::Venus,
            hp: 90,
            pp: 5,
            atk: 14,
            def: 10,
            mag: 2,
            spd: 9,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("crushing-blow")],
            base_xp: 25,
            base_gold: 18,
            tier: 1,
            drop_table: vec![drop("herb", 0.15)],
        },
        // ===== SLAVERS - Tier 1 Scouts (4) =====
        EnemyDefinition {
            id: "earth-scout".into(),
            name: "Earth Scout".into(),
            level: 1,
            element: Element::Venus,
            hp: 50,
            pp: 10,
            atk: 9,
            def: 8,
            mag: 5,
            spd: 8,
            abilities: vec![ea("strike"), ea("guard-break"), ea("earth-spike-damage")],
            base_xp: 15,
            base_gold: 10,
            tier: 1,
            drop_table: vec![drop("herb", 0.20)],
        },
        EnemyDefinition {
            id: "flame-scout".into(),
            name: "Flame Scout".into(),
            level: 1,
            element: Element::Mars,
            hp: 45,
            pp: 12,
            atk: 10,
            def: 6,
            mag: 8,
            spd: 10,
            abilities: vec![ea("strike"), ea("fireball")],
            base_xp: 15,
            base_gold: 10,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "frost-scout".into(),
            name: "Frost Scout".into(),
            level: 1,
            element: Element::Mercury,
            hp: 48,
            pp: 11,
            atk: 8,
            def: 7,
            mag: 7,
            spd: 9,
            abilities: vec![ea("strike"), ea("ice-shard")],
            base_xp: 15,
            base_gold: 10,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "gale-scout".into(),
            name: "Gale Scout".into(),
            level: 1,
            element: Element::Jupiter,
            hp: 42,
            pp: 13,
            atk: 9,
            def: 6,
            mag: 9,
            spd: 12,
            abilities: vec![ea("strike"), ea("gust")],
            base_xp: 15,
            base_gold: 10,
            tier: 1,
            drop_table: Vec::new(),
        },
        // ===== SLAVERS - Tier 2 Soldiers (4) =====
        EnemyDefinition {
            id: "terra-soldier".into(),
            name: "Terra Soldier".into(),
            level: 3,
            element: Element::Venus,
            hp: 85,
            pp: 15,
            atk: 14,
            def: 13,
            mag: 7,
            spd: 9,
            abilities: vec![ea("strike"), ea("heavy-strike"), ea("quake")],
            base_xp: 28,
            base_gold: 16,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "blaze-soldier".into(),
            name: "Blaze Soldier".into(),
            level: 3,
            element: Element::Mars,
            hp: 75,
            pp: 18,
            atk: 15,
            def: 10,
            mag: 12,
            spd: 11,
            abilities: vec![ea("strike"), ea("fireball"), ea("burn-touch")],
            base_xp: 28,
            base_gold: 16,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "tide-soldier".into(),
            name: "Tide Soldier".into(),
            level: 3,
            element: Element::Mercury,
            hp: 80,
            pp: 16,
            atk: 12,
            def: 12,
            mag: 10,
            spd: 10,
            abilities: vec![ea("strike"), ea("ice-shard"), ea("freeze-blast")],
            base_xp: 28,
            base_gold: 16,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "wind-soldier".into(),
            name: "Wind Soldier".into(),
            level: 3,
            element: Element::Jupiter,
            hp: 70,
            pp: 20,
            atk: 13,
            def: 9,
            mag: 13,
            spd: 14,
            abilities: vec![ea("strike"), ea("gust"), ea("paralyze-shock")],
            base_xp: 28,
            base_gold: 16,
            tier: 1,
            drop_table: Vec::new(),
        },
        // ===== SLAVERS - Tier 3 Captains (4) =====
        EnemyDefinition {
            id: "stone-captain".into(),
            name: "Stone Captain".into(),
            level: 5,
            element: Element::Venus,
            hp: 130,
            pp: 20,
            atk: 18,
            def: 18,
            mag: 10,
            spd: 10,
            abilities: vec![
                ea("strike"),
                ea("heavy-strike"),
                ea("quake"),
                ea("boost-def"),
            ],
            base_xp: 50,
            base_gold: 28,
            tier: 1,
            drop_table: vec![drop("potion", 0.15), drop("power_bread", 0.05)],
        },
        EnemyDefinition {
            id: "inferno-captain".into(),
            name: "Inferno Captain".into(),
            level: 5,
            element: Element::Mars,
            hp: 115,
            pp: 25,
            atk: 20,
            def: 14,
            mag: 16,
            spd: 12,
            abilities: vec![
                ea("strike"),
                ea("fireball"),
                ea("burn-touch"),
                ea("boost-atk"),
            ],
            base_xp: 50,
            base_gold: 28,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "glacier-captain".into(),
            name: "Glacier Captain".into(),
            level: 5,
            element: Element::Mercury,
            hp: 125,
            pp: 22,
            atk: 16,
            def: 16,
            mag: 14,
            spd: 11,
            abilities: vec![
                ea("strike"),
                ea("ice-shard"),
                ea("freeze-blast"),
                ea("heal"),
            ],
            base_xp: 50,
            base_gold: 28,
            tier: 1,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "thunder-captain".into(),
            name: "Thunder Captain".into(),
            level: 5,
            element: Element::Jupiter,
            hp: 110,
            pp: 28,
            atk: 17,
            def: 13,
            mag: 18,
            spd: 15,
            abilities: vec![ea("strike"), ea("gust"), ea("paralyze-shock"), ea("blind")],
            base_xp: 50,
            base_gold: 28,
            tier: 1,
            drop_table: vec![drop("jupiter-lightning-flask", 0.08)],
        },
        // ===== SLAVERS - Tier 4 Commanders (4) =====
        EnemyDefinition {
            id: "mountain-commander".into(),
            name: "Mountain Commander".into(),
            level: 7,
            element: Element::Venus,
            hp: 180,
            pp: 28,
            atk: 22,
            def: 24,
            mag: 14,
            spd: 11,
            abilities: vec![
                ea("strike"),
                ea("heavy-strike"),
                ea("quake"),
                ea("guard-break"),
                ea("boost-def"),
            ],
            base_xp: 75,
            base_gold: 40,
            tier: 2,
            drop_table: vec![drop("potion", 0.20), drop("power_bread", 0.05)],
        },
        EnemyDefinition {
            id: "fire-commander".into(),
            name: "Fire Commander".into(),
            level: 7,
            element: Element::Mars,
            hp: 160,
            pp: 35,
            atk: 24,
            def: 18,
            mag: 22,
            spd: 13,
            abilities: vec![
                ea("strike"),
                ea("fireball"),
                ea("burn-touch"),
                ea("boost-atk"),
                ea("weaken-def"),
            ],
            base_xp: 75,
            base_gold: 40,
            tier: 2,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "storm-commander".into(),
            name: "Storm Commander".into(),
            level: 7,
            element: Element::Mercury,
            hp: 170,
            pp: 30,
            atk: 20,
            def: 20,
            mag: 20,
            spd: 12,
            abilities: vec![
                ea("strike"),
                ea("ice-shard"),
                ea("freeze-blast"),
                ea("heal"),
                ea("boost-def"),
            ],
            base_xp: 75,
            base_gold: 40,
            tier: 2,
            drop_table: vec![drop("mercury-water-of-life", 0.08)],
        },
        EnemyDefinition {
            id: "gale-commander".into(),
            name: "Gale Commander".into(),
            level: 7,
            element: Element::Jupiter,
            hp: 150,
            pp: 35,
            atk: 21,
            def: 16,
            mag: 24,
            spd: 16,
            abilities: vec![
                ea("strike"),
                ea("gust"),
                ea("chain-lightning"),
                ea("blind"),
                ea("paralyze-shock"),
            ],
            base_xp: 75,
            base_gold: 40,
            tier: 2,
            drop_table: vec![drop("jupiter-hermes-water", 0.10)],
        },
        // ===== SUPPORT ENEMIES - Heralds/Wardens (2) =====
        EnemyDefinition {
            id: "terra-warden".into(),
            name: "Terra Warden".into(),
            level: 6,
            element: Element::Venus,
            hp: 260,
            pp: 28,
            atk: 16,
            def: 16,
            mag: 14,
            spd: 9,
            abilities: vec![
                ea("strike"),
                ea("quake"),
                ea("boost-atk"),
                ea("boost-def"),
                ea("heal"),
            ],
            base_xp: 58,
            base_gold: 28,
            tier: 2,
            drop_table: vec![drop("potion", 0.15)],
        },
        EnemyDefinition {
            id: "flame-herald".into(),
            name: "Flame Herald".into(),
            level: 7,
            element: Element::Mars,
            hp: 220,
            pp: 32,
            atk: 18,
            def: 14,
            mag: 20,
            spd: 13,
            abilities: vec![
                ea("strike"),
                ea("fireball"),
                ea("burn-touch"),
                ea("boost-atk"),
                ea("weaken-def"),
            ],
            base_xp: 70,
            base_gold: 35,
            tier: 2,
            drop_table: Vec::new(),
        },
        // ===== NEW MONSTERS - Tier 2 Mid-game (4) =====
        EnemyDefinition {
            id: "stone-golem".into(),
            name: "Stone Golem".into(),
            level: 6,
            element: Element::Venus,
            hp: 280,
            pp: 10,
            atk: 20,
            def: 26,
            mag: 6,
            spd: 5,
            abilities: vec![
                ea("strike"),
                ea("heavy-strike"),
                ea("quake"),
                ea("guard-break"),
            ],
            base_xp: 65,
            base_gold: 55,
            tier: 2,
            drop_table: vec![drop("potion", 0.12)],
        },
        EnemyDefinition {
            id: "dire-wolf".into(),
            name: "Dire Wolf".into(),
            level: 7,
            element: Element::Venus,
            hp: 200,
            pp: 12,
            atk: 24,
            def: 16,
            mag: 8,
            spd: 18,
            abilities: vec![
                ea("strike"),
                ea("crushing-blow"),
                ea("venom-bite"),
                ea("precise-jab"),
            ],
            base_xp: 80,
            base_gold: 60,
            tier: 2,
            drop_table: Vec::new(),
        },
        EnemyDefinition {
            id: "dark-mage".into(),
            name: "Dark Mage".into(),
            level: 8,
            element: Element::Jupiter,
            hp: 180,
            pp: 45,
            atk: 14,
            def: 14,
            mag: 30,
            spd: 15,
            abilities: vec![
                ea("chain-lightning"),
                ea("blind"),
                ea("curse"),
                ea("enfeeble"),
                ea("heal"),
            ],
            base_xp: 95,
            base_gold: 72,
            tier: 2,
            drop_table: vec![drop("mercury-mist-elixir", 0.10)],
        },
        EnemyDefinition {
            id: "sand-scorpion".into(),
            name: "Sand Scorpion".into(),
            level: 8,
            element: Element::Mars,
            hp: 220,
            pp: 18,
            atk: 26,
            def: 22,
            mag: 12,
            spd: 14,
            abilities: vec![
                ea("strike"),
                ea("venom-bite"),
                ea("burn-touch"),
                ea("armor-pierce"),
            ],
            base_xp: 90,
            base_gold: 70,
            tier: 2,
            drop_table: vec![drop("antidote", 0.20)],
        },
        // ===== NEW MONSTERS - Tier 3 Tough (4) =====
        EnemyDefinition {
            id: "wyvern".into(),
            name: "Wyvern".into(),
            level: 11,
            element: Element::Jupiter,
            hp: 420,
            pp: 35,
            atk: 32,
            def: 24,
            mag: 26,
            spd: 22,
            abilities: vec![
                ea("gust"),
                ea("gale-force-damage"),
                ea("whirlwind-slash"),
                ea("boost-spd"),
            ],
            base_xp: 200,
            base_gold: 110,
            tier: 3,
            drop_table: vec![drop("potion", 0.20)],
        },
        EnemyDefinition {
            id: "frost-giant".into(),
            name: "Frost Giant".into(),
            level: 12,
            element: Element::Mercury,
            hp: 550,
            pp: 40,
            atk: 34,
            def: 30,
            mag: 28,
            spd: 9,
            abilities: vec![
                ea("ice-shard"),
                ea("freeze-blast"),
                ea("tundra"),
                ea("heavy-strike"),
                ea("boost-def"),
            ],
            base_xp: 220,
            base_gold: 120,
            tier: 3,
            drop_table: vec![drop("elixir", 0.10), drop("mercury-water-of-life", 0.08)],
        },
        EnemyDefinition {
            id: "shadow-knight".into(),
            name: "Shadow Knight".into(),
            level: 13,
            element: Element::Venus,
            hp: 480,
            pp: 30,
            atk: 38,
            def: 32,
            mag: 18,
            spd: 16,
            abilities: vec![
                ea("strike"),
                ea("crushing-blow"),
                ea("guard-break"),
                ea("curse"),
                ea("boost-atk"),
            ],
            base_xp: 240,
            base_gold: 130,
            tier: 3,
            drop_table: vec![drop("potion", 0.15), drop("power_bread", 0.05)],
        },
        EnemyDefinition {
            id: "thunder-drake".into(),
            name: "Thunder Drake".into(),
            level: 14,
            element: Element::Jupiter,
            hp: 500,
            pp: 50,
            atk: 36,
            def: 28,
            mag: 35,
            spd: 20,
            abilities: vec![
                ea("chain-lightning"),
                ea("plasma"),
                ea("paralyze-shock"),
                ea("gust"),
                ea("boost-atk"),
            ],
            base_xp: 260,
            base_gold: 140,
            tier: 3,
            drop_table: vec![drop("jupiter-lightning-flask", 0.12)],
        },
        // ===== NEW ELITES - Tier 4 Elite Monsters (4) =====
        EnemyDefinition {
            id: "ancient-dragon".into(),
            name: "Ancient Dragon".into(),
            level: 16,
            element: Element::Mars,
            hp: 850,
            pp: 70,
            atk: 44,
            def: 36,
            mag: 42,
            spd: 18,
            abilities: vec![
                ea("fireball"),
                ea("inferno"),
                ea("flare"),
                ea("burn-touch"),
                ea("boost-atk"),
                ea("weaken-def"),
            ],
            base_xp: 380,
            base_gold: 180,
            tier: 4,
            drop_table: vec![drop("elixir", 0.25), drop("revive-stone", 0.15)],
        },
        EnemyDefinition {
            id: "lich-lord".into(),
            name: "Lich Lord".into(),
            level: 17,
            element: Element::Mercury,
            hp: 700,
            pp: 90,
            atk: 30,
            def: 28,
            mag: 52,
            spd: 16,
            abilities: vec![
                ea("freeze-blast"),
                ea("tundra"),
                ea("curse"),
                ea("haunt"),
                ea("heal"),
                ea("enfeeble"),
            ],
            base_xp: 400,
            base_gold: 190,
            tier: 4,
            drop_table: vec![drop("elixir", 0.30), drop("revive-stone", 0.20)],
        },
        EnemyDefinition {
            id: "storm-titan-elite".into(),
            name: "Storm Titan".into(),
            level: 18,
            element: Element::Jupiter,
            hp: 950,
            pp: 80,
            atk: 46,
            def: 38,
            mag: 48,
            spd: 20,
            abilities: vec![
                ea("chain-lightning"),
                ea("tornado"),
                ea("plasma"),
                ea("paralyze-shock"),
                ea("boost-atk"),
                ea("boost-spd"),
            ],
            base_xp: 420,
            base_gold: 200,
            tier: 4,
            drop_table: vec![drop("elixir", 0.35), drop("jupiter-hermes-water", 0.20)],
        },
        EnemyDefinition {
            id: "abyssal-horror".into(),
            name: "Abyssal Horror".into(),
            level: 19,
            element: Element::Venus,
            hp: 1000,
            pp: 75,
            atk: 48,
            def: 40,
            mag: 44,
            spd: 14,
            abilities: vec![
                ea("quake"),
                ea("gaia"),
                ea("curse"),
                ea("haunt"),
                ea("guard-break"),
                ea("boost-def"),
            ],
            base_xp: 450,
            base_gold: 210,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.35),
                drop("revive-stone", 0.25),
                drop("lucky_medal", 0.15),
            ],
        },
        // ===== BOSSES (12) =====
        EnemyDefinition {
            id: "slaver-chief".into(),
            name: "Slaver Chief".into(),
            level: 3,
            element: Element::Mars,
            hp: 200,
            pp: 20,
            atk: 16,
            def: 12,
            mag: 10,
            spd: 11,
            abilities: vec![
                ea("strike"),
                ea("heavy-strike"),
                ea("fireball"),
                ea("burn-touch"),
                ea("boost-atk"),
            ],
            base_xp: 60,
            base_gold: 40,
            tier: 4,
            drop_table: vec![drop("potion", 0.50), drop("power_bread", 0.15)],
        },
        EnemyDefinition {
            id: "iron-warden".into(),
            name: "Iron Warden".into(),
            level: 5,
            element: Element::Venus,
            hp: 350,
            pp: 15,
            atk: 18,
            def: 28,
            mag: 8,
            spd: 7,
            abilities: vec![
                ea("strike"),
                ea("guard-break"),
                ea("quake"),
                ea("boost-def"),
                ea("heavy-strike"),
            ],
            base_xp: 80,
            base_gold: 50,
            tier: 4,
            drop_table: vec![drop("potion", 0.40), drop("lucky_medal", 0.08)],
        },
        EnemyDefinition {
            id: "phoenix-lord".into(),
            name: "Phoenix Lord".into(),
            level: 7,
            element: Element::Mars,
            hp: 300,
            pp: 40,
            atk: 22,
            def: 16,
            mag: 28,
            spd: 14,
            abilities: vec![
                ea("fireball"),
                ea("burn-touch"),
                ea("heal"),
                ea("boost-atk"),
                ea("flare"),
            ],
            base_xp: 120,
            base_gold: 80,
            tier: 4,
            drop_table: vec![
                drop("mercury-water-of-life", 0.30),
                drop("power_bread", 0.10),
            ],
        },
        EnemyDefinition {
            id: "glacier-queen".into(),
            name: "Glacier Queen".into(),
            level: 8,
            element: Element::Mercury,
            hp: 320,
            pp: 50,
            atk: 18,
            def: 20,
            mag: 32,
            spd: 13,
            abilities: vec![
                ea("ice-shard"),
                ea("freeze-blast"),
                ea("heal"),
                ea("party-heal"),
                ea("boost-def"),
            ],
            base_xp: 140,
            base_gold: 90,
            tier: 4,
            drop_table: vec![drop("elixir", 0.15), drop("lucky_medal", 0.10)],
        },
        EnemyDefinition {
            id: "storm-tyrant".into(),
            name: "Storm Tyrant".into(),
            level: 9,
            element: Element::Jupiter,
            hp: 280,
            pp: 55,
            atk: 24,
            def: 18,
            mag: 35,
            spd: 20,
            abilities: vec![
                ea("gust"),
                ea("chain-lightning"),
                ea("paralyze-shock"),
                ea("blind"),
                ea("boost-atk"),
            ],
            base_xp: 160,
            base_gold: 100,
            tier: 4,
            drop_table: vec![
                drop("jupiter-hermes-water", 0.25),
                drop("lucky_medal", 0.10),
            ],
        },
        EnemyDefinition {
            id: "earth-titan".into(),
            name: "Earth Titan".into(),
            level: 10,
            element: Element::Venus,
            hp: 500,
            pp: 30,
            atk: 30,
            def: 35,
            mag: 15,
            spd: 8,
            abilities: vec![
                ea("strike"),
                ea("heavy-strike"),
                ea("quake"),
                ea("guard-break"),
                ea("boost-def"),
                ea("gaia"),
            ],
            base_xp: 200,
            base_gold: 120,
            tier: 4,
            drop_table: vec![drop("revive-stone", 0.20), drop("power_bread", 0.15)],
        },
        EnemyDefinition {
            id: "infernal-dragon".into(),
            name: "Infernal Dragon".into(),
            level: 12,
            element: Element::Mars,
            hp: 600,
            pp: 60,
            atk: 35,
            def: 25,
            mag: 40,
            spd: 16,
            abilities: vec![
                ea("fireball"),
                ea("burn-touch"),
                ea("flare"),
                ea("inferno"),
                ea("boost-atk"),
                ea("weaken-def"),
            ],
            base_xp: 250,
            base_gold: 150,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.20),
                drop("revive-stone", 0.15),
                drop("lucky_medal", 0.10),
            ],
        },
        EnemyDefinition {
            id: "leviathan".into(),
            name: "Leviathan".into(),
            level: 14,
            element: Element::Mercury,
            hp: 700,
            pp: 70,
            atk: 28,
            def: 30,
            mag: 45,
            spd: 12,
            abilities: vec![
                ea("ice-shard"),
                ea("freeze-blast"),
                ea("tundra"),
                ea("heal"),
                ea("party-heal"),
                ea("boost-def"),
            ],
            base_xp: 300,
            base_gold: 180,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.25),
                drop("revive-stone", 0.20),
                drop("lucky_medal", 0.12),
            ],
        },
        EnemyDefinition {
            id: "vale-overlord".into(),
            name: "Vale Overlord".into(),
            level: 16,
            element: Element::Jupiter,
            hp: 900,
            pp: 80,
            atk: 40,
            def: 32,
            mag: 50,
            spd: 22,
            abilities: vec![
                ea("gust"),
                ea("chain-lightning"),
                ea("tornado"),
                ea("paralyze-shock"),
                ea("boost-atk"),
                ea("heal"),
            ],
            base_xp: 500,
            base_gold: 300,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.40),
                drop("revive-stone", 0.30),
                drop("lucky_medal", 0.20),
            ],
        },
        // ===== NEW BOSSES - Golden Sun Antagonists (3) =====
        EnemyDefinition {
            id: "saturos".into(),
            name: "Saturos".into(),
            level: 10,
            element: Element::Mars,
            hp: 650,
            pp: 55,
            atk: 32,
            def: 26,
            mag: 36,
            spd: 18,
            abilities: vec![
                ea("fireball"),
                ea("flare"),
                ea("heat-wave"),
                ea("burn-touch"),
                ea("boost-atk"),
                ea("weaken-def"),
            ],
            base_xp: 250,
            base_gold: 150,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.40),
                drop("revive-stone", 0.25),
                drop("power_bread", 0.15),
            ],
        },
        EnemyDefinition {
            id: "menardi".into(),
            name: "Menardi".into(),
            level: 10,
            element: Element::Mars,
            hp: 620,
            pp: 60,
            atk: 30,
            def: 24,
            mag: 38,
            spd: 20,
            abilities: vec![
                ea("fireball"),
                ea("inferno"),
                ea("scorch"),
                ea("heal"),
                ea("boost-atk"),
                ea("burn-touch"),
            ],
            base_xp: 250,
            base_gold: 150,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.40),
                drop("revive-stone", 0.25),
                drop("mercury-water-of-life", 0.15),
            ],
        },
        EnemyDefinition {
            id: "agatio".into(),
            name: "Agatio".into(),
            level: 15,
            element: Element::Mars,
            hp: 1100,
            pp: 80,
            atk: 45,
            def: 35,
            mag: 48,
            spd: 19,
            abilities: vec![
                ea("inferno"),
                ea("supernova"),
                ea("flare"),
                ea("burn-touch"),
                ea("boost-atk"),
                ea("weaken-def"),
                ea("heal"),
            ],
            base_xp: 500,
            base_gold: 280,
            tier: 4,
            drop_table: vec![
                drop("elixir", 0.50),
                drop("revive-stone", 0.35),
                drop("lucky_medal", 0.20),
                drop("power_bread", 0.15),
            ],
        },
    ];

    for enemy in enemies {
        m.insert(enemy.id.clone(), enemy);
    }

    m
}

/// Returns all enemy definitions matching the given tier.
///
/// Tier values: 1 = early game, 2 = mid game, 3 = late game, 4 = boss.
#[allow(dead_code)]
pub fn get_enemies_by_tier(tier: u8) -> Vec<EnemyDefinition> {
    let registry = build_enemy_registry();
    registry.into_values().filter(|e| e.tier == tier).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::abilities::build_ability_registry;

    #[test]
    fn test_all_enemies_have_valid_tier() {
        let registry = build_enemy_registry();
        for (id, enemy) in &registry {
            assert!(
                (1..=4).contains(&enemy.tier),
                "Enemy '{}' has invalid tier {}",
                id,
                enemy.tier
            );
        }
    }

    #[test]
    fn test_enemy_count() {
        let registry = build_enemy_registry();
        assert!(
            registry.len() >= 40,
            "Expected at least 40 enemies, got {}",
            registry.len()
        );
    }

    #[test]
    fn test_get_enemies_by_tier_nonempty() {
        for tier in [1, 2, 4] {
            let enemies = get_enemies_by_tier(tier);
            assert!(
                !enemies.is_empty(),
                "Tier {} should have at least one enemy",
                tier
            );
        }
    }

    #[test]
    fn test_boss_enemies_tier_4() {
        let registry = build_enemy_registry();
        let tier_4: Vec<_> = registry.values().filter(|e| e.tier == 4).collect();
        assert!(
            !tier_4.is_empty(),
            "There should be at least one tier 4 (boss) enemy"
        );
        for enemy in &tier_4 {
            assert!(
                enemy.level >= 3,
                "Boss enemy '{}' should be at least level 3, got {}",
                enemy.id,
                enemy.level
            );
        }
    }

    #[test]
    fn test_drop_table_chances_valid() {
        let registry = build_enemy_registry();
        for (id, enemy) in &registry {
            for (item_id, chance) in &enemy.drop_table {
                assert!(
                    (0.0..=1.0).contains(chance),
                    "Enemy '{}' has invalid drop chance {} for item '{}'",
                    id,
                    chance,
                    item_id
                );
            }
        }
    }

    #[test]
    fn test_all_enemies_have_abilities() {
        let registry = build_enemy_registry();
        for (id, enemy) in &registry {
            assert!(
                !enemy.abilities.is_empty(),
                "Enemy '{}' has no abilities assigned",
                id
            );
        }
    }

    #[test]
    fn test_enemy_abilities_exist_in_registry() {
        let enemy_registry = build_enemy_registry();
        let ability_registry = build_ability_registry();
        for (enemy_id, enemy) in &enemy_registry {
            for ea in &enemy.abilities {
                assert!(
                    ability_registry.contains_key(&ea.ability_id),
                    "Enemy '{}' references ability '{}' which does not exist in the ability registry",
                    enemy_id,
                    ea.ability_id
                );
            }
        }
    }

    #[test]
    fn test_no_duplicate_enemy_ids() {
        let registry = build_enemy_registry();
        let mut seen = std::collections::HashSet::new();
        for (id, _) in &registry {
            assert!(
                seen.insert(id.clone()),
                "Duplicate enemy ID found: '{}'",
                id
            );
        }
    }

    #[test]
    fn test_all_enemies_have_positive_hp_atk_def() {
        let registry = build_enemy_registry();
        for (id, enemy) in &registry {
            assert!(
                enemy.hp > 0,
                "Enemy '{}' has non-positive HP: {}",
                id,
                enemy.hp
            );
            assert!(
                enemy.atk > 0,
                "Enemy '{}' has non-positive ATK: {}",
                id,
                enemy.atk
            );
            assert!(
                enemy.def > 0,
                "Enemy '{}' has non-positive DEF: {}",
                id,
                enemy.def
            );
        }
    }

    #[test]
    fn test_xp_and_gold_scale_with_level() {
        let registry = build_enemy_registry();
        let mut enemies: Vec<_> = registry.values().collect();
        enemies.sort_by_key(|e| e.level);

        // Group by level and verify average xp/gold generally increases.
        let mut level_groups: std::collections::BTreeMap<u8, (Vec<u32>, Vec<u32>)> =
            std::collections::BTreeMap::new();
        for e in &enemies {
            let entry = level_groups
                .entry(e.level)
                .or_insert_with(|| (vec![], vec![]));
            entry.0.push(e.base_xp);
            entry.1.push(e.base_gold);
        }

        let mut prev_avg_xp: f64 = 0.0;
        let mut prev_avg_gold: f64 = 0.0;
        let mut prev_level: u8 = 0;
        for (level, (xps, golds)) in &level_groups {
            let avg_xp: f64 = xps.iter().sum::<u32>() as f64 / xps.len() as f64;
            let avg_gold: f64 = golds.iter().sum::<u32>() as f64 / golds.len() as f64;
            if prev_level > 0 {
                assert!(
                    avg_xp >= prev_avg_xp * 0.5,
                    "Level {} avg XP ({:.0}) dropped too much vs level {} ({:.0})",
                    level,
                    avg_xp,
                    prev_level,
                    prev_avg_xp
                );
                assert!(
                    avg_gold >= prev_avg_gold * 0.5,
                    "Level {} avg gold ({:.0}) dropped too much vs level {} ({:.0})",
                    level,
                    avg_gold,
                    prev_level,
                    prev_avg_gold
                );
            }
            prev_avg_xp = avg_xp;
            prev_avg_gold = avg_gold;
            prev_level = *level;
        }
    }

    #[test]
    fn test_boss_enemies_have_higher_stats_than_regular() {
        let registry = build_enemy_registry();

        // Separate bosses (tier 4) from regular enemies (tier 1-3).
        let bosses: Vec<_> = registry.values().filter(|e| e.tier == 4).collect();
        let regulars: Vec<_> = registry.values().filter(|e| e.tier < 4).collect();

        for boss in &bosses {
            // Find regular enemies at the same level or close (within 2 levels).
            let comparable: Vec<_> = regulars
                .iter()
                .filter(|r| r.level >= boss.level.saturating_sub(2) && r.level <= boss.level + 2)
                .collect();

            if comparable.is_empty() {
                continue;
            }

            let avg_hp: f64 =
                comparable.iter().map(|r| r.hp as f64).sum::<f64>() / comparable.len() as f64;

            assert!(
                boss.hp as f64 > avg_hp,
                "Boss '{}' (level {}, HP {}) should have more HP than average regular \
                 enemies at similar levels ({:.0})",
                boss.id,
                boss.level,
                boss.hp,
                avg_hp
            );
        }
    }

    #[test]
    fn test_expanded_enemy_count() {
        let registry = build_enemy_registry();
        assert!(
            registry.len() >= 65,
            "Expected at least 65 enemies after expansion, got {}",
            registry.len()
        );
    }

    #[test]
    fn test_tier_3_enemies_exist() {
        let enemies = get_enemies_by_tier(3);
        assert!(
            !enemies.is_empty(),
            "Tier 3 should have at least one enemy after expansion"
        );
    }

    #[test]
    fn test_new_boss_enemies_present() {
        let registry = build_enemy_registry();
        let expected_bosses = ["saturos", "menardi", "agatio"];
        for boss_id in &expected_bosses {
            assert!(
                registry.contains_key(*boss_id),
                "Expected boss '{}' to be in the registry",
                boss_id
            );
            let boss = &registry[*boss_id];
            assert_eq!(boss.tier, 4, "Boss '{}' should be tier 4", boss_id);
            assert_eq!(
                boss.element,
                Element::Mars,
                "Boss '{}' should be Mars element",
                boss_id
            );
            assert!(
                !boss.drop_table.is_empty(),
                "Boss '{}' should have item drops",
                boss_id
            );
        }
    }
}
