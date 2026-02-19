//! Turn order calculation.
//!
//! Ported from TypeScript `turn-order.ts`. Pure function, deterministic with PRNG.

use crate::battle::types::{BattleUnit, UnitSide};
use rand::Rng;

/// Calculate turn order. Returns unit IDs sorted by descending speed,
/// player-before-enemy tiebreak, then random tiebreak. KO'd units excluded.
pub fn calculate_turn_order(units: &[BattleUnit], rng: &mut impl Rng) -> Vec<u32> {
    let mut entries: Vec<(u32, i32, UnitSide, f32)> = units
        .iter()
        .filter(|u| u.is_alive())
        .map(|u| (u.id, u.spd, u.side, rng.r#gen()))
        .collect();

    entries.sort_by(|a, b| {
        let spd_cmp = b.1.cmp(&a.1);
        if spd_cmp != std::cmp::Ordering::Equal { return spd_cmp; }

        let side_a = if a.2 == UnitSide::Player { 0 } else { 1 };
        let side_b = if b.2 == UnitSide::Player { 0 } else { 1 };
        let side_cmp = side_a.cmp(&side_b);
        if side_cmp != std::cmp::Ordering::Equal { return side_cmp; }

        a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal)
    });

    entries.into_iter().map(|(id, _, _, _)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::types::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_unit(id: u32, spd: i32, side: UnitSide) -> BattleUnit {
        BattleUnit {
            id, name: format!("Unit{}", id), side,
            element: Element::Venus, level: 5,
            hp: 100, max_hp: 100, pp: 50, max_pp: 50,
            atk: 10, def: 10, mag: 10, spd, luck: 5,
            status_effects: vec![], ability_ids: vec![], djinn_ids: vec![],
            damage_taken: 0, damage_dealt: 0, xp: 0,
            growth_rates: GrowthRates::default(),
        }
    }

    #[test]
    fn test_faster_goes_first() {
        let units = vec![make_unit(1, 10, UnitSide::Player), make_unit(2, 20, UnitSide::Enemy)];
        let mut rng = StdRng::seed_from_u64(42);
        let order = calculate_turn_order(&units, &mut rng);
        assert_eq!(order[0], 2);
    }

    #[test]
    fn test_player_before_enemy_on_tie() {
        let units = vec![make_unit(1, 15, UnitSide::Enemy), make_unit(2, 15, UnitSide::Player)];
        let mut rng = StdRng::seed_from_u64(42);
        let order = calculate_turn_order(&units, &mut rng);
        assert_eq!(order[0], 2);
    }

    #[test]
    fn test_ko_units_excluded() {
        let mut units = vec![make_unit(1, 20, UnitSide::Player), make_unit(2, 30, UnitSide::Enemy)];
        units[1].hp = 0;
        let mut rng = StdRng::seed_from_u64(42);
        let order = calculate_turn_order(&units, &mut rng);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], 1);
    }
}
