use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(unused_imports)]
use crate::components::stats::Element;
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
// Achievement constants
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub mod achievements {
    pub const FIRST_BLOOD: &str = "first_blood";
    pub const FULL_PARTY: &str = "full_party";
    pub const TOWER_ENTERED: &str = "tower_entered";
    pub const TOWER_COMPLETED: &str = "tower_completed";
    pub const BESTIARY_25: &str = "bestiary_25";
    pub const BESTIARY_50: &str = "bestiary_50";
    pub const GOLD_1000: &str = "gold_1000";
    pub const HARD_MODE: &str = "hard_mode";
    pub const NO_KO: &str = "no_ko";
    pub const LEVEL_10: &str = "level_10";
}

// ---------------------------------------------------------------------------
// Achievement entry -- a single achievement definition + unlock state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AchievementEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub unlocked: bool,
}

// ---------------------------------------------------------------------------
// Achievements resource -- tracks all milestones
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct Achievements {
    pub unlocked: HashMap<String, AchievementEntry>,
}

#[allow(dead_code)]
impl Achievements {
    /// Creates an `Achievements` resource with all achievements registered
    /// in their locked (not yet unlocked) state.
    pub fn build_default() -> Self {
        let definitions: Vec<(&str, &str, &str)> = vec![
            (
                achievements::FIRST_BLOOD,
                "First Blood",
                "Win your first battle",
            ),
            (
                achievements::FULL_PARTY,
                "Band of Heroes",
                "Recruit all party members",
            ),
            (
                achievements::TOWER_ENTERED,
                "Into the Depths",
                "Enter the Tower of Trials",
            ),
            (
                achievements::TOWER_COMPLETED,
                "Tower Conqueror",
                "Complete all 10 floors",
            ),
            (
                achievements::BESTIARY_25,
                "Monster Scholar",
                "Discover 25 enemy types",
            ),
            (
                achievements::BESTIARY_50,
                "Monster Master",
                "Discover all 50 enemy types",
            ),
            (
                achievements::GOLD_1000,
                "Wealthy Adept",
                "Accumulate 1000 gold",
            ),
            (
                achievements::HARD_MODE,
                "Hardcore",
                "Win a battle on Hard difficulty",
            ),
            (
                achievements::NO_KO,
                "Flawless Victory",
                "Win a battle with no party members KO'd",
            ),
            (
                achievements::LEVEL_10,
                "Seasoned Warrior",
                "Reach level 10 with any unit",
            ),
        ];

        let mut unlocked = HashMap::new();
        for (id, name, description) in definitions {
            unlocked.insert(
                id.to_string(),
                AchievementEntry {
                    id: id.to_string(),
                    name: name.to_string(),
                    description: description.to_string(),
                    unlocked: false,
                },
            );
        }

        Self { unlocked }
    }

    /// Marks the achievement with the given `id` as unlocked. If the
    /// achievement does not exist in the registry, this is a no-op.
    pub fn unlock(&mut self, id: &str) {
        if let Some(entry) = self.unlocked.get_mut(id) {
            entry.unlocked = true;
        }
    }

    /// Returns `true` if the achievement with the given `id` exists and has
    /// been unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked
            .get(id)
            .map(|entry| entry.unlocked)
            .unwrap_or(false)
    }

    /// Returns `(unlocked_count, total_count)` for progress display.
    pub fn completion_count(&self) -> (usize, usize) {
        let total = self.unlocked.len();
        let done = self.unlocked.values().filter(|e| e.unlocked).count();
        (done, total)
    }
}

// ---------------------------------------------------------------------------
// Weather / atmosphere system — provides visual variety and element bonuses
// ---------------------------------------------------------------------------

/// Weather types that can occur in the overworld and affect battles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    Snow,
    Fog,
    Sandstorm,
}

impl Weather {
    /// Returns all weather variants for iteration.
    #[allow(dead_code)]
    pub fn all() -> &'static [Weather] {
        &[
            Weather::Clear,
            Weather::Rain,
            Weather::Snow,
            Weather::Fog,
            Weather::Sandstorm,
        ]
    }
}

/// Tracks the current weather, its intensity, and a transition timer.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct WeatherState {
    /// The current active weather type.
    pub current: Weather,
    /// Intensity of the weather effect (0.0 = calm, 1.0 = maximum).
    pub intensity: f32,
    /// Seconds remaining until the next weather transition is considered.
    #[allow(dead_code)]
    pub transition_timer: f32,
    /// The zone the player is currently in (used to look up weather weights).
    #[allow(dead_code)]
    pub current_zone: String,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current: Weather::Clear,
            intensity: 0.0,
            transition_timer: 45.0, // start mid-range (30–60s)
            current_zone: "vale_village".to_string(),
        }
    }
}

/// A single zone's weighted weather probabilities.
/// Weights do not need to sum to 1.0 — they are normalised at selection time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoneWeatherWeights {
    pub clear: f32,
    pub rain: f32,
    pub snow: f32,
    pub fog: f32,
    pub sandstorm: f32,
}

impl Default for ZoneWeatherWeights {
    fn default() -> Self {
        Self {
            clear: 0.50,
            rain: 0.20,
            snow: 0.05,
            fog: 0.15,
            sandstorm: 0.10,
        }
    }
}

#[allow(dead_code)]
impl ZoneWeatherWeights {
    /// Returns a list of (Weather, weight) pairs for selection.
    pub fn weighted_list(&self) -> Vec<(Weather, f32)> {
        vec![
            (Weather::Clear, self.clear),
            (Weather::Rain, self.rain),
            (Weather::Snow, self.snow),
            (Weather::Fog, self.fog),
            (Weather::Sandstorm, self.sandstorm),
        ]
    }

    /// Selects a weather type given a random value in [0.0, 1.0).
    /// Weights are normalised internally.  If all weights are zero,
    /// returns `Weather::Clear` as a safe default.
    pub fn select(&self, random_value: f32) -> Weather {
        let pairs = self.weighted_list();
        let total: f32 = pairs.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            return Weather::Clear;
        }
        let mut cumulative = 0.0;
        for (weather, weight) in &pairs {
            cumulative += weight / total;
            if random_value < cumulative {
                return *weather;
            }
        }
        // Floating-point rounding guard — return last variant
        Weather::Sandstorm
    }
}

/// Configuration mapping overworld zone names to their weather probabilities.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct WeatherConfig {
    /// Zone name → weighted weather probabilities.
    pub zones: HashMap<String, ZoneWeatherWeights>,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        let mut zones = HashMap::new();
        // Vale Village — temperate, mostly clear
        zones.insert(
            "vale_village".to_string(),
            ZoneWeatherWeights {
                clear: 0.50,
                rain: 0.25,
                snow: 0.05,
                fog: 0.15,
                sandstorm: 0.05,
            },
        );
        // Mountain area — more snow and fog
        zones.insert(
            "mountain".to_string(),
            ZoneWeatherWeights {
                clear: 0.20,
                rain: 0.10,
                snow: 0.40,
                fog: 0.25,
                sandstorm: 0.05,
            },
        );
        // Desert area — sandstorms dominate
        zones.insert(
            "desert".to_string(),
            ZoneWeatherWeights {
                clear: 0.30,
                rain: 0.02,
                snow: 0.0,
                fog: 0.08,
                sandstorm: 0.60,
            },
        );
        // Tower dungeon — foggy interior
        zones.insert(
            "tower".to_string(),
            ZoneWeatherWeights {
                clear: 0.15,
                rain: 0.0,
                snow: 0.0,
                fog: 0.80,
                sandstorm: 0.05,
            },
        );
        Self { zones }
    }
}

#[allow(dead_code)]
impl WeatherConfig {
    /// Look up the weather weights for a zone.  Falls back to default weights
    /// if the zone is not explicitly configured.
    pub fn weights_for_zone(&self, zone: &str) -> ZoneWeatherWeights {
        self.zones.get(zone).cloned().unwrap_or_default()
    }
}

/// Returns the elemental damage multiplier granted by the current weather.
///
/// * Rain boosts Mercury (Water) by 10 % and penalises Mars (Fire) by 10 %.
/// * Sandstorm boosts Venus (Earth) by 10 % and penalises Jupiter (Wind) by 10 %.
/// * Snow boosts Mercury (Water) by 5 % and Jupiter (Wind) by 5 %.
/// * Fog boosts Jupiter (Wind) by 10 %.
/// * Clear provides no bonus.
///
#[allow(dead_code)]
/// The returned value is a **multiplier** (e.g. `1.1` for a 10 % boost).
pub fn get_weather_element_bonus(weather: &Weather, element: &Element) -> f32 {
    match (weather, element) {
        // Rain: Water +10 %, Fire −10 %
        (Weather::Rain, Element::Mercury) => 1.10,
        (Weather::Rain, Element::Mars) => 0.90,
        // Sandstorm: Earth +10 %, Wind −10 %
        (Weather::Sandstorm, Element::Venus) => 1.10,
        (Weather::Sandstorm, Element::Jupiter) => 0.90,
        // Snow: Water +5 %, Wind +5 %
        (Weather::Snow, Element::Mercury) => 1.05,
        (Weather::Snow, Element::Jupiter) => 1.05,
        // Fog: Wind +10 %
        (Weather::Fog, Element::Jupiter) => 1.10,
        // Clear and all other combinations — no modification
        _ => 1.0,
    }
}

/// Bevy system that ticks the weather transition timer and, when it expires,
/// picks a new weather type based on the current zone's configured weights.
///
/// The timer resets to a random value between 30 and 60 seconds (deterministic
/// via the system's internal counter — a true RNG source can be swapped in
/// later).
#[allow(dead_code)]
pub fn weather_transition_system(
    time: Res<Time>,
    mut weather_state: ResMut<WeatherState>,
    weather_config: Res<WeatherConfig>,
) {
    weather_state.transition_timer -= time.delta_secs();

    if weather_state.transition_timer <= 0.0 {
        let zone = weather_state.current_zone.clone();
        let weights = weather_config.weights_for_zone(&zone);

        // Simple pseudo-random value derived from elapsed wall-clock time.
        // This is *not* cryptographically random, but perfectly fine for
        // weather flavour.  A seeded RNG resource can replace this later.
        let elapsed = time.elapsed_secs_wrapped();
        let pseudo_random = (elapsed * 1_000.0).fract();

        let new_weather = weights.select(pseudo_random);
        weather_state.current = new_weather;

        // Intensity varies with weather type
        weather_state.intensity = match new_weather {
            Weather::Clear => 0.0,
            Weather::Rain => 0.4 + pseudo_random * 0.6, // 0.4–1.0
            Weather::Snow => 0.3 + pseudo_random * 0.5, // 0.3–0.8
            Weather::Fog => 0.5 + pseudo_random * 0.5,  // 0.5–1.0
            Weather::Sandstorm => 0.5 + pseudo_random * 0.5, // 0.5–1.0
        };

        // Reset timer: 30–60 seconds
        weather_state.transition_timer = 30.0 + pseudo_random * 30.0;
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
        app.insert_resource(DifficultySettings::default());
        app.insert_resource(Bestiary::default());
        app.insert_resource(Achievements::build_default());
        app.insert_resource(WeatherState::default());
        app.insert_resource(WeatherConfig::default());

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

    // -----------------------------------------------------------------------
    // Achievement constants tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_achievement_constants_are_unique() {
        let ids = [
            achievements::FIRST_BLOOD,
            achievements::FULL_PARTY,
            achievements::TOWER_ENTERED,
            achievements::TOWER_COMPLETED,
            achievements::BESTIARY_25,
            achievements::BESTIARY_50,
            achievements::GOLD_1000,
            achievements::HARD_MODE,
            achievements::NO_KO,
            achievements::LEVEL_10,
        ];
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "Achievement constants must be unique"
        );
    }

    #[test]
    fn test_achievement_constants_count() {
        // There should be exactly 10 achievement constants
        let ids = [
            achievements::FIRST_BLOOD,
            achievements::FULL_PARTY,
            achievements::TOWER_ENTERED,
            achievements::TOWER_COMPLETED,
            achievements::BESTIARY_25,
            achievements::BESTIARY_50,
            achievements::GOLD_1000,
            achievements::HARD_MODE,
            achievements::NO_KO,
            achievements::LEVEL_10,
        ];
        assert_eq!(ids.len(), 10);
    }

    // -----------------------------------------------------------------------
    // Achievements::build_default tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_default_has_10_achievements() {
        let ach = Achievements::build_default();
        assert_eq!(ach.unlocked.len(), 10);
    }

    #[test]
    fn test_build_default_all_locked() {
        let ach = Achievements::build_default();
        for entry in ach.unlocked.values() {
            assert!(
                !entry.unlocked,
                "Achievement '{}' should start locked",
                entry.id
            );
        }
    }

    #[test]
    fn test_build_default_contains_all_constants() {
        let ach = Achievements::build_default();
        let expected = [
            achievements::FIRST_BLOOD,
            achievements::FULL_PARTY,
            achievements::TOWER_ENTERED,
            achievements::TOWER_COMPLETED,
            achievements::BESTIARY_25,
            achievements::BESTIARY_50,
            achievements::GOLD_1000,
            achievements::HARD_MODE,
            achievements::NO_KO,
            achievements::LEVEL_10,
        ];
        for id in expected {
            assert!(
                ach.unlocked.contains_key(id),
                "build_default() should contain achievement '{}'",
                id
            );
        }
    }

    #[test]
    fn test_build_default_first_blood_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::FIRST_BLOOD];
        assert_eq!(entry.id, "first_blood");
        assert_eq!(entry.name, "First Blood");
        assert_eq!(entry.description, "Win your first battle");
        assert!(!entry.unlocked);
    }

    #[test]
    fn test_build_default_full_party_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::FULL_PARTY];
        assert_eq!(entry.id, "full_party");
        assert_eq!(entry.name, "Band of Heroes");
        assert_eq!(entry.description, "Recruit all party members");
    }

    #[test]
    fn test_build_default_tower_entered_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::TOWER_ENTERED];
        assert_eq!(entry.id, "tower_entered");
        assert_eq!(entry.name, "Into the Depths");
        assert_eq!(entry.description, "Enter the Tower of Trials");
    }

    #[test]
    fn test_build_default_tower_completed_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::TOWER_COMPLETED];
        assert_eq!(entry.id, "tower_completed");
        assert_eq!(entry.name, "Tower Conqueror");
        assert_eq!(entry.description, "Complete all 10 floors");
    }

    #[test]
    fn test_build_default_bestiary_25_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::BESTIARY_25];
        assert_eq!(entry.id, "bestiary_25");
        assert_eq!(entry.name, "Monster Scholar");
        assert_eq!(entry.description, "Discover 25 enemy types");
    }

    #[test]
    fn test_build_default_bestiary_50_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::BESTIARY_50];
        assert_eq!(entry.id, "bestiary_50");
        assert_eq!(entry.name, "Monster Master");
        assert_eq!(entry.description, "Discover all 50 enemy types");
    }

    #[test]
    fn test_build_default_gold_1000_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::GOLD_1000];
        assert_eq!(entry.id, "gold_1000");
        assert_eq!(entry.name, "Wealthy Adept");
        assert_eq!(entry.description, "Accumulate 1000 gold");
    }

    #[test]
    fn test_build_default_hard_mode_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::HARD_MODE];
        assert_eq!(entry.id, "hard_mode");
        assert_eq!(entry.name, "Hardcore");
        assert_eq!(entry.description, "Win a battle on Hard difficulty");
    }

    #[test]
    fn test_build_default_no_ko_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::NO_KO];
        assert_eq!(entry.id, "no_ko");
        assert_eq!(entry.name, "Flawless Victory");
        assert_eq!(entry.description, "Win a battle with no party members KO'd");
    }

    #[test]
    fn test_build_default_level_10_metadata() {
        let ach = Achievements::build_default();
        let entry = &ach.unlocked[achievements::LEVEL_10];
        assert_eq!(entry.id, "level_10");
        assert_eq!(entry.name, "Seasoned Warrior");
        assert_eq!(entry.description, "Reach level 10 with any unit");
    }

    #[test]
    fn test_build_default_entry_ids_match_keys() {
        let ach = Achievements::build_default();
        for (key, entry) in &ach.unlocked {
            assert_eq!(
                key, &entry.id,
                "HashMap key '{}' must match entry id '{}'",
                key, entry.id
            );
        }
    }

    // -----------------------------------------------------------------------
    // Achievements::unlock tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_unlock_sets_achievement_to_unlocked() {
        let mut ach = Achievements::build_default();
        assert!(!ach.unlocked[achievements::FIRST_BLOOD].unlocked);

        ach.unlock(achievements::FIRST_BLOOD);
        assert!(ach.unlocked[achievements::FIRST_BLOOD].unlocked);
    }

    #[test]
    fn test_unlock_unknown_id_is_noop() {
        let mut ach = Achievements::build_default();
        let count_before = ach.unlocked.len();

        ach.unlock("nonexistent_achievement");

        assert_eq!(ach.unlocked.len(), count_before);
    }

    #[test]
    fn test_unlock_idempotent() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::HARD_MODE);
        ach.unlock(achievements::HARD_MODE);
        ach.unlock(achievements::HARD_MODE);

        assert!(ach.unlocked[achievements::HARD_MODE].unlocked);
        // No extra entries should be created
        assert_eq!(ach.unlocked.len(), 10);
    }

    #[test]
    fn test_unlock_multiple_achievements() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::FIRST_BLOOD);
        ach.unlock(achievements::NO_KO);
        ach.unlock(achievements::LEVEL_10);

        assert!(ach.unlocked[achievements::FIRST_BLOOD].unlocked);
        assert!(ach.unlocked[achievements::NO_KO].unlocked);
        assert!(ach.unlocked[achievements::LEVEL_10].unlocked);
        // Others should remain locked
        assert!(!ach.unlocked[achievements::FULL_PARTY].unlocked);
        assert!(!ach.unlocked[achievements::TOWER_ENTERED].unlocked);
    }

    #[test]
    fn test_unlock_does_not_change_metadata() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::GOLD_1000);

        let entry = &ach.unlocked[achievements::GOLD_1000];
        assert_eq!(entry.id, "gold_1000");
        assert_eq!(entry.name, "Wealthy Adept");
        assert_eq!(entry.description, "Accumulate 1000 gold");
        assert!(entry.unlocked);
    }

    // -----------------------------------------------------------------------
    // Achievements::is_unlocked tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_unlocked_returns_false_for_locked() {
        let ach = Achievements::build_default();
        assert!(!ach.is_unlocked(achievements::FIRST_BLOOD));
    }

    #[test]
    fn test_is_unlocked_returns_true_after_unlock() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::TOWER_COMPLETED);
        assert!(ach.is_unlocked(achievements::TOWER_COMPLETED));
    }

    #[test]
    fn test_is_unlocked_returns_false_for_unknown_id() {
        let ach = Achievements::build_default();
        assert!(!ach.is_unlocked("totally_fake_achievement"));
    }

    #[test]
    fn test_is_unlocked_does_not_affect_other_achievements() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::BESTIARY_25);

        assert!(ach.is_unlocked(achievements::BESTIARY_25));
        assert!(!ach.is_unlocked(achievements::BESTIARY_50));
    }

    // -----------------------------------------------------------------------
    // Achievements::completion_count tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_completion_count_all_locked() {
        let ach = Achievements::build_default();
        let (unlocked, total) = ach.completion_count();
        assert_eq!(unlocked, 0);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_completion_count_one_unlocked() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::FIRST_BLOOD);

        let (unlocked, total) = ach.completion_count();
        assert_eq!(unlocked, 1);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_completion_count_several_unlocked() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::FIRST_BLOOD);
        ach.unlock(achievements::FULL_PARTY);
        ach.unlock(achievements::TOWER_ENTERED);
        ach.unlock(achievements::GOLD_1000);

        let (unlocked, total) = ach.completion_count();
        assert_eq!(unlocked, 4);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_completion_count_all_unlocked() {
        let mut ach = Achievements::build_default();
        let all_ids = [
            achievements::FIRST_BLOOD,
            achievements::FULL_PARTY,
            achievements::TOWER_ENTERED,
            achievements::TOWER_COMPLETED,
            achievements::BESTIARY_25,
            achievements::BESTIARY_50,
            achievements::GOLD_1000,
            achievements::HARD_MODE,
            achievements::NO_KO,
            achievements::LEVEL_10,
        ];
        for id in all_ids {
            ach.unlock(id);
        }

        let (unlocked, total) = ach.completion_count();
        assert_eq!(unlocked, 10);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_completion_count_empty_achievements() {
        let ach = Achievements::default();
        let (unlocked, total) = ach.completion_count();
        assert_eq!(unlocked, 0);
        assert_eq!(total, 0);
    }

    // -----------------------------------------------------------------------
    // Achievements Default trait tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_achievements_default_is_empty() {
        let ach = Achievements::default();
        assert!(ach.unlocked.is_empty());
    }

    #[test]
    fn test_achievements_default_differs_from_build_default() {
        let default_ach = Achievements::default();
        let built_ach = Achievements::build_default();
        assert_eq!(default_ach.unlocked.len(), 0);
        assert_eq!(built_ach.unlocked.len(), 10);
    }

    // -----------------------------------------------------------------------
    // Achievements clone tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_achievements_clone() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::FIRST_BLOOD);
        ach.unlock(achievements::NO_KO);

        let cloned = ach.clone();
        assert_eq!(cloned.unlocked.len(), 10);
        assert!(cloned.is_unlocked(achievements::FIRST_BLOOD));
        assert!(cloned.is_unlocked(achievements::NO_KO));
        assert!(!cloned.is_unlocked(achievements::HARD_MODE));
    }

    #[test]
    fn test_achievements_clone_independence() {
        let mut ach = Achievements::build_default();
        let mut cloned = ach.clone();

        ach.unlock(achievements::FIRST_BLOOD);
        cloned.unlock(achievements::LEVEL_10);

        // Mutations should not affect each other
        assert!(ach.is_unlocked(achievements::FIRST_BLOOD));
        assert!(!ach.is_unlocked(achievements::LEVEL_10));
        assert!(!cloned.is_unlocked(achievements::FIRST_BLOOD));
        assert!(cloned.is_unlocked(achievements::LEVEL_10));
    }

    // -----------------------------------------------------------------------
    // Achievements serialize/deserialize tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_achievements_serialize_deserialize() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::FIRST_BLOOD);
        ach.unlock(achievements::TOWER_ENTERED);

        let serialized = ron::to_string(&ach).expect("serialize failed");
        let deserialized: Achievements = ron::from_str(&serialized).expect("deserialize failed");

        assert_eq!(deserialized.unlocked.len(), 10);
        assert!(deserialized.is_unlocked(achievements::FIRST_BLOOD));
        assert!(deserialized.is_unlocked(achievements::TOWER_ENTERED));
        assert!(!deserialized.is_unlocked(achievements::FULL_PARTY));
    }

    #[test]
    fn test_achievements_serialize_preserves_metadata() {
        let mut ach = Achievements::build_default();
        ach.unlock(achievements::GOLD_1000);

        let serialized = ron::to_string(&ach).expect("serialize failed");
        let deserialized: Achievements = ron::from_str(&serialized).expect("deserialize failed");

        let entry = &deserialized.unlocked[achievements::GOLD_1000];
        assert_eq!(entry.id, "gold_1000");
        assert_eq!(entry.name, "Wealthy Adept");
        assert_eq!(entry.description, "Accumulate 1000 gold");
        assert!(entry.unlocked);
    }

    // -----------------------------------------------------------------------
    // Achievements integration-style tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_unlock_all_then_verify_completion() {
        let mut ach = Achievements::build_default();
        assert_eq!(ach.completion_count(), (0, 10));

        ach.unlock(achievements::FIRST_BLOOD);
        assert_eq!(ach.completion_count(), (1, 10));

        ach.unlock(achievements::FULL_PARTY);
        ach.unlock(achievements::TOWER_ENTERED);
        ach.unlock(achievements::TOWER_COMPLETED);
        ach.unlock(achievements::BESTIARY_25);
        assert_eq!(ach.completion_count(), (5, 10));

        ach.unlock(achievements::BESTIARY_50);
        ach.unlock(achievements::GOLD_1000);
        ach.unlock(achievements::HARD_MODE);
        ach.unlock(achievements::NO_KO);
        ach.unlock(achievements::LEVEL_10);
        assert_eq!(ach.completion_count(), (10, 10));
    }

    #[test]
    fn test_unlock_unknown_does_not_change_completion() {
        let mut ach = Achievements::build_default();
        ach.unlock("fake_id_1");
        ach.unlock("fake_id_2");

        assert_eq!(ach.completion_count(), (0, 10));
    }

    #[test]
    fn test_achievement_entry_debug_format() {
        let entry = AchievementEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "A test achievement".to_string(),
            unlocked: false,
        };
        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("Test"));
    }

    #[test]
    fn test_achievements_debug_format() {
        let ach = Achievements::build_default();
        let debug_str = format!("{:?}", ach);
        assert!(debug_str.contains("Achievements"));
    }

    // -----------------------------------------------------------------------
    // Integration tests — data flow across systems
    // -----------------------------------------------------------------------

    #[test]
    fn test_party_gold_accumulation() {
        // Setup: start with default gold (100)
        let mut party = Party::default();
        assert_eq!(party.gold, 100, "Default party should start with 100 gold");

        // Action: simulate gold rewards from multiple battles and shop transactions
        party.gold += 50; // battle reward 1
        assert_eq!(
            party.gold, 150,
            "Gold should be 150 after first battle reward"
        );

        party.gold += 120; // battle reward 2
        assert_eq!(
            party.gold, 270,
            "Gold should be 270 after second battle reward"
        );

        party.gold -= 80; // shop purchase
        assert_eq!(party.gold, 190, "Gold should be 190 after shop purchase");

        party.gold += 300; // large battle reward
        assert_eq!(
            party.gold, 490,
            "Gold should be 490 after large battle reward"
        );

        party.gold += 510; // accumulate to 1000
        assert_eq!(
            party.gold, 1000,
            "Gold should reach 1000 for achievement threshold"
        );

        // Verify: gold accumulates correctly across many operations
        party.gold += 5000;
        party.gold -= 3000;
        assert_eq!(
            party.gold, 3000,
            "Gold should be 3000 after further accumulation and spending"
        );
    }

    #[test]
    fn test_party_full_roster() {
        // Setup: create a party with empty active and bench
        let mut party = Party::default();
        party.active.clear();
        party.bench.clear();

        // Action: add all 10 units — 4 active + 6 bench
        let all_units = [
            "adept",
            "war-mage",
            "mystic",
            "ranger",
            "sentinel",
            "stormcaller",
            "blaze",
            "karis",
            "tyrell",
            "felix",
        ];

        // First 4 go to active party
        for &unit_id in &all_units[..4] {
            party.active.push(unit_id.to_string());
        }
        // Remaining 6 go to bench
        for &unit_id in &all_units[4..] {
            party.bench.push(unit_id.to_string());
        }

        // Verify: correct counts
        assert_eq!(
            party.active.len(),
            4,
            "Active party should have exactly 4 members"
        );
        assert_eq!(party.bench.len(), 6, "Bench should have exactly 6 members");
        assert_eq!(
            party.active.len() + party.bench.len(),
            10,
            "Total roster should be 10 units"
        );

        // Verify: specific members are in the correct slots
        assert_eq!(
            party.active[0], "adept",
            "First active member should be adept"
        );
        assert_eq!(
            party.active[3], "ranger",
            "Fourth active member should be ranger"
        );
        assert_eq!(
            party.bench[0], "sentinel",
            "First bench member should be sentinel"
        );
        assert_eq!(party.bench[5], "felix", "Last bench member should be felix");

        // Verify: no duplicates across active + bench
        let mut all_roster: Vec<&str> = party.active.iter().map(|s| s.as_str()).collect();
        all_roster.extend(party.bench.iter().map(|s| s.as_str()));
        let unique: std::collections::HashSet<&str> = all_roster.iter().copied().collect();
        assert_eq!(
            unique.len(),
            10,
            "All 10 roster members should be unique across active and bench"
        );
    }

    #[test]
    fn test_bestiary_full_completion() {
        // Setup: get all enemy IDs from the actual registry
        let enemy_registry = crate::data::enemies::build_enemy_registry();
        let total_enemy_types = enemy_registry.len();
        let mut bestiary = Bestiary::default();

        // Verify: starts empty
        assert_eq!(
            bestiary.entries.len(),
            0,
            "Bestiary should start with no entries"
        );
        assert!(
            (bestiary.completion_percent(total_enemy_types) - 0.0).abs() < f32::EPSILON,
            "Completion should be 0% initially"
        );

        // Action: record encounters for every enemy type
        let mut count = 0;
        for (enemy_id, enemy_def) in &enemy_registry {
            bestiary.record_encounter(enemy_id, &enemy_def.name);
            count += 1;

            // Also defeat half of them
            if count % 2 == 0 {
                bestiary.record_defeat(enemy_id);
            }
        }

        // Verify: 100% completion
        assert_eq!(
            bestiary.entries.len(),
            total_enemy_types,
            "Bestiary should contain all {} enemy types",
            total_enemy_types
        );
        let completion = bestiary.completion_percent(total_enemy_types);
        assert!(
            (completion - 100.0).abs() < f32::EPSILON,
            "Bestiary completion should be 100.0%, got {}",
            completion
        );

        // Verify: all enemies are discovered
        for enemy_id in enemy_registry.keys() {
            assert!(
                bestiary.is_discovered(enemy_id),
                "Enemy '{}' should be discovered after recording encounter",
                enemy_id
            );
        }

        // Verify: encounter counts are correct
        for entry in bestiary.entries.values() {
            assert_eq!(
                entry.times_encountered, 1,
                "Enemy '{}' should have been encountered exactly once",
                entry.enemy_id
            );
        }

        // Verify: roughly half defeated
        let total_defeated: u32 = bestiary.entries.values().map(|e| e.times_defeated).sum();
        assert_eq!(
            total_defeated as usize,
            total_enemy_types / 2,
            "Half of the enemies should have been defeated"
        );
    }

    #[test]
    fn test_achievements_progression_scenario() {
        // Setup: build a fresh achievement tracker
        let mut ach = Achievements::build_default();

        // Verify: initial state — all locked
        let (unlocked, total) = ach.completion_count();
        assert_eq!(unlocked, 0, "No achievements should be unlocked at start");
        assert_eq!(total, 10, "Total achievements should be 10");

        // Action: simulate a game playthrough, unlocking achievements in order

        // Phase 1: First battle won
        ach.unlock(achievements::FIRST_BLOOD);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(unlocked, 1, "Should have 1 achievement after first battle");
        assert!(
            ach.is_unlocked(achievements::FIRST_BLOOD),
            "FIRST_BLOOD should be unlocked"
        );

        // Phase 2: Won flawlessly
        ach.unlock(achievements::NO_KO);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 2,
            "Should have 2 achievements after flawless victory"
        );

        // Phase 3: Recruited full party
        ach.unlock(achievements::FULL_PARTY);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 3,
            "Should have 3 achievements after full party recruitment"
        );

        // Phase 4: Entered the tower
        ach.unlock(achievements::TOWER_ENTERED);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 4,
            "Should have 4 achievements after entering tower"
        );

        // Phase 5: Hit bestiary milestone
        ach.unlock(achievements::BESTIARY_25);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 5,
            "Should have 5 achievements after 25 bestiary entries"
        );

        // Phase 6: Accumulated gold
        ach.unlock(achievements::GOLD_1000);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 6,
            "Should have 6 achievements after accumulating 1000 gold"
        );

        // Phase 7: Reached level 10
        ach.unlock(achievements::LEVEL_10);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 7,
            "Should have 7 achievements after reaching level 10"
        );

        // Phase 8: Completed the tower
        ach.unlock(achievements::TOWER_COMPLETED);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 8,
            "Should have 8 achievements after completing tower"
        );

        // Phase 9: Full bestiary
        ach.unlock(achievements::BESTIARY_50);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 9,
            "Should have 9 achievements after full bestiary"
        );

        // Phase 10: Hard mode victory
        ach.unlock(achievements::HARD_MODE);
        let (unlocked, total) = ach.completion_count();
        assert_eq!(
            unlocked, 10,
            "Should have all 10 achievements after hard mode victory"
        );
        assert_eq!(
            unlocked, total,
            "Unlocked count should equal total for 100% completion"
        );

        // Verify: re-unlocking does not increase count
        ach.unlock(achievements::FIRST_BLOOD);
        ach.unlock(achievements::HARD_MODE);
        let (unlocked, _) = ach.completion_count();
        assert_eq!(
            unlocked, 10,
            "Re-unlocking achievements should not increase completion count"
        );
    }

    #[test]
    fn test_difficulty_affects_all_multipliers() {
        // Setup: test each difficulty level
        let difficulties = [
            (
                Difficulty::Easy,
                0.8_f32,  // enemy_stat
                1.2_f32,  // xp
                1.3_f32,  // gold
                0.15_f32, // flee bonus
            ),
            (Difficulty::Normal, 1.0_f32, 1.0_f32, 1.0_f32, 0.0_f32),
            (Difficulty::Hard, 1.3_f32, 1.5_f32, 0.8_f32, -0.10_f32),
        ];

        for (difficulty, expected_enemy, expected_xp, expected_gold, expected_flee) in difficulties
        {
            let settings = DifficultySettings { difficulty };

            // Verify: all multipliers are consistent for this difficulty
            assert!(
                (settings.enemy_stat_multiplier() - expected_enemy).abs() < f32::EPSILON,
                "{:?} enemy_stat_multiplier should be {}, got {}",
                difficulty,
                expected_enemy,
                settings.enemy_stat_multiplier()
            );
            assert!(
                (settings.xp_multiplier() - expected_xp).abs() < f32::EPSILON,
                "{:?} xp_multiplier should be {}, got {}",
                difficulty,
                expected_xp,
                settings.xp_multiplier()
            );
            assert!(
                (settings.gold_multiplier() - expected_gold).abs() < f32::EPSILON,
                "{:?} gold_multiplier should be {}, got {}",
                difficulty,
                expected_gold,
                settings.gold_multiplier()
            );
            assert!(
                (settings.flee_chance_bonus() - expected_flee).abs() < f32::EPSILON,
                "{:?} flee_chance_bonus should be {}, got {}",
                difficulty,
                expected_flee,
                settings.flee_chance_bonus()
            );
        }

        // Verify: cross-difficulty consistency — harder = higher enemy stats
        let easy = DifficultySettings {
            difficulty: Difficulty::Easy,
        };
        let normal = DifficultySettings {
            difficulty: Difficulty::Normal,
        };
        let hard = DifficultySettings {
            difficulty: Difficulty::Hard,
        };

        assert!(
            easy.enemy_stat_multiplier() < normal.enemy_stat_multiplier(),
            "Easy enemies should be weaker than Normal"
        );
        assert!(
            normal.enemy_stat_multiplier() < hard.enemy_stat_multiplier(),
            "Normal enemies should be weaker than Hard"
        );
        assert!(
            hard.gold_multiplier() < easy.gold_multiplier(),
            "Hard difficulty should give less gold than Easy"
        );
        assert!(
            hard.flee_chance_bonus() < easy.flee_chance_bonus(),
            "Hard difficulty should have lower flee chance than Easy"
        );
    }

    #[test]
    fn test_story_flags_quest_progression() {
        // Setup: fresh party with no flags
        let mut party = Party::default();
        assert_eq!(
            party.flag_count(),
            0,
            "Party should start with no story flags"
        );

        // Action: set flags in quest progression order

        // Step 1: Talk to the elder
        party.set_flag(story::TALKED_TO_ELDER, true);
        assert!(
            party.has_flag(story::TALKED_TO_ELDER),
            "TALKED_TO_ELDER should be set after talking to elder"
        );
        assert_eq!(party.flag_count(), 1, "Should have 1 flag after step 1");

        // Step 2: Win first battle
        party.set_flag(story::FIRST_BATTLE_WON, true);
        assert!(
            party.has_flag(story::FIRST_BATTLE_WON),
            "FIRST_BATTLE_WON should be set"
        );
        assert_eq!(party.flag_count(), 2, "Should have 2 flags after step 2");

        // Step 3: Recruit party members
        party.set_flag(story::RECRUITED_KARIS, true);
        party.set_flag(story::RECRUITED_TYRELL, true);
        party.set_flag(story::RECRUITED_AMITI, true);
        assert_eq!(
            party.flag_count(),
            5,
            "Should have 5 flags after recruiting all party members"
        );

        // Step 4: Enter the tower
        party.set_flag(story::TOWER_ENTERED, true);
        assert!(
            party.has_flag(story::TOWER_ENTERED),
            "TOWER_ENTERED should be set"
        );
        assert_eq!(party.flag_count(), 6, "Should have 6 flags after step 4");

        // Step 5: Reach floor 5
        party.set_flag(story::TOWER_FLOOR_5, true);
        assert_eq!(party.flag_count(), 7, "Should have 7 flags after step 5");

        // Step 6: Complete the tower
        party.set_flag(story::TOWER_COMPLETED, true);
        assert!(
            party.has_flag(story::TOWER_COMPLETED),
            "TOWER_COMPLETED should be set"
        );
        assert_eq!(
            party.flag_count(),
            8,
            "Should have all 8 story flags set after completing the tower"
        );

        // Verify: all flags are set
        let all_flags = [
            story::TALKED_TO_ELDER,
            story::FIRST_BATTLE_WON,
            story::RECRUITED_KARIS,
            story::RECRUITED_TYRELL,
            story::RECRUITED_AMITI,
            story::TOWER_ENTERED,
            story::TOWER_FLOOR_5,
            story::TOWER_COMPLETED,
        ];
        for flag in all_flags {
            assert!(
                party.has_flag(flag),
                "Flag '{}' should be set after full quest progression",
                flag
            );
        }

        // Verify: has_flag returns false for unset flags
        assert!(
            !party.has_flag("nonexistent_quest"),
            "has_flag should return false for flags that were never set"
        );
    }

    #[test]
    fn test_party_equipment_slots() {
        // Setup: create a party with multiple active units
        let mut party = Party {
            active: vec![
                "adept".to_string(),
                "war-mage".to_string(),
                "mystic".to_string(),
                "ranger".to_string(),
            ],
            ..Default::default()
        };

        // Action: equip items to multiple units in different slots

        // Equip adept: weapon + armor + accessory
        let adept_equip = party.equipment.entry("adept".to_string()).or_default();
        adept_equip.insert("weapon".to_string(), "iron-sword".to_string());
        adept_equip.insert("armor".to_string(), "leather-armor".to_string());
        adept_equip.insert("accessory".to_string(), "lucky-charm".to_string());

        // Equip war-mage: weapon + armor
        let wm_equip = party.equipment.entry("war-mage".to_string()).or_default();
        wm_equip.insert("weapon".to_string(), "battle-axe".to_string());
        wm_equip.insert("armor".to_string(), "iron-armor".to_string());

        // Equip mystic: weapon only
        let mystic_equip = party.equipment.entry("mystic".to_string()).or_default();
        mystic_equip.insert("weapon".to_string(), "arcane-rod".to_string());

        // Equip ranger: weapon + accessory
        let ranger_equip = party.equipment.entry("ranger".to_string()).or_default();
        ranger_equip.insert("weapon".to_string(), "short-bow".to_string());
        ranger_equip.insert("accessory".to_string(), "speed-ring".to_string());

        // Verify: equipment HashMap has correct structure
        assert_eq!(
            party.equipment.len(),
            4,
            "4 units should have equipment entries"
        );

        // Verify: adept has 3 slots
        let adept_slots = &party.equipment["adept"];
        assert_eq!(adept_slots.len(), 3, "Adept should have 3 equipped slots");
        assert_eq!(
            adept_slots["weapon"], "iron-sword",
            "Adept weapon should be iron-sword"
        );
        assert_eq!(
            adept_slots["armor"], "leather-armor",
            "Adept armor should be leather-armor"
        );
        assert_eq!(
            adept_slots["accessory"], "lucky-charm",
            "Adept accessory should be lucky-charm"
        );

        // Verify: war-mage has 2 slots
        let wm_slots = &party.equipment["war-mage"];
        assert_eq!(wm_slots.len(), 2, "War Mage should have 2 equipped slots");
        assert_eq!(
            wm_slots["weapon"], "battle-axe",
            "War Mage weapon should be battle-axe"
        );

        // Verify: mystic has 1 slot
        assert_eq!(
            party.equipment["mystic"].len(),
            1,
            "Mystic should have 1 equipped slot"
        );

        // Verify: ranger has 2 slots
        let ranger_slots = &party.equipment["ranger"];
        assert_eq!(ranger_slots.len(), 2, "Ranger should have 2 equipped slots");
        assert_eq!(
            ranger_slots["accessory"], "speed-ring",
            "Ranger accessory should be speed-ring"
        );

        // Action: swap adept's weapon
        party
            .equipment
            .get_mut("adept")
            .unwrap()
            .insert("weapon".to_string(), "steel-sword".to_string());

        // Verify: weapon was updated, not duplicated
        assert_eq!(
            party.equipment["adept"]["weapon"], "steel-sword",
            "Adept weapon should be updated to steel-sword"
        );
        assert_eq!(
            party.equipment["adept"].len(),
            3,
            "Adept should still have 3 equipped slots after weapon swap"
        );
    }

    #[test]
    fn test_party_serialization_round_trip() {
        // Setup: create a party with non-trivial state
        let mut party = Party {
            active: vec![
                "adept".to_string(),
                "war-mage".to_string(),
                "mystic".to_string(),
                "ranger".to_string(),
            ],
            bench: vec!["sentinel".to_string(), "stormcaller".to_string()],
            gold: 2500,
            inventory: vec![
                "potion".to_string(),
                "potion".to_string(),
                "elixir".to_string(),
            ],
            difficulty: Difficulty::Hard,
            ..Default::default()
        };

        // Set equipment
        let mut adept_equip = HashMap::new();
        adept_equip.insert("weapon".to_string(), "iron-sword".to_string());
        adept_equip.insert("armor".to_string(), "leather-armor".to_string());
        party.equipment.insert("adept".to_string(), adept_equip);

        // Set unit levels and HP/PP
        party.unit_levels.insert("adept".to_string(), (5, 1200));
        party.unit_levels.insert("war-mage".to_string(), (3, 400));
        party.unit_hp_pp.insert("adept".to_string(), (85, 20));
        party.unit_hp_pp.insert("war-mage".to_string(), (60, 15));

        // Set djinn assignments
        party
            .djinn_assignments
            .insert("flint".to_string(), "adept".to_string());
        party
            .djinn_assignments
            .insert("forge".to_string(), "war-mage".to_string());

        // Set story flags
        party.set_flag(story::TALKED_TO_ELDER, true);
        party.set_flag(story::FIRST_BATTLE_WON, true);
        party.set_flag(story::RECRUITED_KARIS, true);
        party.set_flag(story::TOWER_ENTERED, true);

        // Action: serialize to RON and deserialize back
        let serialized =
            ron::to_string(&party).expect("Party should serialize to RON successfully");
        let deserialized: Party =
            ron::from_str(&serialized).expect("Party should deserialize from RON successfully");

        // Verify: all fields match after round trip
        assert_eq!(
            deserialized.active, party.active,
            "Active roster should match after round trip"
        );
        assert_eq!(
            deserialized.bench, party.bench,
            "Bench roster should match after round trip"
        );
        assert_eq!(
            deserialized.gold, party.gold,
            "Gold should match after round trip"
        );
        assert_eq!(
            deserialized.inventory, party.inventory,
            "Inventory should match after round trip"
        );
        assert_eq!(
            deserialized.difficulty, party.difficulty,
            "Difficulty should match after round trip"
        );
        assert_eq!(
            deserialized.equipment, party.equipment,
            "Equipment should match after round trip"
        );
        assert_eq!(
            deserialized.unit_levels, party.unit_levels,
            "Unit levels should match after round trip"
        );
        assert_eq!(
            deserialized.unit_hp_pp, party.unit_hp_pp,
            "Unit HP/PP should match after round trip"
        );
        assert_eq!(
            deserialized.djinn_assignments, party.djinn_assignments,
            "Djinn assignments should match after round trip"
        );
        assert_eq!(
            deserialized.story_flags, party.story_flags,
            "Story flags should match after round trip"
        );

        // Verify: functional methods work on deserialized data
        assert!(
            deserialized.has_flag(story::TALKED_TO_ELDER),
            "has_flag should work on deserialized party"
        );
        assert!(
            deserialized.has_flag(story::TOWER_ENTERED),
            "has_flag should work on deserialized party"
        );
        assert!(
            !deserialized.has_flag(story::TOWER_COMPLETED),
            "Unset flag should remain unset after round trip"
        );
        assert_eq!(
            deserialized.flag_count(),
            4,
            "Flag count should be 4 after round trip"
        );
    }

    // -----------------------------------------------------------------------
    // Weather system tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_weather_default_is_clear() {
        let state = WeatherState::default();
        assert_eq!(state.current, Weather::Clear);
        assert_eq!(state.intensity, 0.0);
        assert_eq!(state.current_zone, "vale_village");
        assert!(
            (30.0..=60.0).contains(&state.transition_timer),
            "Default transition timer should be between 30 and 60 seconds"
        );
    }

    #[test]
    fn test_weather_enum_all_variants() {
        let all = Weather::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&Weather::Clear));
        assert!(all.contains(&Weather::Rain));
        assert!(all.contains(&Weather::Snow));
        assert!(all.contains(&Weather::Fog));
        assert!(all.contains(&Weather::Sandstorm));
    }

    #[test]
    fn test_weather_element_bonus_rain_boosts_mercury() {
        let bonus = get_weather_element_bonus(&Weather::Rain, &Element::Mercury);
        assert!(
            (bonus - 1.10).abs() < f32::EPSILON,
            "Rain should boost Mercury by 10%"
        );
    }

    #[test]
    fn test_weather_element_bonus_rain_penalises_mars() {
        let bonus = get_weather_element_bonus(&Weather::Rain, &Element::Mars);
        assert!(
            (bonus - 0.90).abs() < f32::EPSILON,
            "Rain should penalise Mars by 10%"
        );
    }

    #[test]
    fn test_weather_element_bonus_sandstorm_boosts_venus() {
        let bonus = get_weather_element_bonus(&Weather::Sandstorm, &Element::Venus);
        assert!(
            (bonus - 1.10).abs() < f32::EPSILON,
            "Sandstorm should boost Venus by 10%"
        );
    }

    #[test]
    fn test_weather_element_bonus_sandstorm_penalises_jupiter() {
        let bonus = get_weather_element_bonus(&Weather::Sandstorm, &Element::Jupiter);
        assert!(
            (bonus - 0.90).abs() < f32::EPSILON,
            "Sandstorm should penalise Jupiter by 10%"
        );
    }

    #[test]
    fn test_weather_element_bonus_snow_boosts_mercury_and_jupiter() {
        let mercury_bonus = get_weather_element_bonus(&Weather::Snow, &Element::Mercury);
        let jupiter_bonus = get_weather_element_bonus(&Weather::Snow, &Element::Jupiter);
        assert!(
            (mercury_bonus - 1.05).abs() < f32::EPSILON,
            "Snow should boost Mercury by 5%"
        );
        assert!(
            (jupiter_bonus - 1.05).abs() < f32::EPSILON,
            "Snow should boost Jupiter by 5%"
        );
    }

    #[test]
    fn test_weather_element_bonus_fog_boosts_jupiter() {
        let bonus = get_weather_element_bonus(&Weather::Fog, &Element::Jupiter);
        assert!(
            (bonus - 1.10).abs() < f32::EPSILON,
            "Fog should boost Jupiter by 10%"
        );
    }

    #[test]
    fn test_weather_element_bonus_clear_is_neutral() {
        for element in &[
            Element::Venus,
            Element::Mercury,
            Element::Mars,
            Element::Jupiter,
            Element::Neutral,
        ] {
            let bonus = get_weather_element_bonus(&Weather::Clear, element);
            assert!(
                (bonus - 1.0).abs() < f32::EPSILON,
                "Clear weather should give 1.0 multiplier for {:?}",
                element
            );
        }
    }

    #[test]
    fn test_weather_element_bonus_neutral_element_unaffected() {
        for weather in Weather::all() {
            let bonus = get_weather_element_bonus(weather, &Element::Neutral);
            assert!(
                (bonus - 1.0).abs() < f32::EPSILON,
                "Neutral element should be unaffected by {:?}",
                weather
            );
        }
    }

    #[test]
    fn test_weather_element_bonus_rain_neutral_to_venus() {
        let bonus = get_weather_element_bonus(&Weather::Rain, &Element::Venus);
        assert!(
            (bonus - 1.0).abs() < f32::EPSILON,
            "Rain should not affect Venus"
        );
    }

    #[test]
    fn test_weather_element_bonus_sandstorm_neutral_to_mercury() {
        let bonus = get_weather_element_bonus(&Weather::Sandstorm, &Element::Mercury);
        assert!(
            (bonus - 1.0).abs() < f32::EPSILON,
            "Sandstorm should not affect Mercury"
        );
    }

    #[test]
    fn test_zone_weather_weights_default() {
        let weights = ZoneWeatherWeights::default();
        let total = weights.clear + weights.rain + weights.snow + weights.fog + weights.sandstorm;
        assert!(
            (total - 1.0).abs() < 0.01,
            "Default weights should approximately sum to 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_zone_weather_weights_select_low_returns_clear() {
        // With default weights (clear=0.50), a random value near 0 should pick Clear
        let weights = ZoneWeatherWeights::default();
        let selected = weights.select(0.0);
        assert_eq!(
            selected,
            Weather::Clear,
            "Random value 0.0 with default weights should select Clear"
        );
    }

    #[test]
    fn test_zone_weather_weights_select_high_returns_sandstorm() {
        // With default weights, a random value near 1.0 should pick the last bucket (Sandstorm)
        let weights = ZoneWeatherWeights::default();
        let selected = weights.select(0.999);
        assert_eq!(
            selected,
            Weather::Sandstorm,
            "Random value 0.999 with default weights should select Sandstorm"
        );
    }

    #[test]
    fn test_zone_weather_weights_select_all_zero_returns_clear() {
        let weights = ZoneWeatherWeights {
            clear: 0.0,
            rain: 0.0,
            snow: 0.0,
            fog: 0.0,
            sandstorm: 0.0,
        };
        let selected = weights.select(0.5);
        assert_eq!(
            selected,
            Weather::Clear,
            "All-zero weights should default to Clear"
        );
    }

    #[test]
    fn test_zone_weather_weights_single_option() {
        let weights = ZoneWeatherWeights {
            clear: 0.0,
            rain: 1.0,
            snow: 0.0,
            fog: 0.0,
            sandstorm: 0.0,
        };
        // Any random value should return Rain since it's the only non-zero weight
        for val in [0.0, 0.25, 0.5, 0.75, 0.99] {
            let selected = weights.select(val);
            assert_eq!(
                selected,
                Weather::Rain,
                "Only-rain weights should always select Rain (val={})",
                val
            );
        }
    }

    #[test]
    fn test_weather_config_default_has_expected_zones() {
        let config = WeatherConfig::default();
        assert!(
            config.zones.contains_key("vale_village"),
            "Default config should have vale_village zone"
        );
        assert!(
            config.zones.contains_key("mountain"),
            "Default config should have mountain zone"
        );
        assert!(
            config.zones.contains_key("desert"),
            "Default config should have desert zone"
        );
        assert!(
            config.zones.contains_key("tower"),
            "Default config should have tower zone"
        );
    }

    #[test]
    fn test_weather_config_weights_for_known_zone() {
        let config = WeatherConfig::default();
        let desert_weights = config.weights_for_zone("desert");
        assert!(
            desert_weights.sandstorm > 0.5,
            "Desert zone should heavily favour sandstorms"
        );
    }

    #[test]
    fn test_weather_config_weights_for_unknown_zone_falls_back() {
        let config = WeatherConfig::default();
        let fallback = config.weights_for_zone("nonexistent_zone");
        // Should get ZoneWeatherWeights::default()
        let default = ZoneWeatherWeights::default();
        assert!(
            (fallback.clear - default.clear).abs() < f32::EPSILON,
            "Unknown zone should fall back to default weights"
        );
    }

    #[test]
    fn test_weather_serialization_round_trip() {
        let state = WeatherState {
            current: Weather::Rain,
            intensity: 0.75,
            transition_timer: 42.5,
            current_zone: "mountain".to_string(),
        };

        let serialized = ron::to_string(&state).expect("WeatherState should serialize to RON");
        let deserialized: WeatherState =
            ron::from_str(&serialized).expect("WeatherState should deserialize from RON");

        assert_eq!(deserialized.current, Weather::Rain);
        assert!((deserialized.intensity - 0.75).abs() < f32::EPSILON);
        assert!((deserialized.transition_timer - 42.5).abs() < f32::EPSILON);
        assert_eq!(deserialized.current_zone, "mountain");
    }

    #[test]
    fn test_weather_enum_serialization_round_trip() {
        for weather in Weather::all() {
            let serialized = ron::to_string(weather).expect("Weather variant should serialize");
            let deserialized: Weather =
                ron::from_str(&serialized).expect("Weather variant should deserialize");
            assert_eq!(
                *weather, deserialized,
                "Round trip should preserve {:?}",
                weather
            );
        }
    }

    #[test]
    fn test_weather_config_serialization_round_trip() {
        let config = WeatherConfig::default();
        let serialized = ron::to_string(&config).expect("WeatherConfig should serialize to RON");
        let deserialized: WeatherConfig =
            ron::from_str(&serialized).expect("WeatherConfig should deserialize from RON");

        assert_eq!(
            deserialized.zones.len(),
            config.zones.len(),
            "Zone count should match after round trip"
        );
        for zone_name in config.zones.keys() {
            assert!(
                deserialized.zones.contains_key(zone_name),
                "Zone '{}' should be present after round trip",
                zone_name
            );
        }
    }

    #[test]
    fn test_zone_weather_weighted_list_length() {
        let weights = ZoneWeatherWeights::default();
        let list = weights.weighted_list();
        assert_eq!(
            list.len(),
            5,
            "Weighted list should have exactly 5 entries (one per weather type)"
        );
    }

    #[test]
    fn test_weather_state_manual_transition() {
        // Simulate a manual weather state change as the system would do
        let mut state = WeatherState::default();
        assert_eq!(state.current, Weather::Clear);

        state.current = Weather::Sandstorm;
        state.intensity = 0.8;
        state.transition_timer = 35.0;

        assert_eq!(state.current, Weather::Sandstorm);
        assert!((state.intensity - 0.8).abs() < f32::EPSILON);
        assert!((state.transition_timer - 35.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_weather_element_bonus_comprehensive_matrix() {
        // Test every (Weather, Element) combination to ensure no panics
        // and all return values are in a valid range.
        for weather in Weather::all() {
            for element in &[
                Element::Venus,
                Element::Mercury,
                Element::Mars,
                Element::Jupiter,
                Element::Neutral,
            ] {
                let bonus = get_weather_element_bonus(weather, element);
                assert!(
                    (0.5..=2.0).contains(&bonus),
                    "Bonus for {:?}/{:?} should be in [0.5, 2.0], got {}",
                    weather,
                    element,
                    bonus
                );
            }
        }
    }
}
