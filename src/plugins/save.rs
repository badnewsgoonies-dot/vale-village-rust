use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::battle::types::{BattleUnit, UnitSide};
use crate::components::world::{GridPosition, Player};
use crate::plugins::core_plugin::{Bestiary, GameData, GameState, Party};

// ---------------------------------------------------------------------------
// Save file structure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMemberSaveData {
    pub unit_id: String,
    pub hp: i32,
    pub pp: i32,
    pub level: u8,
    pub xp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    /// Full party resource snapshot (roster, inventory, equipment).
    pub party: Party,
    /// Per-party-member runtime stats (HP/PP/level/XP).
    pub party_data: Vec<PartyMemberSaveData>,
    /// Per-unit level/xp state: unit_id -> (level, xp).
    pub unit_levels: HashMap<String, (u8, u32)>,
    /// Djinn ownership: djinn_id -> unit_id.
    pub djinn_assignments: HashMap<String, String>,
    /// Story progress flags.
    pub story_flags: HashMap<String, bool>,
    /// Current map the player is on.
    pub current_map: String,
    /// Player position on the current map.
    pub player_position: GridPosition,
    /// Tower floor progress.
    pub tower_floor: u8,
    /// Gold amount (mirrors Party::gold for quick access).
    pub gold: u32,
    /// Play time in seconds.
    pub play_time_secs: f64,
    /// Monster compendium / bestiary state.
    pub bestiary: Bestiary,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: 1,
            party: Party::default(),
            party_data: Vec::new(),
            unit_levels: HashMap::new(),
            djinn_assignments: HashMap::new(),
            story_flags: HashMap::new(),
            current_map: "village".into(),
            player_position: GridPosition::new(5, 5),
            tower_floor: 0,
            gold: Party::default().gold,
            play_time_secs: 0.0,
            bestiary: Bestiary::default(),
        }
    }
}

impl SaveData {
    /// Build save data from the current Bevy world/resources.
    pub fn from_game_state(world: &mut World) -> Self {
        let party = world.get_resource::<Party>().cloned().unwrap_or_default();
        let bestiary = world
            .get_resource::<Bestiary>()
            .cloned()
            .unwrap_or_default();
        let gold = party.gold;

        let current_map = world
            .get_resource::<State<GameState>>()
            .map(|state| format!("{:?}", state.get()))
            .unwrap_or_else(|| "Overworld".to_string());

        let play_time_secs = world
            .get_resource::<Time>()
            .map(|time| time.elapsed_secs_f64())
            .unwrap_or(0.0);

        let player_position = {
            let mut query = world.query_filtered::<&GridPosition, With<Player>>();
            query
                .iter(world)
                .next()
                .copied()
                .unwrap_or(GridPosition::new(5, 5))
        };

        let (name_to_unit_id, base_stats_by_id) = world
            .get_resource::<GameData>()
            .map(|game_data| {
                let name_to_unit_id = game_data
                    .units
                    .iter()
                    .map(|(id, def)| (def.name.clone(), id.clone()))
                    .collect::<HashMap<_, _>>();
                let base_stats_by_id = game_data
                    .units
                    .iter()
                    .map(|(id, def)| (id.clone(), (def.base_hp, def.base_pp)))
                    .collect::<HashMap<_, _>>();
                (name_to_unit_id, base_stats_by_id)
            })
            .unwrap_or_default();

        // Party currently stores roster/economy. If player battle units are live,
        // prefer their runtime HP/PP/level/XP values for the save snapshot.
        let mut runtime_stats_by_unit_id = HashMap::<String, PartyMemberSaveData>::new();
        {
            let mut battle_query = world.query::<&BattleUnit>();
            for unit in battle_query
                .iter(world)
                .filter(|u| u.side == UnitSide::Player)
            {
                if let Some(unit_id) = name_to_unit_id.get(&unit.name) {
                    runtime_stats_by_unit_id.insert(
                        unit_id.clone(),
                        PartyMemberSaveData {
                            unit_id: unit_id.clone(),
                            hp: unit.hp,
                            pp: unit.pp,
                            level: unit.level,
                            xp: unit.xp,
                        },
                    );
                }
            }
        }

        // Fall back to Party persisted levels/HP if no battle units are live
        if runtime_stats_by_unit_id.is_empty() {
            for unit_id in party.active.iter().chain(party.bench.iter()) {
                if runtime_stats_by_unit_id.contains_key(unit_id) {
                    continue;
                }
                let (level, xp) = party.unit_levels.get(unit_id).copied().unwrap_or((1, 0));
                let (hp, pp) = party.unit_hp_pp.get(unit_id).copied().unwrap_or_else(|| {
                    let (base_hp, base_pp) =
                        base_stats_by_id.get(unit_id).copied().unwrap_or((100, 30));
                    (base_hp, base_pp)
                });
                runtime_stats_by_unit_id.insert(
                    unit_id.clone(),
                    PartyMemberSaveData {
                        unit_id: unit_id.clone(),
                        hp,
                        pp,
                        level,
                        xp,
                    },
                );
            }
        }

        let mut member_ids = Vec::<String>::new();
        for unit_id in party.active.iter().chain(party.bench.iter()) {
            if !member_ids.iter().any(|id| id == unit_id) {
                member_ids.push(unit_id.clone());
            }
        }

        let party_data = member_ids
            .into_iter()
            .map(|unit_id| {
                if let Some(runtime) = runtime_stats_by_unit_id.get(&unit_id) {
                    runtime.clone()
                } else {
                    let (hp, pp) = base_stats_by_id.get(&unit_id).copied().unwrap_or((100, 30));
                    PartyMemberSaveData {
                        unit_id: unit_id.clone(),
                        hp,
                        pp,
                        level: 1,
                        xp: 0,
                    }
                }
            })
            .collect::<Vec<_>>();

        let unit_levels = party_data
            .iter()
            .map(|member| (member.unit_id.clone(), (member.level, member.xp)))
            .collect::<HashMap<_, _>>();

        let story_flags = party.story_flags.clone();

        Self {
            version: 1,
            party,
            party_data,
            unit_levels,
            djinn_assignments: HashMap::new(),
            story_flags,
            current_map,
            player_position,
            tower_floor: 0,
            gold,
            play_time_secs,
            bestiary,
        }
    }

    /// Apply this save data back into the active Bevy world/resources.
    pub fn apply_to_game(&self, world: &mut World) {
        let mut restored_party = self.party.clone();
        restored_party.gold = self.gold;
        restored_party.story_flags = self.story_flags.clone();

        // Restore unit_levels and unit_hp_pp on the party from saved party_data
        for member in &self.party_data {
            restored_party
                .unit_levels
                .insert(member.unit_id.clone(), (member.level, member.xp));
            restored_party
                .unit_hp_pp
                .insert(member.unit_id.clone(), (member.hp, member.pp));
        }

        if let Some(mut party) = world.get_resource_mut::<Party>() {
            *party = restored_party;
        } else {
            world.insert_resource(restored_party);
        }

        // Restore bestiary
        if let Some(mut bestiary) = world.get_resource_mut::<Bestiary>() {
            *bestiary = self.bestiary.clone();
        } else {
            world.insert_resource(self.bestiary.clone());
        }

        let player_entities = {
            let mut query = world.query_filtered::<Entity, With<Player>>();
            query.iter(world).collect::<Vec<_>>()
        };

        for entity in player_entities {
            if let Some(mut grid_pos) = world.get_mut::<GridPosition>(entity) {
                *grid_pos = self.player_position;
            }

            if let Some(mut transform) = world.get_mut::<Transform>(entity) {
                const TILE_SIZE: f32 = 32.0;
                transform.translation.x = self.player_position.x as f32 * TILE_SIZE;
                transform.translation.y = -(self.player_position.y as f32) * TILE_SIZE;
            }
        }

        let name_to_unit_id = world
            .get_resource::<GameData>()
            .map(|game_data| {
                game_data
                    .units
                    .iter()
                    .map(|(id, def)| (def.name.clone(), id.clone()))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        let stats_by_unit_id = self
            .party_data
            .iter()
            .map(|member| (member.unit_id.clone(), member))
            .collect::<HashMap<_, _>>();

        let battle_entities = {
            let mut query = world.query_filtered::<Entity, With<BattleUnit>>();
            query.iter(world).collect::<Vec<_>>()
        };

        for entity in battle_entities {
            let Some(mut unit) = world.get_mut::<BattleUnit>(entity) else {
                continue;
            };
            if unit.side != UnitSide::Player {
                continue;
            }

            let Some(unit_id) = name_to_unit_id.get(&unit.name) else {
                continue;
            };
            let Some(saved_stats) = stats_by_unit_id.get(unit_id) else {
                continue;
            };

            unit.hp = saved_stats.hp.clamp(0, unit.max_hp);
            unit.pp = saved_stats.pp.clamp(0, unit.max_pp);
            unit.level = saved_stats.level;
            unit.xp = saved_stats.xp;
        }
    }
}

// ---------------------------------------------------------------------------
// Save/Load system resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct SaveSystem {
    pub save_dir: PathBuf,
    pub max_slots: usize,
}

impl Default for SaveSystem {
    fn default() -> Self {
        // Use the OS data directory
        let save_dir = if let Some(dirs) = dirs::data_dir() {
            dirs.join("vale_village").join("saves")
        } else {
            PathBuf::from("saves")
        };

        Self {
            save_dir,
            max_slots: 3,
        }
    }
}

impl SaveSystem {
    fn slot_path(&self, slot: usize) -> PathBuf {
        self.save_dir.join(format!("save_{}.ron", slot))
    }

    /// Save game state to the given slot (0-indexed).
    pub fn save(&self, slot: usize, data: &SaveData) -> Result<(), String> {
        if slot >= self.max_slots {
            return Err(format!("Invalid save slot: {}", slot));
        }

        // Ensure save directory exists
        std::fs::create_dir_all(&self.save_dir)
            .map_err(|e| format!("Failed to create save directory: {}", e))?;

        let path = self.slot_path(slot);
        let serialized = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize save data: {}", e))?;

        std::fs::write(&path, serialized)
            .map_err(|e| format!("Failed to write save file: {}", e))?;

        info!("Game saved to slot {} at {:?}", slot, path);
        Ok(())
    }

    /// Load game state from the given slot.
    pub fn load(&self, slot: usize) -> Result<SaveData, String> {
        if slot >= self.max_slots {
            return Err(format!("Invalid save slot: {}", slot));
        }

        let path = self.slot_path(slot);
        if !path.exists() {
            return Err(format!("No save file in slot {}", slot));
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read save file: {}", e))?;

        let data: SaveData = ron::from_str(&contents)
            .map_err(|e| format!("Failed to deserialize save data: {}", e))?;

        info!("Game loaded from slot {} at {:?}", slot, path);
        Ok(data)
    }

    /// Check which slots have save data.
    #[allow(dead_code)]
    pub fn list_saves(&self) -> Vec<(usize, bool)> {
        (0..self.max_slots)
            .map(|slot| (slot, self.slot_path(slot).exists()))
            .collect()
    }

    /// Delete a save slot.
    #[allow(dead_code)]
    pub fn delete(&self, slot: usize) -> Result<(), String> {
        let path = self.slot_path(slot);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete save file: {}", e))?;
        }
        Ok(())
    }

    /// Auto-save to slot 0 (the dedicated auto-save slot).
    /// Logs a warning on failure instead of panicking.
    #[allow(dead_code)]
    pub fn auto_save(&self, data: &SaveData) {
        const AUTO_SAVE_SLOT: usize = 0;
        if let Err(err) = self.save(AUTO_SAVE_SLOT, data) {
            warn!("Auto-save failed: {}", err);
        }
    }
}

// ---------------------------------------------------------------------------
// Save plugin
// ---------------------------------------------------------------------------

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveSystem::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::core_plugin::{BestiaryEntry, Difficulty};
    use std::collections::HashMap;

    /// Build a fully-populated SaveData for testing. Every field is set to a
    /// non-default value so that round-trip tests are meaningful.
    fn make_test_save_data() -> SaveData {
        let mut story_flags = HashMap::new();
        story_flags.insert("tower_entered".to_string(), true);
        story_flags.insert("recruited_karis".to_string(), true);
        story_flags.insert("first_battle_won".to_string(), false);

        let mut unit_levels = HashMap::new();
        unit_levels.insert("adept".to_string(), (12, 4500));
        unit_levels.insert("karis".to_string(), (10, 3200));

        let mut djinn_assignments = HashMap::new();
        djinn_assignments.insert("flint".to_string(), "adept".to_string());
        djinn_assignments.insert("gust".to_string(), "karis".to_string());

        let mut equipment = HashMap::new();
        let mut adept_equip = HashMap::new();
        adept_equip.insert("weapon".to_string(), "long_sword".to_string());
        adept_equip.insert("armor".to_string(), "leather_armor".to_string());
        equipment.insert("adept".to_string(), adept_equip);

        let mut unit_hp_pp = HashMap::new();
        unit_hp_pp.insert("adept".to_string(), (180, 45));
        unit_hp_pp.insert("karis".to_string(), (140, 60));

        let party = Party {
            active: vec!["adept".to_string(), "karis".to_string()],
            bench: vec!["tyrell".to_string()],
            gold: 2500,
            inventory: vec![
                "herb".to_string(),
                "antidote".to_string(),
                "elixir".to_string(),
            ],
            equipment,
            unit_levels: unit_levels.clone(),
            unit_hp_pp,
            djinn_assignments: djinn_assignments.clone(),
            story_flags: story_flags.clone(),
            difficulty: Difficulty::Hard,
        };

        let party_data = vec![
            PartyMemberSaveData {
                unit_id: "adept".to_string(),
                hp: 180,
                pp: 45,
                level: 12,
                xp: 4500,
            },
            PartyMemberSaveData {
                unit_id: "karis".to_string(),
                hp: 140,
                pp: 60,
                level: 10,
                xp: 3200,
            },
        ];

        let mut bestiary = Bestiary::default();
        bestiary.entries.insert(
            "slime_01".to_string(),
            BestiaryEntry {
                enemy_id: "slime_01".to_string(),
                enemy_name: "Green Slime".to_string(),
                times_encountered: 5,
                times_defeated: 3,
                first_encountered: false,
            },
        );
        bestiary.entries.insert(
            "bat_01".to_string(),
            BestiaryEntry {
                enemy_id: "bat_01".to_string(),
                enemy_name: "Cave Bat".to_string(),
                times_encountered: 2,
                times_defeated: 1,
                first_encountered: false,
            },
        );

        SaveData {
            version: 1,
            party,
            party_data,
            unit_levels,
            djinn_assignments,
            story_flags,
            current_map: "tower_floor_3".to_string(),
            player_position: GridPosition::new(15, 22),
            tower_floor: 3,
            gold: 2500,
            play_time_secs: 7200.5,
            bestiary,
        }
    }

    /// Create a SaveSystem pointing at a unique temporary directory.
    fn make_test_save_system() -> (SaveSystem, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "vale_village_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sys = SaveSystem {
            save_dir: base.clone(),
            max_slots: 3,
        };
        (sys, base)
    }

    /// Clean up a temp directory (best-effort).
    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    // -------------------------------------------------------------------
    // Round-trip test
    // -------------------------------------------------------------------

    #[test]
    fn test_save_and_load_round_trip() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        sys.save(0, &data).expect("save should succeed");
        let loaded = sys.load(0).expect("load should succeed");

        // Version
        assert_eq!(loaded.version, data.version);

        // Party basics
        assert_eq!(loaded.party.active, data.party.active);
        assert_eq!(loaded.party.bench, data.party.bench);
        assert_eq!(loaded.party.gold, data.party.gold);
        assert_eq!(loaded.party.inventory, data.party.inventory);
        assert_eq!(loaded.party.equipment, data.party.equipment);
        assert_eq!(loaded.party.difficulty, data.party.difficulty);

        // Party data (per-member stats)
        assert_eq!(loaded.party_data.len(), data.party_data.len());
        for (l, d) in loaded.party_data.iter().zip(data.party_data.iter()) {
            assert_eq!(l.unit_id, d.unit_id);
            assert_eq!(l.hp, d.hp);
            assert_eq!(l.pp, d.pp);
            assert_eq!(l.level, d.level);
            assert_eq!(l.xp, d.xp);
        }

        // Unit levels
        assert_eq!(loaded.unit_levels, data.unit_levels);

        // Djinn assignments
        assert_eq!(loaded.djinn_assignments, data.djinn_assignments);

        // Story flags
        assert_eq!(loaded.story_flags, data.story_flags);

        // Map/position
        assert_eq!(loaded.current_map, data.current_map);
        assert_eq!(loaded.player_position, data.player_position);
        assert_eq!(loaded.tower_floor, data.tower_floor);

        // Economy / time
        assert_eq!(loaded.gold, data.gold);
        assert!((loaded.play_time_secs - data.play_time_secs).abs() < f64::EPSILON);

        // Bestiary
        assert_eq!(loaded.bestiary.entries.len(), data.bestiary.entries.len());
        for (id, entry) in &data.bestiary.entries {
            let loaded_entry = loaded
                .bestiary
                .entries
                .get(id)
                .expect("bestiary entry should exist");
            assert_eq!(loaded_entry.enemy_id, entry.enemy_id);
            assert_eq!(loaded_entry.enemy_name, entry.enemy_name);
            assert_eq!(loaded_entry.times_encountered, entry.times_encountered);
            assert_eq!(loaded_entry.times_defeated, entry.times_defeated);
            assert_eq!(loaded_entry.first_encountered, entry.first_encountered);
        }

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Invalid slot
    // -------------------------------------------------------------------

    #[test]
    fn test_save_to_invalid_slot() {
        let (sys, dir) = make_test_save_system();
        let data = SaveData::default();

        let result = sys.save(3, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid save slot"));

        let result = sys.save(100, &data);
        assert!(result.is_err());

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Load from empty slot
    // -------------------------------------------------------------------

    #[test]
    fn test_load_from_empty_slot() {
        let (sys, dir) = make_test_save_system();

        let result = sys.load(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No save file in slot"));

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Available saves
    // -------------------------------------------------------------------

    #[test]
    fn test_available_saves() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        // Initially all slots are empty
        let saves = sys.list_saves();
        assert_eq!(saves.len(), 3);
        assert!(!saves[0].1);
        assert!(!saves[1].1);
        assert!(!saves[2].1);

        // Save to slots 0 and 2
        sys.save(0, &data).expect("save slot 0");
        sys.save(2, &data).expect("save slot 2");

        let saves = sys.list_saves();
        assert!(saves[0].1, "slot 0 should have a save");
        assert!(!saves[1].1, "slot 1 should be empty");
        assert!(saves[2].1, "slot 2 should have a save");

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Delete save
    // -------------------------------------------------------------------

    #[test]
    fn test_delete_save() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        sys.save(1, &data).expect("save should succeed");
        assert!(sys.load(1).is_ok(), "load after save should succeed");

        sys.delete(1).expect("delete should succeed");
        let result = sys.load(1);
        assert!(result.is_err(), "load after delete should fail");
        assert!(result.unwrap_err().contains("No save file in slot"));

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Story flags preservation
    // -------------------------------------------------------------------

    #[test]
    fn test_save_preserves_story_flags() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        sys.save(0, &data).expect("save should succeed");
        let loaded = sys.load(0).expect("load should succeed");

        assert_eq!(loaded.story_flags.len(), 3);
        assert_eq!(loaded.story_flags.get("tower_entered"), Some(&true));
        assert_eq!(loaded.story_flags.get("recruited_karis"), Some(&true));
        assert_eq!(loaded.story_flags.get("first_battle_won"), Some(&false));

        // Also verify the party-level story_flags mirror
        assert_eq!(loaded.party.story_flags.get("tower_entered"), Some(&true));
        assert_eq!(loaded.party.story_flags.get("recruited_karis"), Some(&true));
        assert_eq!(
            loaded.party.story_flags.get("first_battle_won"),
            Some(&false)
        );

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Inventory / equipment preservation
    // -------------------------------------------------------------------

    #[test]
    fn test_save_preserves_inventory() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        sys.save(0, &data).expect("save should succeed");
        let loaded = sys.load(0).expect("load should succeed");

        // Inventory items
        assert_eq!(loaded.party.inventory.len(), 3);
        assert!(loaded.party.inventory.contains(&"herb".to_string()));
        assert!(loaded.party.inventory.contains(&"antidote".to_string()));
        assert!(loaded.party.inventory.contains(&"elixir".to_string()));

        // Equipment
        let adept_equip = loaded
            .party
            .equipment
            .get("adept")
            .expect("adept equipment should exist");
        assert_eq!(adept_equip.get("weapon"), Some(&"long_sword".to_string()));
        assert_eq!(adept_equip.get("armor"), Some(&"leather_armor".to_string()));

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Bestiary preservation
    // -------------------------------------------------------------------

    #[test]
    fn test_save_preserves_bestiary() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        sys.save(0, &data).expect("save should succeed");
        let loaded = sys.load(0).expect("load should succeed");

        assert_eq!(loaded.bestiary.entries.len(), 2);

        let slime = loaded
            .bestiary
            .entries
            .get("slime_01")
            .expect("slime entry should exist");
        assert_eq!(slime.enemy_name, "Green Slime");
        assert_eq!(slime.times_encountered, 5);
        assert_eq!(slime.times_defeated, 3);
        assert!(!slime.first_encountered);

        let bat = loaded
            .bestiary
            .entries
            .get("bat_01")
            .expect("bat entry should exist");
        assert_eq!(bat.enemy_name, "Cave Bat");
        assert_eq!(bat.times_encountered, 2);
        assert_eq!(bat.times_defeated, 1);
        assert!(!bat.first_encountered);

        cleanup(&dir);
    }

    // -------------------------------------------------------------------
    // Difficulty preservation
    // -------------------------------------------------------------------

    #[test]
    fn test_save_preserves_difficulty() {
        let (sys, dir) = make_test_save_system();
        let data = make_test_save_data();

        // Confirm the test data uses Hard difficulty
        assert_eq!(data.party.difficulty, Difficulty::Hard);

        sys.save(0, &data).expect("save should succeed");
        let loaded = sys.load(0).expect("load should succeed");

        assert_eq!(loaded.party.difficulty, Difficulty::Hard);

        cleanup(&dir);
    }
}
