use bevy::prelude::*;
use std::collections::HashMap;

use super::core_plugin::GameState;

// ---------------------------------------------------------------------------
// SpriteHandles resource — maps unit/enemy/icon IDs to loaded image handles
// ---------------------------------------------------------------------------

/// Stores `Handle<Image>` references for all pre-loaded sprite assets, keyed
/// by the game-internal IDs used in the unit, enemy, and item registries.
#[derive(Resource, Default)]
pub struct SpriteHandles {
    /// Player unit sprites: unit_id → Handle<Image>
    pub units: HashMap<String, Handle<Image>>,
    /// Enemy sprites: enemy_id → Handle<Image>
    pub enemies: HashMap<String, Handle<Image>>,
    /// Miscellaneous icon sprites: icon_name → Handle<Image>
    #[allow(dead_code)]
    pub icons: HashMap<String, Handle<Image>>,
}

// ---------------------------------------------------------------------------
// Mapping helpers — connect game IDs to asset file paths
// ---------------------------------------------------------------------------

/// Returns the mapping from unit IDs to their placeholder sprite paths.
/// The path is relative to the `assets/` folder (Bevy convention).
fn unit_sprite_mappings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("adept", "sprites/placeholders/adept.png"),
        ("war-mage", "sprites/placeholders/war_mage.png"),
        ("mystic", "sprites/placeholders/mystic.png"),
        ("ranger", "sprites/placeholders/ranger.png"),
        ("sentinel", "sprites/placeholders/sentinel.png"),
        ("stormcaller", "sprites/placeholders/stormcaller.png"),
        // blaze, karis, tyrell, felix don't have unique placeholders yet;
        // map them to the closest thematic sprite as a fallback.
        ("blaze", "sprites/placeholders/war_mage.png"),
        ("karis", "sprites/placeholders/mystic.png"),
        ("tyrell", "sprites/placeholders/gladiator.png"),
        ("felix", "sprites/placeholders/sentinel.png"),
    ]
}

/// Returns the mapping from enemy IDs to their placeholder sprite paths.
/// Enemies without a unique placeholder are mapped to the closest match
/// based on creature type (wolf, slime, beetle, bandit, etc.).
fn enemy_sprite_mappings() -> Vec<(&'static str, &'static str)> {
    vec![
        // --- Slimes ---
        ("mercury-slime", "sprites/placeholders/slime.png"),
        // --- Wolves ---
        ("venus-wolf", "sprites/placeholders/wolf.png"),
        ("mars-wolf", "sprites/placeholders/wolf.png"),
        ("mercury-wolf", "sprites/placeholders/wolf.png"),
        ("jupiter-wolf", "sprites/placeholders/wolf.png"),
        // --- Bandits / Scouts ---
        ("mars-bandit", "sprites/placeholders/bandit.png"),
        ("earth-scout", "sprites/placeholders/bandit.png"),
        ("flame-scout", "sprites/placeholders/bandit.png"),
        ("frost-scout", "sprites/placeholders/bandit.png"),
        ("gale-scout", "sprites/placeholders/bandit.png"),
        // --- Sprites ---
        ("jupiter-sprite", "sprites/placeholders/sprite.png"),
        // --- Beetles ---
        ("venus-beetle", "sprites/placeholders/beetle.png"),
        // --- Bears (reuse gladiator as heavy melee placeholder) ---
        ("venus-bear", "sprites/placeholders/gladiator.png"),
        ("mars-bear", "sprites/placeholders/gladiator.png"),
        ("mercury-bear", "sprites/placeholders/gladiator.png"),
        ("jupiter-bear", "sprites/placeholders/gladiator.png"),
        // --- Support / Casters (reuse mystic placeholder) ---
        ("frost-mystic", "sprites/placeholders/mystic.png"),
        ("gale-priest", "sprites/placeholders/sprite.png"),
        ("stone-guardian", "sprites/placeholders/sentinel.png"),
        ("ember-cleric", "sprites/placeholders/war_mage.png"),
        ("earth-shaman", "sprites/placeholders/sentinel.png"),
        ("tide-enchanter", "sprites/placeholders/mystic.png"),
        ("frost-oracle", "sprites/placeholders/mystic.png"),
        // --- Soldiers (reuse gladiator) ---
        ("terra-soldier", "sprites/placeholders/gladiator.png"),
        ("blaze-soldier", "sprites/placeholders/gladiator.png"),
        ("tide-soldier", "sprites/placeholders/gladiator.png"),
        ("wind-soldier", "sprites/placeholders/gladiator.png"),
        // --- Captains (reuse bandit) ---
        ("stone-captain", "sprites/placeholders/bandit.png"),
        ("inferno-captain", "sprites/placeholders/bandit.png"),
        ("glacier-captain", "sprites/placeholders/bandit.png"),
        ("thunder-captain", "sprites/placeholders/bandit.png"),
        // --- Commanders (reuse gladiator) ---
        ("mountain-commander", "sprites/placeholders/gladiator.png"),
        ("fire-commander", "sprites/placeholders/gladiator.png"),
        ("storm-commander", "sprites/placeholders/gladiator.png"),
        ("gale-commander", "sprites/placeholders/gladiator.png"),
        // --- Wardens / Heralds ---
        ("terra-warden", "sprites/placeholders/sentinel.png"),
        ("flame-herald", "sprites/placeholders/war_mage.png"),
        // --- Bosses ---
        ("slaver-chief", "sprites/placeholders/bandit.png"),
        ("iron-warden", "sprites/placeholders/sentinel.png"),
        ("phoenix-lord", "sprites/placeholders/war_mage.png"),
        ("glacier-queen", "sprites/placeholders/mystic.png"),
        ("storm-tyrant", "sprites/placeholders/stormcaller.png"),
        ("earth-titan", "sprites/placeholders/elemental_guardian.png"),
        (
            "infernal-dragon",
            "sprites/placeholders/guardian_shard_fire.png",
        ),
        ("leviathan", "sprites/placeholders/guardian_shard_water.png"),
        (
            "vale-overlord",
            "sprites/placeholders/elemental_guardian.png",
        ),
    ]
}

// ---------------------------------------------------------------------------
// Public helper — look up a sprite handle by unit or enemy ID
// ---------------------------------------------------------------------------

/// Returns a clone of the `Handle<Image>` for the given unit ID, if loaded.
#[allow(dead_code)]
pub fn get_unit_sprite(handles: &SpriteHandles, unit_id: &str) -> Option<Handle<Image>> {
    handles.units.get(unit_id).cloned()
}

/// Returns a clone of the `Handle<Image>` for the given enemy ID, if loaded.
#[allow(dead_code)]
pub fn get_enemy_sprite(handles: &SpriteHandles, enemy_id: &str) -> Option<Handle<Image>> {
    handles.enemies.get(enemy_id).cloned()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Loads all placeholder sprite PNGs via the asset server and stores the
/// resulting handles in the `SpriteHandles` resource.
fn load_sprite_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut handles = SpriteHandles::default();

    // Load unit sprites
    for (id, path) in unit_sprite_mappings() {
        let handle: Handle<Image> = asset_server.load(path);
        handles.units.insert(id.into(), handle);
    }

    // Load enemy sprites
    for (id, path) in enemy_sprite_mappings() {
        let handle: Handle<Image> = asset_server.load(path);
        handles.enemies.insert(id.into(), handle);
    }

    info!(
        "Loaded sprite handles: {} units, {} enemies",
        handles.units.len(),
        handles.enemies.len()
    );

    commands.insert_resource(handles);
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Registers the sprite loading system that runs during the `Loading` state.
/// After loading, handles are available in the `SpriteHandles` resource for
/// any system that needs to assign sprites to entities.
pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), load_sprite_assets);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_sprite_mappings_cover_all_units() {
        let mappings = unit_sprite_mappings();
        let expected_ids = [
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
        for id in &expected_ids {
            assert!(
                mappings.iter().any(|(mid, _)| mid == id),
                "Missing unit sprite mapping for '{}'",
                id
            );
        }
    }

    #[test]
    fn test_unit_sprite_mappings_unique_ids() {
        let mappings = unit_sprite_mappings();
        let mut seen = std::collections::HashSet::new();
        for (id, _) in &mappings {
            assert!(
                seen.insert(*id),
                "Duplicate unit sprite mapping for '{}'",
                id
            );
        }
    }

    #[test]
    fn test_enemy_sprite_mappings_cover_known_enemies() {
        let mappings = enemy_sprite_mappings();
        // Spot-check a selection of enemy IDs from each category
        let spot_check = [
            "mercury-slime",
            "venus-wolf",
            "mars-bandit",
            "jupiter-sprite",
            "venus-beetle",
            "slaver-chief",
            "vale-overlord",
            "leviathan",
            "infernal-dragon",
            "earth-titan",
        ];
        for id in &spot_check {
            assert!(
                mappings.iter().any(|(mid, _)| mid == id),
                "Missing enemy sprite mapping for '{}'",
                id
            );
        }
    }

    #[test]
    fn test_enemy_sprite_mappings_unique_ids() {
        let mappings = enemy_sprite_mappings();
        let mut seen = std::collections::HashSet::new();
        for (id, _) in &mappings {
            assert!(
                seen.insert(*id),
                "Duplicate enemy sprite mapping for '{}'",
                id
            );
        }
    }

    #[test]
    fn test_all_sprite_paths_use_correct_prefix() {
        let unit_mappings = unit_sprite_mappings();
        let enemy_mappings = enemy_sprite_mappings();
        for (id, path) in unit_mappings.iter().chain(enemy_mappings.iter()) {
            assert!(
                path.starts_with("sprites/placeholders/"),
                "Sprite path for '{}' should start with 'sprites/placeholders/', got '{}'",
                id,
                path
            );
            assert!(
                path.ends_with(".png"),
                "Sprite path for '{}' should end with '.png', got '{}'",
                id,
                path
            );
        }
    }

    #[test]
    fn test_get_unit_sprite_returns_some_for_loaded() {
        let mut handles = SpriteHandles::default();
        let fake_handle = Handle::default();
        handles.units.insert("adept".into(), fake_handle.clone());

        let result = get_unit_sprite(&handles, "adept");
        assert!(result.is_some(), "Should find 'adept' in loaded handles");
    }

    #[test]
    fn test_get_unit_sprite_returns_none_for_missing() {
        let handles = SpriteHandles::default();
        let result = get_unit_sprite(&handles, "nonexistent-unit");
        assert!(result.is_none(), "Should return None for unloaded unit ID");
    }

    #[test]
    fn test_get_enemy_sprite_returns_some_for_loaded() {
        let mut handles = SpriteHandles::default();
        let fake_handle = Handle::default();
        handles
            .enemies
            .insert("mercury-slime".into(), fake_handle.clone());

        let result = get_enemy_sprite(&handles, "mercury-slime");
        assert!(
            result.is_some(),
            "Should find 'mercury-slime' in loaded handles"
        );
    }

    #[test]
    fn test_get_enemy_sprite_returns_none_for_missing() {
        let handles = SpriteHandles::default();
        let result = get_enemy_sprite(&handles, "nonexistent-enemy");
        assert!(result.is_none(), "Should return None for unloaded enemy ID");
    }

    #[test]
    fn test_unit_mappings_match_registry_count() {
        let mappings = unit_sprite_mappings();
        // The game has 10 playable units; we should have exactly 10 mappings
        assert_eq!(
            mappings.len(),
            10,
            "Expected 10 unit sprite mappings, got {}",
            mappings.len()
        );
    }

    #[test]
    fn test_enemy_mappings_cover_all_registry_enemies() {
        // Verify every enemy from the enemy registry has a sprite mapping
        let registry = crate::data::enemies::build_enemy_registry();
        let mappings = enemy_sprite_mappings();
        let mapped_ids: std::collections::HashSet<&str> =
            mappings.iter().map(|(id, _)| *id).collect();

        for enemy_id in registry.keys() {
            assert!(
                mapped_ids.contains(enemy_id.as_str()),
                "Enemy '{}' from registry has no sprite mapping",
                enemy_id
            );
        }
    }
}
