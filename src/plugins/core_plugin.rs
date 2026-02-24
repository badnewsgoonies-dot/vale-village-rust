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
// Difficulty settings -- affects enemy stats, rewards, and flee chances
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

#[derive(Default, Resource, Clone, Debug, Serialize, Deserialize)]
pub struct DifficultySettings {
    pub difficulty: Difficulty,
}

#[allow(dead_code)]
impl DifficultySettings {
    /// Returns a multiplier applied to enemy stats (HP, Attack, etc.).
    /// Easy: 0.8, Normal: 1.0, Hard: 1.3
    pub fn enemy_stat_multiplier(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 0.8,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.3,
        }
    }

    /// Returns a multiplier applied to XP rewards from battles.
    /// Easy: 1.2, Normal: 1.0, Hard: 1.5
    pub fn xp_multiplier(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 1.2,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.5,
        }
    }

    /// Returns a multiplier applied to gold rewards from battles.
    /// Easy: 1.3, Normal: 1.0, Hard: 0.8
    pub fn gold_multiplier(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 1.3,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 0.8,
        }
    }

    /// Returns a bonus (or penalty) applied to the base flee chance.
    /// Easy: +0.15, Normal: 0.0, Hard: -0.10
    pub fn flee_chance_bonus(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 0.15,
            Difficulty::Normal => 0.0,
            Difficulty::Hard => -0.10,
        }
    }
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
    /// Current difficulty setting (persisted with save data).
    pub difficulty: Difficulty,
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
            difficulty: Difficulty::default(),
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
// Bestiary resource -- tracks discovered enemies
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BestiaryEntry {
    pub enemy_id: String,
    pub enemy_name: String,
    pub times_encountered: u32,
    pub times_defeated: u32,
    pub first_encountered: bool,
}

#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct Bestiary {
    /// Enemy ID -> BestiaryEntry
    pub entries: HashMap<String, BestiaryEntry>,
}

#[allow(dead_code)]
impl Bestiary {
    /// Record that the player encountered an enemy. If this is the first time,
    /// a new entry is created with `first_encountered` set to `true`. On
    /// subsequent encounters `times_encountered` is incremented and
    /// `first_encountered` is set to `false`.
    pub fn record_encounter(&mut self, enemy_id: &str, enemy_name: &str) {
        let entry = self
            .entries
            .entry(enemy_id.to_string())
            .or_insert_with(|| BestiaryEntry {
                enemy_id: enemy_id.to_string(),
                enemy_name: enemy_name.to_string(),
                times_encountered: 0,
                times_defeated: 0,
                first_encountered: true,
            });
        entry.times_encountered += 1;
        // After the very first call, subsequent encounters clear the flag.
        if entry.times_encountered > 1 {
            entry.first_encountered = false;
        }
    }

    /// Record that the player defeated an enemy. Only increments the counter;
    /// it does **not** automatically call `record_encounter`.
    pub fn record_defeat(&mut self, enemy_id: &str) {
        if let Some(entry) = self.entries.get_mut(enemy_id) {
            entry.times_defeated += 1;
        }
    }

    /// Returns `true` if the enemy has been encountered at least once.
    pub fn is_discovered(&self, enemy_id: &str) -> bool {
        self.entries.contains_key(enemy_id)
    }

    /// Returns the percentage of enemy types discovered (0.0 – 100.0).
    /// If `total_enemy_types` is zero, returns `0.0` to avoid division by zero.
    pub fn completion_percent(&self, total_enemy_types: usize) -> f32 {
        if total_enemy_types == 0 {
            return 0.0;
        }
        (self.entries.len() as f32 / total_enemy_types as f32) * 100.0
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
        app.insert_resource(DifficultySettings::default());
        app.insert_resource(Bestiary::default());

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

    // -----------------------------------------------------------------------
    // Difficulty enum tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_difficulty_default_is_normal() {
        assert_eq!(Difficulty::default(), Difficulty::Normal);
    }

    #[test]
    fn test_difficulty_settings_default_is_normal() {
        let settings = DifficultySettings::default();
        assert_eq!(settings.difficulty, Difficulty::Normal);
    }

    #[test]
    fn test_party_default_difficulty_is_normal() {
        let party = Party::default();
        assert_eq!(party.difficulty, Difficulty::Normal);
    }

    // -----------------------------------------------------------------------
    // Enemy stat multiplier tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_enemy_stat_multiplier_easy() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        assert!((settings.enemy_stat_multiplier() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_enemy_stat_multiplier_normal() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        assert!((settings.enemy_stat_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_enemy_stat_multiplier_hard() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!((settings.enemy_stat_multiplier() - 1.3).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // XP multiplier tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_xp_multiplier_easy() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        assert!((settings.xp_multiplier() - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_xp_multiplier_normal() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        assert!((settings.xp_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_xp_multiplier_hard() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!((settings.xp_multiplier() - 1.5).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Gold multiplier tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_gold_multiplier_easy() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        assert!((settings.gold_multiplier() - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_gold_multiplier_normal() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        assert!((settings.gold_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_gold_multiplier_hard() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!((settings.gold_multiplier() - 0.8).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Flee chance bonus tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_flee_chance_bonus_easy() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        assert!((settings.flee_chance_bonus() - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn test_flee_chance_bonus_normal() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        assert!((settings.flee_chance_bonus() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_flee_chance_bonus_hard() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!((settings.flee_chance_bonus() - (-0.10)).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Cross-difficulty comparison tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_enemy_stats_increase_with_difficulty() {
        let easy = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        let normal = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        let hard = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!(easy.enemy_stat_multiplier() < normal.enemy_stat_multiplier());
        assert!(normal.enemy_stat_multiplier() < hard.enemy_stat_multiplier());
    }

    #[test]
    fn test_xp_rewards_highest_on_hard() {
        let easy = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        let normal = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        let hard = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!(normal.xp_multiplier() < easy.xp_multiplier());
        assert!(easy.xp_multiplier() < hard.xp_multiplier());
    }

    #[test]
    fn test_gold_rewards_decrease_with_difficulty() {
        let easy = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        let normal = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        let hard = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!(hard.gold_multiplier() < normal.gold_multiplier());
        assert!(normal.gold_multiplier() < easy.gold_multiplier());
    }

    #[test]
    fn test_flee_chance_decreases_with_difficulty() {
        let easy = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        let normal = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        let hard = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        assert!(hard.flee_chance_bonus() < normal.flee_chance_bonus());
        assert!(normal.flee_chance_bonus() < easy.flee_chance_bonus());
    }

    #[test]
    fn test_difficulty_equality() {
        assert_eq!(Difficulty::Easy, Difficulty::Easy);
        assert_eq!(Difficulty::Normal, Difficulty::Normal);
        assert_eq!(Difficulty::Hard, Difficulty::Hard);
        assert_ne!(Difficulty::Easy, Difficulty::Normal);
        assert_ne!(Difficulty::Normal, Difficulty::Hard);
        assert_ne!(Difficulty::Easy, Difficulty::Hard);
    }

    #[test]
    fn test_difficulty_settings_clone() {
        let settings = DifficultySettings {
            difficulty: Difficulty::Hard,
        };
        let cloned = settings.clone();
        assert_eq!(cloned.difficulty, Difficulty::Hard);
    }

    // -----------------------------------------------------------------------
    // Bestiary tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bestiary_default_is_empty() {
        let bestiary = Bestiary::default();
        assert!(bestiary.entries.is_empty());
    }

    #[test]
    fn test_record_encounter_creates_entry() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");

        assert!(bestiary.entries.contains_key("slime_01"));
        let entry = &bestiary.entries["slime_01"];
        assert_eq!(entry.enemy_id, "slime_01");
        assert_eq!(entry.enemy_name, "Green Slime");
        assert_eq!(entry.times_encountered, 1);
        assert_eq!(entry.times_defeated, 0);
        assert!(entry.first_encountered);
    }

    #[test]
    fn test_record_encounter_increments_count() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("slime_01", "Green Slime");

        let entry = &bestiary.entries["slime_01"];
        assert_eq!(entry.times_encountered, 3);
    }

    #[test]
    fn test_record_encounter_clears_first_encountered_after_second() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("bat_01", "Cave Bat");

        assert!(bestiary.entries["bat_01"].first_encountered);

        bestiary.record_encounter("bat_01", "Cave Bat");
        assert!(!bestiary.entries["bat_01"].first_encountered);
    }

    #[test]
    fn test_record_encounter_first_encountered_stays_false_after_many() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("bat_01", "Cave Bat");
        bestiary.record_encounter("bat_01", "Cave Bat");
        bestiary.record_encounter("bat_01", "Cave Bat");
        bestiary.record_encounter("bat_01", "Cave Bat");

        assert!(!bestiary.entries["bat_01"].first_encountered);
        assert_eq!(bestiary.entries["bat_01"].times_encountered, 4);
    }

    #[test]
    fn test_record_encounter_multiple_enemies() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("bat_01", "Cave Bat");
        bestiary.record_encounter("golem_01", "Stone Golem");

        assert_eq!(bestiary.entries.len(), 3);
        assert_eq!(bestiary.entries["slime_01"].times_encountered, 1);
        assert_eq!(bestiary.entries["bat_01"].times_encountered, 1);
        assert_eq!(bestiary.entries["golem_01"].times_encountered, 1);
    }

    #[test]
    fn test_record_defeat_increments_count() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_defeat("slime_01");

        assert_eq!(bestiary.entries["slime_01"].times_defeated, 1);
    }

    #[test]
    fn test_record_defeat_multiple_times() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_defeat("slime_01");
        bestiary.record_defeat("slime_01");
        bestiary.record_defeat("slime_01");

        assert_eq!(bestiary.entries["slime_01"].times_defeated, 3);
    }

    #[test]
    fn test_record_defeat_unknown_enemy_is_noop() {
        let mut bestiary = Bestiary::default();
        // Defeating an enemy that was never encountered should do nothing
        bestiary.record_defeat("unknown_enemy");
        assert!(bestiary.entries.is_empty());
    }

    #[test]
    fn test_record_defeat_does_not_increment_encounters() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_defeat("slime_01");

        assert_eq!(bestiary.entries["slime_01"].times_encountered, 1);
        assert_eq!(bestiary.entries["slime_01"].times_defeated, 1);
    }

    #[test]
    fn test_is_discovered_returns_false_for_unknown() {
        let bestiary = Bestiary::default();
        assert!(!bestiary.is_discovered("slime_01"));
    }

    #[test]
    fn test_is_discovered_returns_true_after_encounter() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        assert!(bestiary.is_discovered("slime_01"));
    }

    #[test]
    fn test_is_discovered_only_for_encountered_enemies() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");

        assert!(bestiary.is_discovered("slime_01"));
        assert!(!bestiary.is_discovered("bat_01"));
        assert!(!bestiary.is_discovered("golem_01"));
    }

    #[test]
    fn test_completion_percent_empty_bestiary() {
        let bestiary = Bestiary::default();
        assert!((bestiary.completion_percent(50) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_percent_zero_total() {
        let bestiary = Bestiary::default();
        assert!((bestiary.completion_percent(0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_percent_zero_total_with_entries() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        // Even with entries, zero total should return 0.0
        assert!((bestiary.completion_percent(0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_percent_partial() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("bat_01", "Cave Bat");

        // 2 out of 10 = 20%
        let percent = bestiary.completion_percent(10);
        assert!((percent - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_percent_full() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("bat_01", "Cave Bat");
        bestiary.record_encounter("golem_01", "Stone Golem");

        // 3 out of 3 = 100%
        let percent = bestiary.completion_percent(3);
        assert!((percent - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_percent_one_of_one() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("boss_01", "Final Boss");

        let percent = bestiary.completion_percent(1);
        assert!((percent - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_percent_half() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("e1", "Enemy 1");
        bestiary.record_encounter("e2", "Enemy 2");
        bestiary.record_encounter("e3", "Enemy 3");

        // 3 out of 6 = 50%
        let percent = bestiary.completion_percent(6);
        assert!((percent - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bestiary_encounter_then_defeat_workflow() {
        let mut bestiary = Bestiary::default();

        // First encounter
        bestiary.record_encounter("dragon_01", "Fire Dragon");
        assert!(bestiary.entries["dragon_01"].first_encountered);
        assert_eq!(bestiary.entries["dragon_01"].times_encountered, 1);
        assert_eq!(bestiary.entries["dragon_01"].times_defeated, 0);

        // Defeat it
        bestiary.record_defeat("dragon_01");
        assert_eq!(bestiary.entries["dragon_01"].times_defeated, 1);

        // Second encounter
        bestiary.record_encounter("dragon_01", "Fire Dragon");
        assert!(!bestiary.entries["dragon_01"].first_encountered);
        assert_eq!(bestiary.entries["dragon_01"].times_encountered, 2);

        // Defeat again
        bestiary.record_defeat("dragon_01");
        assert_eq!(bestiary.entries["dragon_01"].times_defeated, 2);
    }

    #[test]
    fn test_bestiary_clone() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_defeat("slime_01");

        let cloned = bestiary.clone();
        assert_eq!(cloned.entries.len(), 1);
        assert_eq!(cloned.entries["slime_01"].times_encountered, 1);
        assert_eq!(cloned.entries["slime_01"].times_defeated, 1);
    }

    #[test]
    fn test_bestiary_serialize_deserialize() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("bat_01", "Cave Bat");
        bestiary.record_defeat("slime_01");

        let serialized = ron::to_string(&bestiary).expect("serialize failed");
        let deserialized: Bestiary = ron::from_str(&serialized).expect("deserialize failed");

        assert_eq!(deserialized.entries.len(), 2);
        assert_eq!(deserialized.entries["slime_01"].times_encountered, 1);
        assert_eq!(deserialized.entries["slime_01"].times_defeated, 1);
        assert_eq!(deserialized.entries["bat_01"].times_encountered, 1);
        assert_eq!(deserialized.entries["bat_01"].times_defeated, 0);
    }

    #[test]
    fn test_bestiary_multiple_encounters_do_not_duplicate_entries() {
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_encounter("slime_01", "Green Slime");

        // Should still be 1 entry, not 3
        assert_eq!(bestiary.entries.len(), 1);
        assert_eq!(bestiary.entries["slime_01"].times_encountered, 3);
    }

    #[test]
    fn test_bestiary_defeated_more_than_encountered_is_possible() {
        // This could happen if record_defeat is called without record_encounter
        // being called first for every battle. The API allows it once an entry exists.
        let mut bestiary = Bestiary::default();
        bestiary.record_encounter("slime_01", "Green Slime");
        bestiary.record_defeat("slime_01");
        bestiary.record_defeat("slime_01");
        bestiary.record_defeat("slime_01");

        // 1 encounter, 3 defeats -- the API does not enforce the invariant
        assert_eq!(bestiary.entries["slime_01"].times_encountered, 1);
        assert_eq!(bestiary.entries["slime_01"].times_defeated, 3);
    }

    #[test]
    fn test_bestiary_completion_percent_not_affected_by_encounter_count() {
        let mut bestiary = Bestiary::default();
        // Encounter the same enemy 100 times
        for _ in 0..100 {
            bestiary.record_encounter("slime_01", "Green Slime");
        }

        // Still only 1 unique enemy discovered out of 10
        let percent = bestiary.completion_percent(10);
        assert!((percent - 10.0).abs() < f32::EPSILON);
    }
}
