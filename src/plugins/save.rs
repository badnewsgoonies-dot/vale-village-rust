use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::battle::types::{BattleUnit, UnitSide};
use crate::components::world::{GridPosition, Player};
use crate::plugins::core_plugin::{GameData, GameState, Party};

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
        }
    }
}

impl SaveData {
    /// Build save data from the current Bevy world/resources.
    pub fn from_game_state(world: &mut World) -> Self {
        let party = world.get_resource::<Party>().cloned().unwrap_or_default();
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
