//! Battle-specific type definitions.
//!
//! Uses shared core types from `components::stats` (Element, UnitStats) and
//! `data::abilities` (Ability, AbilityType, TargetKind). Defines battle-only
//! structures for state tracking, actions, rewards, and events.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Re-export shared types for convenience within the battle module.
pub use crate::components::stats::Element;
pub use crate::data::abilities::{
    Ability as AbilityDef, AbilityType, AiHints, AiTargetPref, TargetKind,
};
// data::djinn types available if needed: DjinnDefinition, DjinnState

// ---------------------------------------------------------------------------
// Status effects (richer than components::stats::StatusEffect for battle use)
// ---------------------------------------------------------------------------

/// Which stat a buff/debuff applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum StatKind {
    Atk,
    Def,
    Mag,
    Spd,
    Luck,
}

/// Tag enum for status-kind matching (used by immunity checks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum StatusKind {
    Poison,
    Burn,
    Freeze,
    Stun,
    Paralyze,
    Blind,
    HealOverTime,
    Buff,
    Debuff,
    Shield,
    Invulnerable,
    DamageReduction,
    AutoRevive,
    Immunity,
}

/// Rich status effects used during battle resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum BattleStatusEffect {
    Poison {
        duration: i32,
    },
    Burn {
        duration: i32,
    },
    Freeze {
        duration: i32,
    },
    Stun {
        duration: i32,
    },
    Paralyze {
        duration: i32,
    },
    Blind {
        duration: i32,
    },
    HealOverTime {
        heal_per_turn: i32,
        duration: i32,
    },
    Buff {
        stat: StatKind,
        modifier: i32,
        duration: i32,
    },
    Debuff {
        stat: StatKind,
        modifier: i32,
        duration: i32,
    },
    Shield {
        remaining_charges: i32,
        duration: i32,
    },
    Invulnerable {
        duration: i32,
    },
    DamageReduction {
        percent: f32,
        duration: i32,
    },
    AutoRevive {
        hp_percent: f32,
        uses_remaining: i32,
    },
    Immunity {
        types: Vec<StatusKind>,
        all_negative: bool,
        duration: i32,
    },
}

impl BattleStatusEffect {
    #[allow(dead_code)]
    pub fn is_negative(&self) -> bool {
        matches!(
            self,
            BattleStatusEffect::Poison { .. }
                | BattleStatusEffect::Burn { .. }
                | BattleStatusEffect::Freeze { .. }
                | BattleStatusEffect::Stun { .. }
                | BattleStatusEffect::Paralyze { .. }
                | BattleStatusEffect::Blind { .. }
                | BattleStatusEffect::Debuff { .. }
        )
    }

    pub fn kind(&self) -> StatusKind {
        match self {
            BattleStatusEffect::Poison { .. } => StatusKind::Poison,
            BattleStatusEffect::Burn { .. } => StatusKind::Burn,
            BattleStatusEffect::Freeze { .. } => StatusKind::Freeze,
            BattleStatusEffect::Stun { .. } => StatusKind::Stun,
            BattleStatusEffect::Paralyze { .. } => StatusKind::Paralyze,
            BattleStatusEffect::Blind { .. } => StatusKind::Blind,
            BattleStatusEffect::HealOverTime { .. } => StatusKind::HealOverTime,
            BattleStatusEffect::Buff { .. } => StatusKind::Buff,
            BattleStatusEffect::Debuff { .. } => StatusKind::Debuff,
            BattleStatusEffect::Shield { .. } => StatusKind::Shield,
            BattleStatusEffect::Invulnerable { .. } => StatusKind::Invulnerable,
            BattleStatusEffect::DamageReduction { .. } => StatusKind::DamageReduction,
            BattleStatusEffect::AutoRevive { .. } => StatusKind::AutoRevive,
            BattleStatusEffect::Immunity { .. } => StatusKind::Immunity,
        }
    }

    /// Returns a human-readable display name for this status effect.
    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            BattleStatusEffect::Poison { .. } => "Poison",
            BattleStatusEffect::Burn { .. } => "Burn",
            BattleStatusEffect::Freeze { .. } => "Freeze",
            BattleStatusEffect::Paralyze { .. } => "Paralyze",
            BattleStatusEffect::Stun { .. } => "Stun",
            BattleStatusEffect::Blind { .. } => "Blind",
            BattleStatusEffect::HealOverTime { .. } => "Regen",
            BattleStatusEffect::Buff { stat, .. } => match stat {
                StatKind::Atk => "ATK Up",
                StatKind::Def => "DEF Up",
                StatKind::Mag => "MAG Up",
                StatKind::Spd => "SPD Up",
                StatKind::Luck => "LCK Up",
            },
            BattleStatusEffect::Debuff { stat, .. } => match stat {
                StatKind::Atk => "ATK Down",
                StatKind::Def => "DEF Down",
                StatKind::Mag => "MAG Down",
                StatKind::Spd => "SPD Down",
                StatKind::Luck => "LCK Down",
            },
            BattleStatusEffect::Shield { .. } => "Shield",
            BattleStatusEffect::Invulnerable { .. } => "Invulnerable",
            BattleStatusEffect::DamageReduction { .. } => "Damage Reduction",
            BattleStatusEffect::AutoRevive { .. } => "Auto-Revive",
            BattleStatusEffect::Immunity { .. } => "Immunity",
        }
    }

    /// Returns the remaining duration in turns for this status effect.
    #[allow(dead_code)]
    pub fn remaining_turns(&self) -> i32 {
        match self {
            BattleStatusEffect::Poison { duration }
            | BattleStatusEffect::Burn { duration }
            | BattleStatusEffect::Freeze { duration }
            | BattleStatusEffect::Stun { duration }
            | BattleStatusEffect::Paralyze { duration }
            | BattleStatusEffect::Blind { duration }
            | BattleStatusEffect::Invulnerable { duration } => *duration,
            BattleStatusEffect::HealOverTime { duration, .. } => *duration,
            BattleStatusEffect::Buff { duration, .. } => *duration,
            BattleStatusEffect::Debuff { duration, .. } => *duration,
            BattleStatusEffect::Shield { duration, .. } => *duration,
            BattleStatusEffect::DamageReduction { duration, .. } => *duration,
            BattleStatusEffect::AutoRevive { uses_remaining, .. } => *uses_remaining,
            BattleStatusEffect::Immunity { duration, .. } => *duration,
        }
    }
}

// ---------------------------------------------------------------------------
// Battle unit (wrapper around stats for in-battle tracking)
// ---------------------------------------------------------------------------

/// Which side this unit fights on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum UnitSide {
    Player,
    Enemy,
}

/// Growth rates per level-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
pub struct GrowthRates {
    pub hp: i32,
    pub pp: i32,
    pub atk: i32,
    pub def: i32,
    pub mag: i32,
    pub spd: i32,
}

/// A unit participating in battle. Contains current HP/PP/status plus metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Component, Reflect)]
pub struct BattleUnit {
    pub id: u32,
    pub name: String,
    pub side: UnitSide,
    pub element: Element,
    pub level: u8,

    // Current stats
    pub hp: i32,
    pub max_hp: i32,
    pub pp: i32,
    pub max_pp: i32,
    pub atk: i32,
    pub def: i32,
    pub mag: i32,
    pub spd: i32,
    pub luck: i32,

    pub status_effects: Vec<BattleStatusEffect>,
    /// Ability IDs this unit knows (looked up from GameData).
    pub ability_ids: Vec<String>,
    /// Djinn IDs attached to this unit.
    pub djinn_ids: Vec<String>,

    // Tracking
    pub damage_taken: i32,
    pub damage_dealt: i32,
    pub xp: u32,
    pub growth_rates: GrowthRates,
}

impl BattleUnit {
    pub fn is_ko(&self) -> bool {
        self.hp <= 0
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Returns a comma-separated summary of active status effects, e.g. "Poison, Burn".
    /// Returns an empty string if no status effects are active.
    #[allow(dead_code)]
    pub fn status_summary(&self) -> String {
        self.status_effects
            .iter()
            .map(|s| s.display_name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Returns current HP as a percentage from 0.0 to 1.0.
    /// Returns 0.0 if max_hp is zero (avoids division by zero).
    #[allow(dead_code)]
    pub fn hp_percent(&self) -> f32 {
        if self.max_hp == 0 {
            return 0.0;
        }
        (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0)
    }

    /// Returns current PP as a percentage from 0.0 to 1.0.
    /// Returns 0.0 if max_pp is zero (avoids division by zero).
    #[allow(dead_code)]
    pub fn pp_percent(&self) -> f32 {
        if self.max_pp == 0 {
            return 0.0;
        }
        (self.pp as f32 / self.max_pp as f32).clamp(0.0, 1.0)
    }

    /// Returns true if any Debuff status effect is active on this unit.
    #[allow(dead_code)]
    pub fn is_debuffed(&self) -> bool {
        self.status_effects
            .iter()
            .any(|s| matches!(s, BattleStatusEffect::Debuff { .. }))
    }

    /// Returns true if any Buff status effect is active on this unit.
    #[allow(dead_code)]
    pub fn is_buffed(&self) -> bool {
        self.status_effects
            .iter()
            .any(|s| matches!(s, BattleStatusEffect::Buff { .. }))
    }
}

// ---------------------------------------------------------------------------
// Battle action
// ---------------------------------------------------------------------------

/// A single action chosen during the command phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum BattleAction {
    Attack { target_id: u32 },
    Ability { ability_id: String, target_id: u32 },
    Item { item_id: String, target_id: u32 },
    Defend,
    Flee,
    DjinnUnleash { djinn_id: String, target_id: u32 },
    Summon { djinn_ids: Vec<String> },
}

// ---------------------------------------------------------------------------
// Battle phase (Bevy state)
// ---------------------------------------------------------------------------

/// Tracks the battle phase as a Bevy State.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States, Reflect)]
pub enum BattlePhase {
    #[default]
    Inactive,
    CommandSelect,
    AiSelect,
    Resolution,
    Victory,
    Defeat,
}

// ---------------------------------------------------------------------------
// Battle state resource
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Resource, Reflect)]
pub struct BattleStateRes {
    pub turn_number: u32,
    pub turn_order: Vec<u32>,
    pub current_actor_index: usize,
    pub actions: Vec<(u32, BattleAction)>,
    pub selecting_unit_index: usize,
    pub encounter_id: String,
    pub fled: bool,
}

impl Default for BattleStateRes {
    fn default() -> Self {
        Self {
            turn_number: 1,
            turn_order: Vec::new(),
            current_actor_index: 0,
            actions: Vec::new(),
            selecting_unit_index: 0,
            encounter_id: String::new(),
            fled: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Command select UI state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum CommandMenu {
    #[default]
    TopLevel,
    AbilitySelect,
    TargetSelect,
    ItemSelect,
    DjinnSelect,
}

#[derive(Debug, Clone, Default, Resource, Reflect)]
pub struct CommandSelectState {
    pub menu: CommandMenu,
    pub cursor_index: usize,
    pub pending_actions: Vec<Option<BattleAction>>,
    pub selected_ability: Option<String>,
    pub selected_djinn: Option<String>,
    pub selecting_unit_index: usize,
}

// ---------------------------------------------------------------------------
// Djinn battle tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum DjinnBattleState {
    Set,
    Standby,
    Recovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DjinnTracker {
    pub djinn_id: String,
    pub state: DjinnBattleState,
    pub owner_unit_id: u32,
    pub last_activated_turn: u32,
    pub recovery_turns_remaining: u32,
}

#[derive(Debug, Clone, Default, Resource, Reflect)]
pub struct DjinnBattleRes {
    pub trackers: Vec<DjinnTracker>,
}

// ---------------------------------------------------------------------------
// Battle rewards
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleRewards {
    pub total_xp: u32,
    pub total_gold: u32,
    pub xp_per_unit: u32,
    pub party_size: u32,
    pub survivor_count: u32,
    pub all_survived: bool,
    pub enemies_defeated: u32,
    pub item_drops: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUpEvent {
    pub unit_id: u32,
    pub unit_name: String,
    pub old_level: u8,
    pub new_level: u8,
    pub stat_gains: StatGains,
    pub new_abilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatGains {
    pub hp: i32,
    pub pp: i32,
    pub atk: i32,
    pub def: i32,
    pub mag: i32,
    pub spd: i32,
}

// ---------------------------------------------------------------------------
// Battle events
// ---------------------------------------------------------------------------

#[derive(Event, Debug, Clone)]
pub struct StartBattleEvent {
    pub encounter_id: String,
    pub enemy_units: Vec<BattleUnit>,
}

#[derive(Event, Debug, Clone)]
pub struct EndBattleEvent {
    pub victory: bool,
    #[allow(dead_code)]
    pub rewards: Option<BattleRewards>,
    #[allow(dead_code)]
    pub level_ups: Vec<LevelUpEvent>,
}

#[derive(Event, Debug, Clone)]
pub struct DamageEvent {
    #[allow(dead_code)]
    pub attacker_id: u32,
    #[allow(dead_code)]
    pub target_id: u32,
    #[allow(dead_code)]
    pub damage: i32,
    #[allow(dead_code)]
    pub element: Option<Element>,
    #[allow(dead_code)]
    pub was_blocked: bool,
}

#[derive(Event, Debug, Clone)]
pub struct HealEvent {
    #[allow(dead_code)]
    pub source_id: u32,
    #[allow(dead_code)]
    pub target_id: u32,
    #[allow(dead_code)]
    pub amount: i32,
    #[allow(dead_code)]
    pub revived: bool,
}

#[derive(Event, Debug, Clone)]
pub struct StatusAppliedEvent {
    #[allow(dead_code)]
    pub target_id: u32,
    #[allow(dead_code)]
    pub status: BattleStatusEffect,
    #[allow(dead_code)]
    pub was_immune: bool,
}

#[derive(Event, Debug, Clone)]
pub struct UnitKoEvent {
    #[allow(dead_code)]
    pub unit_id: u32,
    #[allow(dead_code)]
    pub unit_name: String,
    #[allow(dead_code)]
    pub side: UnitSide,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub mod constants {
    pub const MINIMUM_DAMAGE: i32 = 1;
    pub const MINIMUM_HEALING: i32 = 1;
    pub const DEFENSE_MULTIPLIER: f32 = 0.5;
    pub const PSYNERGY_DEFENSE_MULTIPLIER: f32 = 0.3;
    #[allow(dead_code)]
    pub const MAX_ELEMENTAL_RESIST: f32 = 0.75;
    pub const DEFEND_DAMAGE_REDUCTION: f32 = 0.5;

    #[allow(dead_code)]
    pub const ELEMENT_ADVANTAGE_MULTIPLIER: f32 = 1.25;
    #[allow(dead_code)]
    pub const ELEMENT_DISADVANTAGE_MULTIPLIER: f32 = 0.75;

    pub const POISON_PERCENT: f32 = 0.08;
    pub const BURN_PERCENT: f32 = 0.10;
    pub const FREEZE_BREAK_CHANCE: f32 = 0.30;
    pub const PARALYZE_FAIL_CHANCE: f32 = 0.25;
    pub const BUFF_DEBUFF_STACK_LIMIT: usize = 3;

    pub const DJINN_RECOVERY_TURNS: u32 = 2;
    pub const SUMMON_DAMAGE_1: i32 = 80;
    pub const SUMMON_DAMAGE_2: i32 = 150;
    pub const SUMMON_DAMAGE_3: i32 = 300;

    pub const XP_CURVE: [u32; 21] = [
        0, // index 0 (unused)
        0, // level 1
        100, 350, 850, 1850, 3100, 4700, 6700, 9200, 12300, 16000, 20400, 25600, 31700, 38800,
        47000, 56400, 67100, 79200, 92800,
    ];
    pub const MAX_LEVEL: u8 = 20;

    pub const BASE_FLEE_CHANCE: f32 = 0.5;
    pub const SPEED_FLEE_BONUS: f32 = 0.02;

    pub const DAMAGE_VARIANCE_MIN: f32 = 0.9;
    pub const DAMAGE_VARIANCE_MAX: f32 = 1.1;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a `BattleUnit` with sensible defaults for testing.
    fn make_unit(hp: i32, max_hp: i32, pp: i32, max_pp: i32) -> BattleUnit {
        BattleUnit {
            id: 1,
            name: "TestUnit".to_string(),
            side: UnitSide::Player,
            element: Element::Venus,
            level: 5,
            hp,
            max_hp,
            pp,
            max_pp,
            atk: 20,
            def: 15,
            mag: 10,
            spd: 12,
            luck: 8,
            status_effects: Vec::new(),
            ability_ids: Vec::new(),
            djinn_ids: Vec::new(),
            damage_taken: 0,
            damage_dealt: 0,
            xp: 0,
            growth_rates: GrowthRates::default(),
        }
    }

    // -- BattleStatusEffect::display_name --

    #[test]
    fn display_name_simple_statuses() {
        assert_eq!(
            BattleStatusEffect::Poison { duration: 3 }.display_name(),
            "Poison"
        );
        assert_eq!(
            BattleStatusEffect::Burn { duration: 2 }.display_name(),
            "Burn"
        );
        assert_eq!(
            BattleStatusEffect::Freeze { duration: 1 }.display_name(),
            "Freeze"
        );
        assert_eq!(
            BattleStatusEffect::Paralyze { duration: 2 }.display_name(),
            "Paralyze"
        );
        assert_eq!(
            BattleStatusEffect::Stun { duration: 1 }.display_name(),
            "Stun"
        );
        assert_eq!(
            BattleStatusEffect::Blind { duration: 3 }.display_name(),
            "Blind"
        );
    }

    #[test]
    fn display_name_heal_over_time() {
        let hot = BattleStatusEffect::HealOverTime {
            heal_per_turn: 10,
            duration: 3,
        };
        assert_eq!(hot.display_name(), "Regen");
    }

    #[test]
    fn display_name_buff_variants() {
        let atk_up = BattleStatusEffect::Buff {
            stat: StatKind::Atk,
            modifier: 5,
            duration: 3,
        };
        assert_eq!(atk_up.display_name(), "ATK Up");

        let def_up = BattleStatusEffect::Buff {
            stat: StatKind::Def,
            modifier: 3,
            duration: 2,
        };
        assert_eq!(def_up.display_name(), "DEF Up");

        let mag_up = BattleStatusEffect::Buff {
            stat: StatKind::Mag,
            modifier: 4,
            duration: 3,
        };
        assert_eq!(mag_up.display_name(), "MAG Up");

        let spd_up = BattleStatusEffect::Buff {
            stat: StatKind::Spd,
            modifier: 2,
            duration: 2,
        };
        assert_eq!(spd_up.display_name(), "SPD Up");

        let lck_up = BattleStatusEffect::Buff {
            stat: StatKind::Luck,
            modifier: 1,
            duration: 1,
        };
        assert_eq!(lck_up.display_name(), "LCK Up");
    }

    #[test]
    fn display_name_debuff_variants() {
        let atk_down = BattleStatusEffect::Debuff {
            stat: StatKind::Atk,
            modifier: -5,
            duration: 3,
        };
        assert_eq!(atk_down.display_name(), "ATK Down");

        let def_down = BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -3,
            duration: 2,
        };
        assert_eq!(def_down.display_name(), "DEF Down");

        let mag_down = BattleStatusEffect::Debuff {
            stat: StatKind::Mag,
            modifier: -4,
            duration: 3,
        };
        assert_eq!(mag_down.display_name(), "MAG Down");

        let spd_down = BattleStatusEffect::Debuff {
            stat: StatKind::Spd,
            modifier: -2,
            duration: 2,
        };
        assert_eq!(spd_down.display_name(), "SPD Down");

        let lck_down = BattleStatusEffect::Debuff {
            stat: StatKind::Luck,
            modifier: -1,
            duration: 1,
        };
        assert_eq!(lck_down.display_name(), "LCK Down");
    }

    #[test]
    fn display_name_defensive_statuses() {
        let shield = BattleStatusEffect::Shield {
            remaining_charges: 2,
            duration: 3,
        };
        assert_eq!(shield.display_name(), "Shield");

        let invuln = BattleStatusEffect::Invulnerable { duration: 1 };
        assert_eq!(invuln.display_name(), "Invulnerable");

        let dr = BattleStatusEffect::DamageReduction {
            percent: 0.25,
            duration: 3,
        };
        assert_eq!(dr.display_name(), "Damage Reduction");

        let auto_rev = BattleStatusEffect::AutoRevive {
            hp_percent: 0.5,
            uses_remaining: 1,
        };
        assert_eq!(auto_rev.display_name(), "Auto-Revive");

        let immunity = BattleStatusEffect::Immunity {
            types: vec![StatusKind::Poison],
            all_negative: false,
            duration: 5,
        };
        assert_eq!(immunity.display_name(), "Immunity");
    }

    // -- BattleStatusEffect::remaining_turns --

    #[test]
    fn remaining_turns_simple_statuses() {
        assert_eq!(
            BattleStatusEffect::Poison { duration: 3 }.remaining_turns(),
            3
        );
        assert_eq!(
            BattleStatusEffect::Burn { duration: 2 }.remaining_turns(),
            2
        );
        assert_eq!(
            BattleStatusEffect::Freeze { duration: 1 }.remaining_turns(),
            1
        );
        assert_eq!(
            BattleStatusEffect::Stun { duration: 4 }.remaining_turns(),
            4
        );
        assert_eq!(
            BattleStatusEffect::Paralyze { duration: 2 }.remaining_turns(),
            2
        );
        assert_eq!(
            BattleStatusEffect::Blind { duration: 5 }.remaining_turns(),
            5
        );
        assert_eq!(
            BattleStatusEffect::Invulnerable { duration: 1 }.remaining_turns(),
            1
        );
    }

    #[test]
    fn remaining_turns_compound_statuses() {
        let hot = BattleStatusEffect::HealOverTime {
            heal_per_turn: 10,
            duration: 3,
        };
        assert_eq!(hot.remaining_turns(), 3);

        let buff = BattleStatusEffect::Buff {
            stat: StatKind::Atk,
            modifier: 5,
            duration: 4,
        };
        assert_eq!(buff.remaining_turns(), 4);

        let debuff = BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -3,
            duration: 2,
        };
        assert_eq!(debuff.remaining_turns(), 2);

        let shield = BattleStatusEffect::Shield {
            remaining_charges: 2,
            duration: 5,
        };
        assert_eq!(shield.remaining_turns(), 5);

        let dr = BattleStatusEffect::DamageReduction {
            percent: 0.25,
            duration: 3,
        };
        assert_eq!(dr.remaining_turns(), 3);

        let auto_rev = BattleStatusEffect::AutoRevive {
            hp_percent: 0.5,
            uses_remaining: 1,
        };
        assert_eq!(auto_rev.remaining_turns(), 1);

        let immunity = BattleStatusEffect::Immunity {
            types: vec![],
            all_negative: true,
            duration: 6,
        };
        assert_eq!(immunity.remaining_turns(), 6);
    }

    // -- BattleStatusEffect::is_negative --

    #[test]
    fn is_negative_returns_true_for_harmful_statuses() {
        assert!(BattleStatusEffect::Poison { duration: 3 }.is_negative());
        assert!(BattleStatusEffect::Burn { duration: 2 }.is_negative());
        assert!(BattleStatusEffect::Freeze { duration: 1 }.is_negative());
        assert!(BattleStatusEffect::Stun { duration: 1 }.is_negative());
        assert!(BattleStatusEffect::Paralyze { duration: 2 }.is_negative());
        assert!(BattleStatusEffect::Blind { duration: 3 }.is_negative());
        assert!(
            BattleStatusEffect::Debuff {
                stat: StatKind::Atk,
                modifier: -5,
                duration: 3,
            }
            .is_negative()
        );
    }

    #[test]
    fn is_negative_returns_false_for_beneficial_statuses() {
        assert!(
            !BattleStatusEffect::HealOverTime {
                heal_per_turn: 10,
                duration: 3,
            }
            .is_negative()
        );
        assert!(
            !BattleStatusEffect::Buff {
                stat: StatKind::Atk,
                modifier: 5,
                duration: 3,
            }
            .is_negative()
        );
        assert!(
            !BattleStatusEffect::Shield {
                remaining_charges: 2,
                duration: 3,
            }
            .is_negative()
        );
        assert!(!BattleStatusEffect::Invulnerable { duration: 1 }.is_negative());
        assert!(
            !BattleStatusEffect::AutoRevive {
                hp_percent: 0.5,
                uses_remaining: 1,
            }
            .is_negative()
        );
    }

    // -- BattleUnit::hp_percent --

    #[test]
    fn hp_percent_full() {
        let unit = make_unit(100, 100, 50, 50);
        assert!((unit.hp_percent() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hp_percent_half() {
        let unit = make_unit(50, 100, 50, 50);
        assert!((unit.hp_percent() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn hp_percent_zero() {
        let unit = make_unit(0, 100, 50, 50);
        assert!((unit.hp_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hp_percent_zero_max_hp() {
        let unit = make_unit(0, 0, 50, 50);
        assert!((unit.hp_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hp_percent_clamped_to_one() {
        // If somehow hp > max_hp (overheal edge case), clamp to 1.0
        let unit = make_unit(120, 100, 50, 50);
        assert!((unit.hp_percent() - 1.0).abs() < f32::EPSILON);
    }

    // -- BattleUnit::pp_percent --

    #[test]
    fn pp_percent_full() {
        let unit = make_unit(100, 100, 50, 50);
        assert!((unit.pp_percent() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pp_percent_half() {
        let unit = make_unit(100, 100, 25, 50);
        assert!((unit.pp_percent() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn pp_percent_zero() {
        let unit = make_unit(100, 100, 0, 50);
        assert!((unit.pp_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pp_percent_zero_max_pp() {
        let unit = make_unit(100, 100, 0, 0);
        assert!((unit.pp_percent() - 0.0).abs() < f32::EPSILON);
    }

    // -- BattleUnit::status_summary --

    #[test]
    fn status_summary_empty() {
        let unit = make_unit(100, 100, 50, 50);
        assert_eq!(unit.status_summary(), "");
    }

    #[test]
    fn status_summary_single() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects
            .push(BattleStatusEffect::Poison { duration: 3 });
        assert_eq!(unit.status_summary(), "Poison");
    }

    #[test]
    fn status_summary_multiple() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects
            .push(BattleStatusEffect::Poison { duration: 3 });
        unit.status_effects
            .push(BattleStatusEffect::Burn { duration: 2 });
        assert_eq!(unit.status_summary(), "Poison, Burn");
    }

    #[test]
    fn status_summary_with_buff_and_debuff() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects.push(BattleStatusEffect::Buff {
            stat: StatKind::Atk,
            modifier: 5,
            duration: 3,
        });
        unit.status_effects.push(BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -3,
            duration: 2,
        });
        assert_eq!(unit.status_summary(), "ATK Up, DEF Down");
    }

    // -- BattleUnit::is_debuffed --

    #[test]
    fn is_debuffed_false_when_no_effects() {
        let unit = make_unit(100, 100, 50, 50);
        assert!(!unit.is_debuffed());
    }

    #[test]
    fn is_debuffed_false_when_only_buffs() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects.push(BattleStatusEffect::Buff {
            stat: StatKind::Atk,
            modifier: 5,
            duration: 3,
        });
        assert!(!unit.is_debuffed());
    }

    #[test]
    fn is_debuffed_true_when_debuff_present() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects.push(BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -3,
            duration: 2,
        });
        assert!(unit.is_debuffed());
    }

    #[test]
    fn is_debuffed_false_for_other_negative_statuses() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects
            .push(BattleStatusEffect::Poison { duration: 3 });
        // Poison is negative but not a Debuff variant
        assert!(!unit.is_debuffed());
    }

    // -- BattleUnit::is_buffed --

    #[test]
    fn is_buffed_false_when_no_effects() {
        let unit = make_unit(100, 100, 50, 50);
        assert!(!unit.is_buffed());
    }

    #[test]
    fn is_buffed_false_when_only_debuffs() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects.push(BattleStatusEffect::Debuff {
            stat: StatKind::Def,
            modifier: -3,
            duration: 2,
        });
        assert!(!unit.is_buffed());
    }

    #[test]
    fn is_buffed_true_when_buff_present() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects.push(BattleStatusEffect::Buff {
            stat: StatKind::Atk,
            modifier: 5,
            duration: 3,
        });
        assert!(unit.is_buffed());
    }

    #[test]
    fn is_buffed_false_for_shield() {
        let mut unit = make_unit(100, 100, 50, 50);
        unit.status_effects.push(BattleStatusEffect::Shield {
            remaining_charges: 2,
            duration: 3,
        });
        // Shield is positive but not a Buff variant
        assert!(!unit.is_buffed());
    }
}
