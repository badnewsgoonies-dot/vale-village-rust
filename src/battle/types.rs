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
    Poison { duration: i32 },
    Burn { duration: i32 },
    Freeze { duration: i32 },
    Stun { duration: i32 },
    Paralyze { duration: i32 },
    Blind { duration: i32 },
    HealOverTime { heal_per_turn: i32, duration: i32 },
    Buff { stat: StatKind, modifier: i32, duration: i32 },
    Debuff { stat: StatKind, modifier: i32, duration: i32 },
    Shield { remaining_charges: i32, duration: i32 },
    Invulnerable { duration: i32 },
    DamageReduction { percent: f32, duration: i32 },
    AutoRevive { hp_percent: f32, uses_remaining: i32 },
    Immunity { types: Vec<StatusKind>, all_negative: bool, duration: i32 },
}

impl BattleStatusEffect {
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
    pub rewards: Option<BattleRewards>,
    pub level_ups: Vec<LevelUpEvent>,
}

#[derive(Event, Debug, Clone)]
pub struct DamageEvent {
    pub attacker_id: u32,
    pub target_id: u32,
    pub damage: i32,
    pub element: Option<Element>,
    pub was_blocked: bool,
}

#[derive(Event, Debug, Clone)]
pub struct HealEvent {
    pub source_id: u32,
    pub target_id: u32,
    pub amount: i32,
    pub revived: bool,
}

#[derive(Event, Debug, Clone)]
pub struct StatusAppliedEvent {
    pub target_id: u32,
    pub status: BattleStatusEffect,
    pub was_immune: bool,
}

#[derive(Event, Debug, Clone)]
pub struct UnitKoEvent {
    pub unit_id: u32,
    pub unit_name: String,
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
    pub const MAX_ELEMENTAL_RESIST: f32 = 0.75;
    pub const DEFEND_DAMAGE_REDUCTION: f32 = 0.5;

    pub const ELEMENT_ADVANTAGE_MULTIPLIER: f32 = 1.25;
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
        0,     // index 0 (unused)
        0,     // level 1
        100, 350, 850, 1850, 3100, 4700, 6700, 9200,
        12300, 16000, 20400, 25600, 31700, 38800, 47000, 56400,
        67100, 79200, 92800,
    ];
    pub const MAX_LEVEL: u8 = 20;

    pub const BASE_FLEE_CHANCE: f32 = 0.5;
    pub const SPEED_FLEE_BONUS: f32 = 0.02;

    pub const DAMAGE_VARIANCE_MIN: f32 = 0.9;
    pub const DAMAGE_VARIANCE_MAX: f32 = 1.1;
}
