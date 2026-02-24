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
    #[allow(dead_code)]
    pub items: HashMap<String, ItemDefinition>,
    #[allow(dead_code)]
    pub equipment: HashMap<String, EquipmentDefinition>,
    #[allow(dead_code)]
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
    /// Inventory of owned item/equipment IDs.
    pub inventory: Vec<String>,
    /// Equipped items per unit: unit_id -> (slot_name -> equipment_id).
    pub equipment: HashMap<String, HashMap<String, String>>,
    /// Persisted level/XP per unit: unit_id -> (level, xp)
    pub unit_levels: HashMap<String, (u8, u32)>,
    /// Persisted HP/PP per unit: unit_id -> (current_hp, current_pp)
    pub unit_hp_pp: HashMap<String, (i32, i32)>,
    /// Djinn assignments: djinn_id -> owning unit_id.
    pub djinn_assignments: HashMap<String, String>,
    /// Story progress flags for quest/event tracking.
    pub story_flags: HashMap<String, bool>,
}

impl Default for Party {
    fn default() -> Self {
        Self {
            active: vec!["adept".into()],
            bench: Vec::new(),
            gold: 100,
            inventory: Vec::new(),
            equipment: HashMap::new(),
            unit_levels: HashMap::new(),
            unit_hp_pp: HashMap::new(),
            djinn_assignments: HashMap::new(),
            story_flags: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
impl Party {
    /// Sets a story flag to the given value.
    pub fn set_flag(&mut self, flag: &str, value: bool) {
        self.story_flags.insert(flag.to_string(), value);
    }

    /// Returns true if the given story flag exists and is set to true.
    pub fn has_flag(&self, flag: &str) -> bool {
        self.story_flags.get(flag).copied().unwrap_or(false)
    }

    /// Returns the number of story flags that are set to true (for progress tracking).
    pub fn flag_count(&self) -> usize {
        self.story_flags.values().filter(|&&v| v).count()
    }
}

// ---------------------------------------------------------------------------
// Pre-defined story flag constants
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub mod story {
    pub const RECRUITED_KARIS: &str = "recruited_karis";
    pub const RECRUITED_TYRELL: &str = "recruited_tyrell";
    pub const RECRUITED_AMITI: &str = "recruited_amiti";
    pub const TOWER_ENTERED: &str = "tower_entered";
    pub const TOWER_FLOOR_5: &str = "tower_floor_5";
    pub const TOWER_COMPLETED: &str = "tower_completed";
    pub const TALKED_TO_ELDER: &str = "talked_to_elder";
    pub const FIRST_BATTLE_WON: &str = "first_battle_won";
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_flag_and_has_flag() {
        let mut party = Party::default();
        assert!(!party.has_flag(story::TOWER_ENTERED));

        party.set_flag(story::TOWER_ENTERED, true);
        assert!(party.has_flag(story::TOWER_ENTERED));
    }

    #[test]
    fn test_set_flag_to_false() {
        let mut party = Party::default();
        party.set_flag(story::FIRST_BATTLE_WON, true);
        assert!(party.has_flag(story::FIRST_BATTLE_WON));

        party.set_flag(story::FIRST_BATTLE_WON, false);
        assert!(!party.has_flag(story::FIRST_BATTLE_WON));
    }

    #[test]
    fn test_has_flag_returns_false_for_missing_flag() {
        let party = Party::default();
        assert!(!party.has_flag("nonexistent_flag"));
    }

    #[test]
    fn test_flag_count_empty() {
        let party = Party::default();
        assert_eq!(party.flag_count(), 0);
    }

    #[test]
    fn test_flag_count_with_true_and_false_flags() {
        let mut party = Party::default();
        party.set_flag(story::RECRUITED_KARIS, true);
        party.set_flag(story::RECRUITED_TYRELL, true);
        party.set_flag(story::RECRUITED_AMITI, false);
        party.set_flag(story::TOWER_ENTERED, true);

        // Only 3 flags are true; the false one should not be counted
        assert_eq!(party.flag_count(), 3);
    }

    #[test]
    fn test_flag_count_after_unsetting() {
        let mut party = Party::default();
        party.set_flag(story::TALKED_TO_ELDER, true);
        party.set_flag(story::TOWER_COMPLETED, true);
        assert_eq!(party.flag_count(), 2);

        party.set_flag(story::TALKED_TO_ELDER, false);
        assert_eq!(party.flag_count(), 1);
    }

    #[test]
    fn test_story_flag_constants_are_unique() {
        let flags = [
            story::RECRUITED_KARIS,
            story::RECRUITED_TYRELL,
            story::RECRUITED_AMITI,
            story::TOWER_ENTERED,
            story::TOWER_FLOOR_5,
            story::TOWER_COMPLETED,
            story::TALKED_TO_ELDER,
            story::FIRST_BATTLE_WON,
        ];
        let unique: std::collections::HashSet<&str> = flags.iter().copied().collect();
        assert_eq!(
            flags.len(),
            unique.len(),
            "Story flag constants must be unique"
        );
    }

    #[test]
    fn test_default_party_has_no_story_flags() {
        let party = Party::default();
        assert!(party.story_flags.is_empty());
        assert_eq!(party.flag_count(), 0);
    }
}
