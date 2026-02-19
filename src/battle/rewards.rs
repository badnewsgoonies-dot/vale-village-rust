//! Battle reward calculation and XP distribution.
//!
//! Ported from TypeScript `rewards.ts` and `xp.ts`. Pure functions.

use crate::battle::types::{
    constants, BattleRewards, BattleUnit, GrowthRates, LevelUpEvent, StatGains,
};

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

/// Calculate rewards from a battle using enemy base_xp and base_gold from their data.
pub fn calculate_battle_rewards(
    enemy_xp_gold: &[(u32, u32)], // (base_xp, base_gold) per enemy
    party_size: u32,
    survivor_count: u32,
) -> BattleRewards {
    let total_xp: u32 = enemy_xp_gold.iter().map(|(xp, _)| xp).sum();
    let total_gold: u32 = enemy_xp_gold.iter().map(|(_, gold)| gold).sum();
    let xp_per_unit = if party_size > 0 { total_xp / party_size } else { 0 };

    BattleRewards {
        total_xp,
        total_gold,
        xp_per_unit,
        party_size,
        survivor_count,
        all_survived: party_size > 0 && survivor_count == party_size,
        enemies_defeated: enemy_xp_gold.len() as u32,
        item_drops: Vec::new(),
    }
}

pub fn distribute_rewards(party: &mut [BattleUnit], rewards: &BattleRewards) -> Vec<LevelUpEvent> {
    let mut level_ups = Vec::new();

    for unit in party.iter_mut() {
        if unit.level >= constants::MAX_LEVEL { continue; }

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

            level_ups.push(LevelUpEvent {
                unit_id: unit.id,
                unit_name: unit.name.clone(),
                old_level,
                new_level,
                stat_gains,
                new_abilities: vec![],
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
}
