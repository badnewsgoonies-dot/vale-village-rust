use bevy::prelude::*;
use rand::Rng;

use super::core_plugin::GameState;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Resource, Reflect)]
pub struct TowerState {
    pub current_floor: u8,
    pub max_floor_reached: u8,
    pub is_active: bool,
    pub floors_cleared: Vec<u8>,
}

impl Default for TowerState {
    fn default() -> Self {
        Self {
            current_floor: 1,
            max_floor_reached: 1,
            is_active: false,
            floors_cleared: Vec::new(),
        }
    }
}

/// Marks whether a tower battle is currently in progress.
///
/// When the player enters a battle from inside the tower, this flag is set to
/// `true`. After the battle resolves and the game transitions back to
/// `GameState::Overworld`, the `check_tower_victory` system inspects this flag
/// to decide whether to advance the tower floor.
#[derive(Debug, Clone, Default, Resource, Reflect)]
pub struct TowerBattleActive(pub bool);

// ---------------------------------------------------------------------------
// Floor definitions
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct FloorDefinition {
    pub floor_number: u8,
    pub encounter_pool: Vec<String>,
    pub enemy_count_range: (usize, usize),
    pub enemy_level_bonus: i32,
    pub is_boss_floor: bool,
    pub boss_id: Option<String>,
    pub floor_reward_gold: u32,
    pub floor_reward_items: Vec<String>,
}

/// Build the 10 tower floor definitions with escalating difficulty.
#[allow(dead_code)]
pub fn build_floor_definitions() -> Vec<FloorDefinition> {
    let tier1: Vec<String> = vec![
        "mercury-slime".into(),
        "venus-wolf".into(),
        "mars-bandit".into(),
    ];
    let tier2: Vec<String> = vec![
        "jupiter-hawk".into(),
        "venus-golem".into(),
        "mars-lizard".into(),
    ];
    let tier3: Vec<String> = vec![
        "shadow-wyrm".into(),
        "storm-elemental".into(),
        "iron-golem".into(),
    ];

    vec![
        // Floors 1-3: Easy
        FloorDefinition {
            floor_number: 1,
            encounter_pool: tier1.clone(),
            enemy_count_range: (1, 2),
            enemy_level_bonus: 0,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 50,
            floor_reward_items: vec![],
        },
        FloorDefinition {
            floor_number: 2,
            encounter_pool: tier1.clone(),
            enemy_count_range: (1, 2),
            enemy_level_bonus: 0,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 75,
            floor_reward_items: vec![],
        },
        FloorDefinition {
            floor_number: 3,
            encounter_pool: tier1,
            enemy_count_range: (1, 2),
            enemy_level_bonus: 0,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 100,
            floor_reward_items: vec![],
        },
        // Floors 4-6: Medium
        FloorDefinition {
            floor_number: 4,
            encounter_pool: tier2.clone(),
            enemy_count_range: (2, 3),
            enemy_level_bonus: 2,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 150,
            floor_reward_items: vec![],
        },
        // Floor 5: Mini-boss
        FloorDefinition {
            floor_number: 5,
            encounter_pool: tier2.clone(),
            enemy_count_range: (2, 3),
            enemy_level_bonus: 2,
            is_boss_floor: true,
            boss_id: Some("slaver-captain".into()),
            floor_reward_gold: 200,
            floor_reward_items: vec!["elixir".into()],
        },
        FloorDefinition {
            floor_number: 6,
            encounter_pool: tier2,
            enemy_count_range: (2, 3),
            enemy_level_bonus: 2,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 250,
            floor_reward_items: vec![],
        },
        // Floors 7-9: Hard
        FloorDefinition {
            floor_number: 7,
            encounter_pool: tier3.clone(),
            enemy_count_range: (2, 4),
            enemy_level_bonus: 4,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 300,
            floor_reward_items: vec![],
        },
        FloorDefinition {
            floor_number: 8,
            encounter_pool: tier3.clone(),
            enemy_count_range: (2, 4),
            enemy_level_bonus: 4,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 400,
            floor_reward_items: vec![],
        },
        FloorDefinition {
            floor_number: 9,
            encounter_pool: tier3,
            enemy_count_range: (2, 4),
            enemy_level_bonus: 4,
            is_boss_floor: false,
            boss_id: None,
            floor_reward_gold: 500,
            floor_reward_items: vec![],
        },
        // Floor 10: Final boss
        FloorDefinition {
            floor_number: 10,
            encounter_pool: vec![],
            enemy_count_range: (1, 1),
            enemy_level_bonus: 4,
            is_boss_floor: true,
            boss_id: Some("dark-overlord".into()),
            floor_reward_gold: 500,
            floor_reward_items: vec!["psy-crystal".into(), "elixir".into()],
        },
    ]
}

// ---------------------------------------------------------------------------
// Encounter generation
// ---------------------------------------------------------------------------

/// Generate a floor encounter: returns a Vec of (enemy_id, level_bonus) pairs.
///
/// On boss floors the boss is always included. Regular enemies are drawn from
/// the floor's encounter pool.
#[allow(dead_code)]
pub fn generate_floor_encounter(floor: &FloorDefinition, rng: &mut impl Rng) -> Vec<(String, i32)> {
    let mut enemies: Vec<(String, i32)> = Vec::new();

    // Add boss first if this is a boss floor
    if floor.is_boss_floor
        && let Some(ref boss_id) = floor.boss_id
    {
        enemies.push((boss_id.clone(), floor.enemy_level_bonus));
    }

    // Fill remaining slots from the encounter pool
    if !floor.encounter_pool.is_empty() {
        let (min, max) = floor.enemy_count_range;
        let count = if min >= max {
            min
        } else {
            rng.gen_range(min..=max)
        };

        for _ in 0..count {
            let idx = rng.gen_range(0..floor.encounter_pool.len());
            enemies.push((floor.encounter_pool[idx].clone(), floor.enemy_level_bonus));
        }
    }

    enemies
}

// ---------------------------------------------------------------------------
// Floor advancement
// ---------------------------------------------------------------------------

#[allow(dead_code)]
const MAX_FLOOR: u8 = 10;

/// Advance the tower to the next floor.
///
/// Marks the current floor as cleared, increments `current_floor`, and updates
/// `max_floor_reached`. Returns `Some(new_floor)` on success, or `None` if the
/// tower has already reached the final floor.
#[allow(dead_code)]
pub fn advance_floor(tower_state: &mut TowerState) -> Option<u8> {
    if tower_state.current_floor >= MAX_FLOOR {
        return None;
    }

    // Mark current floor as cleared
    if !tower_state
        .floors_cleared
        .contains(&tower_state.current_floor)
    {
        tower_state.floors_cleared.push(tower_state.current_floor);
    }

    tower_state.current_floor += 1;

    if tower_state.current_floor > tower_state.max_floor_reached {
        tower_state.max_floor_reached = tower_state.current_floor;
    }

    Some(tower_state.current_floor)
}

// ---------------------------------------------------------------------------
// Post-battle progression
// ---------------------------------------------------------------------------

/// System that runs on `OnEnter(GameState::Overworld)` to handle tower
/// advancement after a battle victory.
///
/// When `TowerBattleActive` is `true`, this system:
/// 1. Calls `advance_floor` to move to the next floor.
/// 2. If all 10 floors have been cleared (`current_floor > MAX_FLOOR` or
///    `advance_floor` returns `None` because floor 10 was the current floor),
///    logs a completion message and deactivates the tower.
/// 3. Resets the `TowerBattleActive` flag to `false`.
pub fn check_tower_victory(
    mut tower_state: ResMut<TowerState>,
    mut tower_battle_active: ResMut<TowerBattleActive>,
) {
    if !tower_battle_active.0 {
        return;
    }

    let result = advance_floor(&mut tower_state);

    match result {
        Some(new_floor) => {
            info!("Tower advanced to floor {}/{}", new_floor, MAX_FLOOR);
        }
        None => {
            // current_floor was already at MAX_FLOOR — mark it cleared manually
            // since advance_floor won't do it when at the cap.
            if !tower_state.floors_cleared.contains(&MAX_FLOOR) {
                tower_state.floors_cleared.push(MAX_FLOOR);
            }
            info!("Tower of Trials completed!");
            tower_state.is_active = false;
        }
    }

    tower_battle_active.0 = false;
}

/// Returns a human-readable summary of the tower progress after a floor clear.
///
/// - If all 10 floors have been cleared, returns `"Tower of Trials conquered!"`.
/// - Otherwise, returns `"Floor X/10 cleared"` where X is the most recently
///   cleared floor.
#[allow(dead_code)]
pub fn tower_completion_check(tower_state: &TowerState) -> String {
    if tower_state.floors_cleared.len() >= MAX_FLOOR as usize {
        "Tower of Trials conquered!".to_string()
    } else {
        let last_cleared = tower_state.floors_cleared.last().copied().unwrap_or(0);
        format!("Floor {}/{} cleared", last_cleared, MAX_FLOOR)
    }
}

/// Reset the tower to its default state (for a new game).
#[allow(dead_code)]
pub fn reset_tower(tower_state: &mut TowerState) {
    *tower_state = TowerState::default();
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TowerState>()
            .init_resource::<TowerBattleActive>()
            .add_systems(OnEnter(GameState::Overworld), check_tower_victory);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn test_floor_definitions_count() {
        let floors = build_floor_definitions();
        assert_eq!(floors.len(), 10);
        for (i, floor) in floors.iter().enumerate() {
            assert_eq!(floor.floor_number, (i + 1) as u8);
        }
    }

    #[test]
    fn test_boss_floors() {
        let floors = build_floor_definitions();

        // Floor 5 is a mini-boss floor
        let floor5 = &floors[4];
        assert!(floor5.is_boss_floor);
        assert_eq!(floor5.boss_id, Some("slaver-captain".to_string()));
        assert!(floor5.floor_reward_items.contains(&"elixir".to_string()));

        // Floor 10 is the final boss floor
        let floor10 = &floors[9];
        assert!(floor10.is_boss_floor);
        assert_eq!(floor10.boss_id, Some("dark-overlord".to_string()));
        assert!(
            floor10
                .floor_reward_items
                .contains(&"psy-crystal".to_string())
        );
        assert!(floor10.floor_reward_items.contains(&"elixir".to_string()));

        // Other floors are NOT boss floors
        for (i, floor) in floors.iter().enumerate() {
            if i != 4 && i != 9 {
                assert!(
                    !floor.is_boss_floor,
                    "Floor {} should not be a boss floor",
                    floor.floor_number
                );
            }
        }
    }

    #[test]
    fn test_advance_floor() {
        let mut state = TowerState::default();
        assert_eq!(state.current_floor, 1);
        assert_eq!(state.max_floor_reached, 1);

        // Advance from floor 1 to floor 2
        let result = advance_floor(&mut state);
        assert_eq!(result, Some(2));
        assert_eq!(state.current_floor, 2);
        assert_eq!(state.max_floor_reached, 2);
        assert!(state.floors_cleared.contains(&1));

        // Advance all the way to floor 10
        for expected in 3..=10 {
            let result = advance_floor(&mut state);
            assert_eq!(result, Some(expected));
        }
        assert_eq!(state.current_floor, 10);
        assert_eq!(state.max_floor_reached, 10);

        // Cannot advance past floor 10
        let result = advance_floor(&mut state);
        assert_eq!(result, None);
        assert_eq!(state.current_floor, 10);
    }

    #[test]
    fn test_generate_encounter() {
        let floors = build_floor_definitions();
        let mut rng = StdRng::seed_from_u64(42);

        // Test a regular floor (floor 1: 1-2 enemies)
        let floor1 = &floors[0];
        let encounter = generate_floor_encounter(floor1, &mut rng);
        assert!(
            !encounter.is_empty(),
            "Encounter should have at least one enemy"
        );
        assert!(
            encounter.len() >= floor1.enemy_count_range.0
                && encounter.len() <= floor1.enemy_count_range.1,
            "Floor 1 encounter count {} should be within range {:?}",
            encounter.len(),
            floor1.enemy_count_range,
        );

        // Test a boss floor (floor 5: boss + 2-3 enemies)
        let floor5 = &floors[4];
        let encounter = generate_floor_encounter(floor5, &mut rng);
        assert!(
            encounter.len() > floor5.enemy_count_range.0,
            "Boss floor should include the boss plus regular enemies"
        );
        // First enemy should be the boss
        assert_eq!(encounter[0].0, "slaver-captain");

        // Test floor 10: boss only (empty encounter pool)
        let floor10 = &floors[9];
        let encounter = generate_floor_encounter(floor10, &mut rng);
        assert_eq!(encounter.len(), 1, "Floor 10 should have only the boss");
        assert_eq!(encounter[0].0, "dark-overlord");
    }

    // -----------------------------------------------------------------------
    // Post-battle progression tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_tower_victory_advances_floor() {
        let mut tower_state = TowerState {
            current_floor: 3,
            max_floor_reached: 3,
            is_active: true,
            floors_cleared: vec![1, 2],
        };
        let mut active = TowerBattleActive(true);

        // Simulate the system logic directly (pure-function style)
        check_tower_victory_logic(&mut tower_state, &mut active);

        assert_eq!(tower_state.current_floor, 4);
        assert_eq!(tower_state.max_floor_reached, 4);
        assert!(tower_state.floors_cleared.contains(&3));
        assert!(
            tower_state.is_active,
            "Tower should still be active mid-run"
        );
        assert!(!active.0, "TowerBattleActive should be reset to false");
    }

    #[test]
    fn test_check_tower_victory_completes_tower() {
        let mut tower_state = TowerState {
            current_floor: 10,
            max_floor_reached: 10,
            is_active: true,
            floors_cleared: (1..10).collect(),
        };
        let mut active = TowerBattleActive(true);

        check_tower_victory_logic(&mut tower_state, &mut active);

        // advance_floor returns None at floor 10, so current_floor stays 10
        assert_eq!(tower_state.current_floor, 10);
        assert!(
            tower_state.floors_cleared.contains(&10),
            "Floor 10 should be marked cleared"
        );
        assert!(
            !tower_state.is_active,
            "Tower should be deactivated on completion"
        );
        assert!(!active.0);
    }

    #[test]
    fn test_check_tower_victory_noop_when_inactive() {
        let mut tower_state = TowerState {
            current_floor: 5,
            max_floor_reached: 5,
            is_active: true,
            floors_cleared: vec![1, 2, 3, 4],
        };
        let mut active = TowerBattleActive(false);

        check_tower_victory_logic(&mut tower_state, &mut active);

        // Nothing should change since the flag was false
        assert_eq!(tower_state.current_floor, 5);
        assert_eq!(tower_state.floors_cleared.len(), 4);
        assert!(tower_state.is_active);
    }

    #[test]
    fn test_reset_tower() {
        let mut state = TowerState {
            current_floor: 7,
            max_floor_reached: 7,
            is_active: true,
            floors_cleared: vec![1, 2, 3, 4, 5, 6],
        };

        reset_tower(&mut state);

        assert_eq!(state.current_floor, 1);
        assert_eq!(state.max_floor_reached, 1);
        assert!(!state.is_active);
        assert!(state.floors_cleared.is_empty());
    }

    #[test]
    fn test_tower_completion_check_mid_tower() {
        let state = TowerState {
            current_floor: 4,
            max_floor_reached: 4,
            is_active: true,
            floors_cleared: vec![1, 2, 3],
        };

        let msg = tower_completion_check(&state);
        assert_eq!(msg, "Floor 3/10 cleared");
    }

    #[test]
    fn test_tower_completion_check_all_cleared() {
        let state = TowerState {
            current_floor: 10,
            max_floor_reached: 10,
            is_active: false,
            floors_cleared: (1..=10).collect(),
        };

        let msg = tower_completion_check(&state);
        assert_eq!(msg, "Tower of Trials conquered!");
    }

    #[test]
    fn test_tower_completion_check_no_floors_cleared() {
        let state = TowerState::default();
        let msg = tower_completion_check(&state);
        assert_eq!(msg, "Floor 0/10 cleared");
    }

    /// Helper that mirrors the `check_tower_victory` system logic without
    /// requiring a full Bevy ECS world.
    fn check_tower_victory_logic(
        tower_state: &mut TowerState,
        tower_battle_active: &mut TowerBattleActive,
    ) {
        if !tower_battle_active.0 {
            return;
        }

        let result = advance_floor(tower_state);

        match result {
            Some(_new_floor) => {}
            None => {
                if !tower_state.floors_cleared.contains(&MAX_FLOOR) {
                    tower_state.floors_cleared.push(MAX_FLOOR);
                }
                tower_state.is_active = false;
            }
        }

        tower_battle_active.0 = false;
    }
}
