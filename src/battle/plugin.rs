//! Bevy plugin registration for the battle system.

use bevy::prelude::*;

use crate::battle::{
    systems::*,
    types::*,
};

/// Plugin that registers all battle-related resources, events, states, and systems.
pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app
            // State
            .init_state::<BattlePhase>()
            // Resources
            .init_resource::<BattleState>()
            .init_resource::<CommandSelectState>()
            .init_resource::<DjinnBattleState>()
            .init_resource::<BattleRng>()
            // Events
            .add_event::<StartBattleEvent>()
            .add_event::<EndBattleEvent>()
            .add_event::<DamageEvent>()
            .add_event::<HealEvent>()
            .add_event::<StatusAppliedEvent>()
            .add_event::<UnitKoEvent>()
            // Systems: enter/exit
            .add_systems(OnEnter(BattlePhase::CommandSelect), battle_enter_system)
            .add_systems(OnExit(BattlePhase::Victory), battle_exit_system)
            .add_systems(OnExit(BattlePhase::Defeat), battle_exit_system)
            // Systems: per-phase update
            .add_systems(
                Update,
                command_select_system.run_if(in_state(BattlePhase::CommandSelect)),
            )
            .add_systems(
                Update,
                ai_select_system.run_if(in_state(BattlePhase::AiSelect)),
            )
            .add_systems(
                Update,
                resolution_system.run_if(in_state(BattlePhase::Resolution)),
            )
            .add_systems(
                Update,
                victory_system.run_if(in_state(BattlePhase::Victory)),
            )
            .add_systems(
                Update,
                defeat_system.run_if(in_state(BattlePhase::Defeat)),
            );
    }
}
