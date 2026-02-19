use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::{
    abilities::Ability,
    djinn::DjinnDefinition,
    enemies::EnemyDefinition,
    items::{EquipmentDefinition, ItemDefinition},
    units::UnitDefinition,
};

// ---------------------------------------------------------------------------
// Game state -- top-level state machine for screen transitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    Overworld,
    Battle,
    Shop,
    Inventory,
    Settings,
    Paused,
}

// ---------------------------------------------------------------------------
// Game data resource -- holds all loaded definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Resource)]
pub struct GameData {
    pub abilities: HashMap<String, Ability>,
    pub units: HashMap<String, UnitDefinition>,
    pub enemies: HashMap<String, EnemyDefinition>,
    pub items: HashMap<String, ItemDefinition>,
    pub equipment: HashMap<String, EquipmentDefinition>,
    pub djinn: HashMap<String, DjinnDefinition>,
}

// ---------------------------------------------------------------------------
// Party resource -- current party state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct Party {
    /// Active party member unit IDs (max 4).
    pub active: Vec<String>,
    /// Bench (reserve) unit IDs.
    pub bench: Vec<String>,
    /// Gold currency.
    pub gold: u32,
    /// Inventory of item_id -> quantity.
    pub inventory: HashMap<String, u32>,
    /// Equipped items per unit: unit_id -> (slot_name -> equipment_id).
    pub equipment: HashMap<String, HashMap<String, String>>,
}

impl Default for Party {
    fn default() -> Self {
        Self {
            active: vec!["adept".into()],
            bench: Vec::new(),
            gold: 100,
            inventory: HashMap::new(),
            equipment: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Core game plugin
// ---------------------------------------------------------------------------

pub struct CoreGamePlugin;

impl Plugin for CoreGamePlugin {
    fn build(&self, app: &mut App) {
        // Initialize game state
        app.init_state::<GameState>();

        // Initialize game data resource (loaded from definitions)
        let game_data = GameData {
            abilities: crate::data::abilities::build_ability_registry(),
            units: crate::data::units::build_unit_registry(),
            enemies: crate::data::enemies::build_enemy_registry(),
            items: crate::data::items::build_item_registry(),
            equipment: crate::data::items::build_equipment_registry(),
            djinn: crate::data::djinn::build_djinn_registry(),
        };

        app.insert_resource(game_data);
        app.insert_resource(Party::default());

        // Register component types for reflection
        app.register_type::<crate::components::stats::UnitStats>();
        app.register_type::<crate::components::stats::ActiveStatusEffect>();
        app.register_type::<crate::components::battle::BattleState>();
        app.register_type::<crate::components::battle::TurnOrder>();
        app.register_type::<crate::components::battle::InBattle>();
        app.register_type::<crate::components::battle::EnemyCombatant>();
        app.register_type::<crate::components::battle::PartyCombatant>();
        app.register_type::<crate::components::world::GridPosition>();
        app.register_type::<crate::components::world::Player>();
        app.register_type::<crate::components::world::Npc>();
        app.register_type::<crate::components::world::Solid>();
        app.register_type::<crate::components::world::EncounterZone>();

        // State transition system: move from Loading to MainMenu once ready
        app.add_systems(OnEnter(GameState::Loading), transition_to_main_menu);
    }
}

fn transition_to_main_menu(mut next_state: ResMut<NextState<GameState>>) {
    info!("Game data loaded. Transitioning to MainMenu.");
    next_state.set(GameState::MainMenu);
}
