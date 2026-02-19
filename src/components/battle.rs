//! Battle-related ECS marker components.
//!
//! Rich battle types (BattlePhase, BattleAction, BattleUnit, etc.) live in
//! `crate::battle::types`. This module provides lightweight ECS markers and
//! the old `BattleState` resource that was previously used by the components
//! agent.  Where types overlap, prefer `battle::types`.

use bevy::prelude::*;

/// Marker: this entity is participating in battle.
#[derive(Component, Debug, Default, Reflect)]
pub struct InBattle;

/// Marker: this entity is an enemy in the current battle.
#[derive(Component, Debug, Default, Reflect)]
pub struct EnemyCombatant;

/// Marker: this entity is a player party member in the current battle.
#[derive(Component, Debug, Default, Reflect)]
pub struct PartyCombatant;
