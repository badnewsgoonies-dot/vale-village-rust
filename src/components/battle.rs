use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::stats::Element;

/// Ability targeting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TargetMode {
    SingleEnemy,
    AllEnemies,
    SingleAlly,
    AllAllies,
    SelfOnly,
}

/// Ability type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum AbilityType {
    Physical,
    Psynergy,
    Healing,
    Buff,
    Debuff,
}

/// An ability that a unit can use.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Ability {
    pub id: u32,
    pub name: String,
    pub ability_type: AbilityType,
    pub pp_cost: i32,
    pub base_power: i32,
    pub targets: TargetMode,
    pub element: Element,
    pub unlock_level: u8,
}

/// A player's chosen action for one turn.
#[derive(Debug, Clone, Reflect)]
pub enum BattleAction {
    Fight { ability_index: usize, target: Entity },
    Djinn { djinn_index: usize, target: Entity },
    Item { item_index: usize, target: Entity },
    Defend,
    Flee,
}

/// Marker: this entity is participating in battle.
#[derive(Component, Debug, Default, Reflect)]
pub struct InBattle;

/// Marker: this entity is an enemy in the current battle.
#[derive(Component, Debug, Default, Reflect)]
pub struct EnemyCombatant;

/// Marker: this entity is a player party member in the current battle.
#[derive(Component, Debug, Default, Reflect)]
pub struct PartyCombatant;

/// The current turn order for resolution phase.
#[derive(Resource, Debug, Default, Reflect)]
pub struct TurnOrder {
    pub order: Vec<Entity>,
    pub current_index: usize,
}

/// Which phase of battle we are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum BattlePhase {
    #[default]
    CommandSelect,
    TargetSelect,
    Resolution,
    Victory,
    Defeat,
}

/// Resource tracking the current battle state.
#[derive(Resource, Debug, Default, Reflect)]
pub struct BattleState {
    pub phase: BattlePhase,
    pub selected_party_index: usize,
    pub selected_action_index: usize,
    pub selected_target_index: usize,
    pub actions: Vec<(Entity, BattleAction)>,
}
