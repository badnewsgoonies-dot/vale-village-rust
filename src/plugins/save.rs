use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::plugins::core_plugin::Party;

// ---------------------------------------------------------------------------
// Save file structure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub party: Party,
    /// Per-unit level/xp state: unit_id -> (level, xp).
    pub unit_levels: HashMap<String, (u8, u32)>,
    /// Djinn ownership: djinn_id -> unit_id.
    pub djinn_assignments: HashMap<String, String>,
    /// Story progress flags.
    pub story_flags: HashMap<String, bool>,
    /// Current map the player is on.
    pub current_map: String,
    /// Player position on the current map.
    pub player_x: i32,
    pub player_y: i32,
    /// Tower floor progress.
    pub tower_floor: u8,
    /// Play time in seconds.
    pub play_time_secs: f64,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: 1,
            party: Party::default(),
            unit_levels: HashMap::new(),
            djinn_assignments: HashMap::new(),
            story_flags: HashMap::new(),
            current_map: "village".into(),
            player_x: 5,
            player_y: 5,
            tower_floor: 0,
            play_time_secs: 0.0,
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
    pub fn list_saves(&self) -> Vec<(usize, bool)> {
        (0..self.max_slots)
            .map(|slot| (slot, self.slot_path(slot).exists()))
            .collect()
    }

    /// Delete a save slot.
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
