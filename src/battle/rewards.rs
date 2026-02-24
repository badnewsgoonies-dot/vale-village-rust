//! Battle reward calculation and XP distribution.
//!
//! Ported from TypeScript `rewards.ts` and `xp.ts`. Pure functions.

use crate::battle::types::{
    BattleRewards, BattleUnit, GrowthRates, LevelUpEvent, StatGains, constants,
};
use rand::Rng;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// XP / Level
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn xp_for_level(level: u8) -> u32 {
    let idx = (level as usize).clamp(1, constants::MAX_LEVEL as usize);
    constants::XP_CURVE[idx]
}

pub fn level_from_xp(xp: u32) -> u8 {
    let mut result: u8 = 1;
    for lvl in 1..=constants::MAX_LEVEL {
        if xp >= constants::XP_CURVE[lvl as usize] {
            result = lvl;
        } else {
            break;
        }
    }
    result
}

pub fn add_xp(unit: &BattleUnit, xp_gain: u32) -> (u32, u8, bool) {
    let new_xp = unit.xp.saturating_add(xp_gain);
    let new_level = level_from_xp(new_xp);
    let leveled_up = new_level > unit.level;
    (new_xp, new_level, leveled_up)
}

pub fn calculate_stat_gains(growth: &GrowthRates, old_level: u8, new_level: u8) -> StatGains {
    let diff = (new_level as i32 - old_level as i32).max(0);
    StatGains {
        hp: growth.hp * diff,
        pp: growth.pp * diff,
        atk: growth.atk * diff,
        def: growth.def * diff,
        mag: growth.mag * diff,
        spd: growth.spd * diff,
    }
}

// ---------------------------------------------------------------------------
// Item drops
// ---------------------------------------------------------------------------

const COMMON_ITEMS: &[&str] = &["herb", "antidote"];
const UNCOMMON_ITEMS: &[&str] = &["nut", "potion"];
const RARE_ITEMS: &[&str] = &["psy-crystal", "elixir"];

const COMMON_DROP_CHANCE: f64 = 0.40;
const UNCOMMON_DROP_CHANCE: f64 = 0.15;
const RARE_DROP_CHANCE: f64 = 0.05;

/// Determine item drops from defeated enemies. Each enemy independently rolls
/// for a common, uncommon, or rare drop (chances do not overlap).
pub fn determine_item_drops(enemy_count: u32, rng: &mut impl Rng) -> Vec<String> {
    let mut drops = Vec::new();

    for _ in 0..enemy_count {
        let roll: f64 = rng.r#gen();
        if roll < RARE_DROP_CHANCE {
            let idx = rng.gen_range(0..RARE_ITEMS.len());
            drops.push(RARE_ITEMS[idx].to_string());
        } else if roll < RARE_DROP_CHANCE + UNCOMMON_DROP_CHANCE {
            let idx = rng.gen_range(0..UNCOMMON_ITEMS.len());
            drops.push(UNCOMMON_ITEMS[idx].to_string());
        } else if roll < RARE_DROP_CHANCE + UNCOMMON_DROP_CHANCE + COMMON_DROP_CHANCE {
            let idx = rng.gen_range(0..COMMON_ITEMS.len());
            drops.push(COMMON_ITEMS[idx].to_string());
        }
        // else: no drop for this enemy
    }

    drops
}

// ---------------------------------------------------------------------------
// Battle rewards
// ---------------------------------------------------------------------------

/// Calculate rewards from a battle using enemy base_xp and base_gold from their data.
/// Applies survivor bonuses: +20% XP and +10% gold if all party members survived.
pub fn calculate_battle_rewards(
    enemy_xp_gold: &[(u32, u32)], // (base_xp, base_gold) per enemy
    party_size: u32,
    survivor_count: u32,
    rng: &mut impl Rng,
) -> BattleRewards {
    let all_survived = party_size > 0 && survivor_count == party_size;
    let enemies_defeated = enemy_xp_gold.len() as u32;

    let raw_xp: u32 = enemy_xp_gold.iter().map(|(xp, _)| xp).sum();
    let raw_gold: u32 = enemy_xp_gold.iter().map(|(_, gold)| gold).sum();

    // Apply survivor bonuses
    let total_xp = if all_survived {
        (raw_xp as f64 * 1.2) as u32
    } else {
        raw_xp
    };
    let total_gold = if all_survived {
        (raw_gold as f64 * 1.1) as u32
    } else {
        raw_gold
    };

    let xp_per_unit = if party_size > 0 {
        total_xp / party_size
    } else {
        0
    };

    let item_drops = determine_item_drops(enemies_defeated, rng);

    BattleRewards {
        total_xp,
        total_gold,
        xp_per_unit,
        party_size,
        survivor_count,
        all_survived,
        enemies_defeated,
        item_drops,
    }
}

// ---------------------------------------------------------------------------
// Ability unlocks
// ---------------------------------------------------------------------------

/// Determine which abilities a unit unlocks when leveling up.
/// `ability_unlocks` is a slice of (unlock_level, ability_id) pairs.
/// Returns ability IDs where `unlock_level > old_level && unlock_level <= new_level`.
pub fn determine_level_abilities(
    _unit_ability_ids: &[String],
    old_level: u8,
    new_level: u8,
    ability_unlocks: &[(u8, String)],
) -> Vec<String> {
    ability_unlocks
        .iter()
        .filter(|(unlock_level, _)| *unlock_level > old_level && *unlock_level <= new_level)
        .map(|(_, ability_id)| ability_id.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Reward distribution
// ---------------------------------------------------------------------------

/// Distribute XP rewards to party members and return level-up events.
/// `ability_unlocks` maps unit_id to a list of (unlock_level, ability_id) pairs.
pub fn distribute_rewards(
    party: &mut [BattleUnit],
    rewards: &BattleRewards,
    ability_unlocks: &HashMap<u32, Vec<(u8, String)>>,
) -> Vec<LevelUpEvent> {
    let mut level_ups = Vec::new();

    for unit in party.iter_mut() {
        if unit.level >= constants::MAX_LEVEL {
            continue;
        }

        let old_level = unit.level;
        let (new_xp, new_level, leveled_up) = add_xp(unit, rewards.xp_per_unit);
        unit.xp = new_xp;
        unit.level = new_level;

        if leveled_up {
            let stat_gains = calculate_stat_gains(&unit.growth_rates, old_level, new_level);
            unit.max_hp += stat_gains.hp;
            unit.hp = unit.max_hp;
            unit.max_pp += stat_gains.pp;
            unit.pp = unit.max_pp;
            unit.atk += stat_gains.atk;
            unit.def += stat_gains.def;
            unit.mag += stat_gains.mag;
            unit.spd += stat_gains.spd;

            let new_abilities = if let Some(unlocks) = ability_unlocks.get(&unit.id) {
                determine_level_abilities(&unit.ability_ids, old_level, new_level, unlocks)
            } else {
                vec![]
            };

            level_ups.push(LevelUpEvent {
                unit_id: unit.id,
                unit_name: unit.name.clone(),
                old_level,
                new_level,
                stat_gains,
                new_abilities,
            });
        }
    }
    level_ups
}

/// Flee chance: 50% base + 2% per speed advantage point, clamped [10%, 90%].
pub fn flee_chance(party_avg_speed: f32, enemy_avg_speed: f32) -> f32 {
    let diff = party_avg_speed - enemy_avg_speed;
    (constants::BASE_FLEE_CHANCE + diff * constants::SPEED_FLEE_BONUS).clamp(0.10, 0.90)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_test_unit(id: u32, level: u8, xp: u32) -> BattleUnit {
        use crate::battle::types::{Element, UnitSide};
        BattleUnit {
            id,
            name: format!("Unit{}", id),
            side: UnitSide::Player,
            element: Element::Venus,
            level,
            hp: 100,
            max_hp: 100,
            pp: 50,
            max_pp: 50,
            atk: 20,
            def: 15,
            mag: 18,
            spd: 12,
            luck: 10,
            status_effects: vec![],
            ability_ids: vec![],
            djinn_ids: vec![],
            damage_taken: 0,
            damage_dealt: 0,
            xp,
            growth_rates: GrowthRates {
                hp: 5,
                pp: 3,
                atk: 2,
                def: 2,
                mag: 2,
                spd: 1,
            },
        }
    }

    #[test]
    fn test_xp_curve() {
        assert_eq!(xp_for_level(1), 0);
        assert_eq!(xp_for_level(2), 100);
        assert_eq!(xp_for_level(20), 92800);
    }

    #[test]
    fn test_level_from_xp() {
        assert_eq!(level_from_xp(0), 1);
        assert_eq!(level_from_xp(100), 2);
        assert_eq!(level_from_xp(99999), 20);
    }

    #[test]
    fn test_flee_chance() {
        assert!((flee_chance(10.0, 10.0) - 0.50).abs() < 0.001);
        assert!((flee_chance(50.0, 10.0) - 0.90).abs() < 0.001);
        assert!((flee_chance(5.0, 50.0) - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_item_drops_some_items() {
        let mut rng = StdRng::seed_from_u64(42);
        let drops = determine_item_drops(20, &mut rng);
        // With 20 enemies, we should statistically get some drops
        assert!(
            !drops.is_empty(),
            "Expected some item drops from 20 enemies with seeded rng"
        );
        // All dropped items should be from known item lists
        let all_items: Vec<&str> = COMMON_ITEMS
            .iter()
            .chain(UNCOMMON_ITEMS.iter())
            .chain(RARE_ITEMS.iter())
            .copied()
            .collect();
        for drop in &drops {
            assert!(
                all_items.contains(&drop.as_str()),
                "Unknown drop item: {}",
                drop
            );
        }
    }

    #[test]
    fn test_no_item_drops_zero_enemies() {
        let mut rng = StdRng::seed_from_u64(42);
        let drops = determine_item_drops(0, &mut rng);
        assert!(
            drops.is_empty(),
            "Expected no drops from zero enemies, got {:?}",
            drops
        );
    }

    #[test]
    fn test_survivor_bonus_xp() {
        let mut rng = StdRng::seed_from_u64(42);
        let enemies = vec![(100, 50), (100, 50)]; // 200 total xp, 100 gold

        // All survived: should get 20% XP bonus, 10% gold bonus
        let rewards_all = calculate_battle_rewards(&enemies, 2, 2, &mut rng);
        assert!(rewards_all.all_survived);
        assert_eq!(rewards_all.total_xp, 240); // 200 * 1.2
        assert_eq!(rewards_all.total_gold, 110); // 100 * 1.1
        assert_eq!(rewards_all.xp_per_unit, 120); // 240 / 2

        // Not all survived: no bonus
        let mut rng2 = StdRng::seed_from_u64(42);
        let rewards_some = calculate_battle_rewards(&enemies, 2, 1, &mut rng2);
        assert!(!rewards_some.all_survived);
        assert_eq!(rewards_some.total_xp, 200);
        assert_eq!(rewards_some.total_gold, 100);
        assert_eq!(rewards_some.xp_per_unit, 100); // 200 / 2
    }

    #[test]
    fn test_level_abilities_unlock() {
        let unit_abilities: Vec<String> = vec!["fireball".to_string()];
        let ability_unlocks = vec![
            (2, "ice_shard".to_string()),
            (3, "thunder".to_string()),
            (5, "quake".to_string()),
        ];

        // Leveling from 2 to 4: should unlock abilities at level 3 (thunder)
        let unlocked = determine_level_abilities(&unit_abilities, 2, 4, &ability_unlocks);
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0], "thunder");

        // Leveling from 1 to 3: should unlock abilities at level 2 and 3
        let unlocked2 = determine_level_abilities(&unit_abilities, 1, 3, &ability_unlocks);
        assert_eq!(unlocked2.len(), 2);
        assert!(unlocked2.contains(&"ice_shard".to_string()));
        assert!(unlocked2.contains(&"thunder".to_string()));

        // Leveling from 4 to 4: no new abilities (no level change beyond old)
        let unlocked3 = determine_level_abilities(&unit_abilities, 4, 4, &ability_unlocks);
        assert!(unlocked3.is_empty());
    }

    #[test]
    fn test_distribute_rewards_with_ability_unlocks() {
        // Set up a unit at level 1 with 0 XP, about to level up to 2
        let mut party = vec![make_test_unit(1, 1, 0)];

        let rewards = BattleRewards {
            total_xp: 100,
            total_gold: 50,
            xp_per_unit: 100,
            party_size: 1,
            survivor_count: 1,
            all_survived: true,
            enemies_defeated: 1,
            item_drops: vec![],
        };

        let mut ability_unlocks = HashMap::new();
        ability_unlocks.insert(
            1,
            vec![(2, "flame_burst".to_string()), (5, "inferno".to_string())],
        );

        let level_ups = distribute_rewards(&mut party, &rewards, &ability_unlocks);
        assert_eq!(level_ups.len(), 1);
        assert_eq!(level_ups[0].old_level, 1);
        assert_eq!(level_ups[0].new_level, 2);
        assert_eq!(level_ups[0].new_abilities, vec!["flame_burst".to_string()]);
    }
}
