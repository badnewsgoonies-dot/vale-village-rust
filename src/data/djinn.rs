use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::components::stats::Element;

// ---------------------------------------------------------------------------
// Djinn system — elemental creatures that attach to units
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DjinnState {
    /// Passive boost to the host unit.
    Set,
    /// Ready to be used in a summon.
    Standby,
    /// Recovering after a summon (cooldown turns).
    Recovery(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DjinnTier {
    Tier1,
    Tier2,
    Tier3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatModifier {
    #[serde(default)]
    pub atk: i32,
    #[serde(default)]
    pub def: i32,
    #[serde(default)]
    pub mag: i32,
    #[serde(default)]
    pub spd: i32,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub pp: i32,
}

impl Default for StatModifier {
    fn default() -> Self {
        Self { atk: 0, def: 0, mag: 0, spd: 0, hp: 0, pp: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SummonEffectKind {
    Damage { amount: i32 },
    Heal { amount: i32 },
    Buff { stat_bonus: StatModifier },
    StatusInflict { effect_type: String, duration: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummonEffect {
    pub kind: SummonEffectKind,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjinnDefinition {
    pub id: String,
    pub name: String,
    pub element: Element,
    pub tier: DjinnTier,
    /// Stat bonuses granted while this djinn is in Set state.
    pub set_bonus: StatModifier,
    /// The effect when used in a summon.
    pub summon_effect: SummonEffect,
    /// Ability IDs granted to the host unit while set.
    pub granted_ability_ids: Vec<String>,
    /// Number of turns to recover after summoning.
    pub recovery_turns: u8,
    pub description: String,
}

/// Build the djinn registry (12 djinn: 3 per element).
pub fn build_djinn_registry() -> HashMap<String, DjinnDefinition> {
    let mut m = HashMap::new();

    let djinn = vec![
        // ===== VENUS DJINN (3) =====
        DjinnDefinition {
            id: "flint".into(),
            name: "Flint".into(),
            element: Element::Venus,
            tier: DjinnTier::Tier1,
            set_bonus: StatModifier { atk: 3, def: 2, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 80 },
                description: "Stone Barrage scatters earth shards at all foes.".into(),
            },
            granted_ability_ids: vec!["earth-spike-damage".into()],
            recovery_turns: 2,
            description: "A steadfast earth djinn.".into(),
        },
        DjinnDefinition {
            id: "granite".into(),
            name: "Granite".into(),
            element: Element::Venus,
            tier: DjinnTier::Tier2,
            set_bonus: StatModifier { def: 5, hp: 10, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Buff { stat_bonus: StatModifier { def: 10, ..Default::default() } },
                description: "Terra Wall raises nearby allies' defenses.".into(),
            },
            granted_ability_ids: vec!["stone-skin-utility".into()],
            recovery_turns: 3,
            description: "A fortifying earth djinn.".into(),
        },
        DjinnDefinition {
            id: "bane".into(),
            name: "Bane".into(),
            element: Element::Venus,
            tier: DjinnTier::Tier3,
            set_bonus: StatModifier { atk: 5, def: 3, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 300 },
                description: "Earthquake shakes the whole battlefield.".into(),
            },
            granted_ability_ids: vec!["quake".into()],
            recovery_turns: 4,
            description: "A devastating earth djinn.".into(),
        },

        // ===== MARS DJINN (3) =====
        DjinnDefinition {
            id: "forge".into(),
            name: "Forge".into(),
            element: Element::Mars,
            tier: DjinnTier::Tier1,
            set_bonus: StatModifier { atk: 4, mag: 2, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 120 },
                description: "Firebolt barrage burns every foe.".into(),
            },
            granted_ability_ids: vec!["fireball".into()],
            recovery_turns: 2,
            description: "A fiery mars djinn.".into(),
        },
        DjinnDefinition {
            id: "fever".into(),
            name: "Fever".into(),
            element: Element::Mars,
            tier: DjinnTier::Tier2,
            set_bonus: StatModifier { atk: 3, spd: 3, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::StatusInflict { effect_type: "burn".into(), duration: 3 },
                description: "Inflames all enemies with burning fever.".into(),
            },
            granted_ability_ids: vec!["burn-touch".into()],
            recovery_turns: 3,
            description: "A feverish mars djinn.".into(),
        },
        DjinnDefinition {
            id: "corona".into(),
            name: "Corona".into(),
            element: Element::Mars,
            tier: DjinnTier::Tier3,
            set_bonus: StatModifier { atk: 6, mag: 4, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 350 },
                description: "Solar corona erupts on the battlefield.".into(),
            },
            granted_ability_ids: vec!["flare".into()],
            recovery_turns: 4,
            description: "A radiant mars djinn.".into(),
        },

        // ===== MERCURY DJINN (3) =====
        DjinnDefinition {
            id: "fizz".into(),
            name: "Fizz".into(),
            element: Element::Mercury,
            tier: DjinnTier::Tier1,
            set_bonus: StatModifier { def: 2, mag: 3, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Heal { amount: 100 },
                description: "Healing waters wash over all allies.".into(),
            },
            granted_ability_ids: vec!["heal".into()],
            recovery_turns: 2,
            description: "A healing mercury djinn.".into(),
        },
        DjinnDefinition {
            id: "sleet".into(),
            name: "Sleet".into(),
            element: Element::Mercury,
            tier: DjinnTier::Tier2,
            set_bonus: StatModifier { mag: 4, pp: 5, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::StatusInflict { effect_type: "freeze".into(), duration: 2 },
                description: "Freezing sleet immobilizes all enemies.".into(),
            },
            granted_ability_ids: vec!["freeze-blast".into()],
            recovery_turns: 3,
            description: "A chilling mercury djinn.".into(),
        },
        DjinnDefinition {
            id: "serac".into(),
            name: "Serac".into(),
            element: Element::Mercury,
            tier: DjinnTier::Tier3,
            set_bonus: StatModifier { mag: 6, def: 3, hp: 15, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 280 },
                description: "Glacial avalanche buries all foes.".into(),
            },
            granted_ability_ids: vec!["ice-shard".into()],
            recovery_turns: 4,
            description: "A glacial mercury djinn.".into(),
        },

        // ===== JUPITER DJINN (3) =====
        DjinnDefinition {
            id: "gust-djinn".into(),
            name: "Gust".into(),
            element: Element::Jupiter,
            tier: DjinnTier::Tier1,
            set_bonus: StatModifier { spd: 4, atk: 2, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 100 },
                description: "Howling wind tears through enemies.".into(),
            },
            granted_ability_ids: vec!["gust".into()],
            recovery_turns: 2,
            description: "A swift jupiter djinn.".into(),
        },
        DjinnDefinition {
            id: "squall".into(),
            name: "Squall".into(),
            element: Element::Jupiter,
            tier: DjinnTier::Tier2,
            set_bonus: StatModifier { spd: 3, mag: 3, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::StatusInflict { effect_type: "paralyze".into(), duration: 2 },
                description: "Lightning squall paralyzes all enemies.".into(),
            },
            granted_ability_ids: vec!["paralyze-shock".into()],
            recovery_turns: 3,
            description: "A stormy jupiter djinn.".into(),
        },
        DjinnDefinition {
            id: "tempest-djinn".into(),
            name: "Tempest".into(),
            element: Element::Jupiter,
            tier: DjinnTier::Tier3,
            set_bonus: StatModifier { spd: 5, atk: 4, mag: 3, ..Default::default() },
            summon_effect: SummonEffect {
                kind: SummonEffectKind::Damage { amount: 320 },
                description: "Devastating tempest annihilates the battlefield.".into(),
            },
            granted_ability_ids: vec!["chain-lightning".into()],
            recovery_turns: 4,
            description: "A devastating jupiter djinn.".into(),
        },
    ];

    for d in djinn {
        m.insert(d.id.clone(), d);
    }

    m
}
