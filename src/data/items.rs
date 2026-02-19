use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::components::stats::Element;

// ---------------------------------------------------------------------------
// Item categories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    Consumable,
    KeyItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Weapon,
    Armor,
    Accessory,
    Shield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentTier {
    Basic,
    Bronze,
    Iron,
    Steel,
    Silver,
    Mythril,
    Legendary,
    Artifact,
}

// ---------------------------------------------------------------------------
// Stat bonuses for equipment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatBonus {
    #[serde(default)]
    pub atk: i32,
    #[serde(default)]
    pub def: i32,
    #[serde(default)]
    pub mag: i32,
    #[serde(default)]
    pub spd: i32,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub pp: i32,
}

// ---------------------------------------------------------------------------
// Item effects for consumables
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemEffect {
    #[serde(default)]
    pub hp_restore: i32,
    #[serde(default)]
    pub pp_restore: i32,
    #[serde(default)]
    pub removes_status: Vec<String>,
    #[serde(default)]
    pub revive: bool,
    #[serde(default)]
    pub damage_element: Option<Element>,
    #[serde(default)]
    pub damage_amount: i32,
}

// ---------------------------------------------------------------------------
// Item definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ItemCategory,
    pub cost: u32,
    pub effect: ItemEffect,
}

// ---------------------------------------------------------------------------
// Equipment definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentDefinition {
    pub id: String,
    pub name: String,
    pub slot: EquipmentSlot,
    pub tier: EquipmentTier,
    pub cost: u32,
    pub stat_bonus: StatBonus,
    /// Which elements can equip this (empty = all).
    pub allowed_elements: Vec<Element>,
    /// Ability ID unlocked by equipping this.
    pub unlocks_ability: Option<String>,
    pub description: String,
}

/// Build the consumable item registry.
pub fn build_item_registry() -> HashMap<String, ItemDefinition> {
    let mut m = HashMap::new();

    let items = vec![
        ItemDefinition {
            id: "potion".into(),
            name: "Potion".into(),
            description: "Restores HP to one ally.".into(),
            category: ItemCategory::Consumable,
            cost: 50,
            effect: ItemEffect {
                hp_restore: 120, pp_restore: 0, removes_status: vec![],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "mercury-mist-elixir".into(),
            name: "Mercury Mist Elixir".into(),
            description: "Restores a small amount of PP and heals minor HP.".into(),
            category: ItemCategory::Consumable,
            cost: 120,
            effect: ItemEffect {
                hp_restore: 80, pp_restore: 10, removes_status: vec![],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "mercury-water-of-life".into(),
            name: "Water of Life (Mercury)".into(),
            description: "Heals HP for a single ally significantly.".into(),
            category: ItemCategory::Consumable,
            cost: 420,
            effect: ItemEffect {
                hp_restore: 250, pp_restore: 0, removes_status: vec![],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "mercury-vial".into(),
            name: "Glacial Vial".into(),
            description: "A vial of freezing brine. Lowers enemy speed.".into(),
            category: ItemCategory::Consumable,
            cost: 90,
            effect: ItemEffect {
                hp_restore: 0, pp_restore: 0, removes_status: vec![],
                revive: false, damage_element: Some(Element::Mercury), damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "jupiter-zephyr-scroll".into(),
            name: "Zephyr Scroll".into(),
            description: "Grants a temporary speed buff to a single ally.".into(),
            category: ItemCategory::Consumable,
            cost: 140,
            effect: ItemEffect {
                hp_restore: 0, pp_restore: 0, removes_status: vec![],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "jupiter-lightning-flask".into(),
            name: "Lightning Flask".into(),
            description: "Deals small Jupiter-element magic damage to one enemy.".into(),
            category: ItemCategory::Consumable,
            cost: 200,
            effect: ItemEffect {
                hp_restore: 0, pp_restore: 0, removes_status: vec![],
                revive: false, damage_element: Some(Element::Jupiter), damage_amount: 120,
            },
        },
        ItemDefinition {
            id: "jupiter-hermes-water".into(),
            name: "Hermes' Water".into(),
            description: "Restores PP.".into(),
            category: ItemCategory::Consumable,
            cost: 260,
            effect: ItemEffect {
                hp_restore: 0, pp_restore: 15, removes_status: vec![],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "elixir".into(),
            name: "Elixir".into(),
            description: "Fully restores HP and PP for one ally.".into(),
            category: ItemCategory::Consumable,
            cost: 2000,
            effect: ItemEffect {
                hp_restore: 9999, pp_restore: 9999, removes_status: vec![],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "antidote".into(),
            name: "Antidote".into(),
            description: "Cures poison and other minor ailments.".into(),
            category: ItemCategory::Consumable,
            cost: 35,
            effect: ItemEffect {
                hp_restore: 0, pp_restore: 0,
                removes_status: vec!["poison".into()],
                revive: false, damage_element: None, damage_amount: 0,
            },
        },
        ItemDefinition {
            id: "revive-stone".into(),
            name: "Revive Stone".into(),
            description: "Revives a fallen ally with partial HP.".into(),
            category: ItemCategory::Consumable,
            cost: 500,
            effect: ItemEffect {
                hp_restore: 100, pp_restore: 0, removes_status: vec![],
                revive: true, damage_element: None, damage_amount: 0,
            },
        },
    ];

    for item in items {
        m.insert(item.id.clone(), item);
    }

    m
}

/// Build the equipment registry.
pub fn build_equipment_registry() -> HashMap<String, EquipmentDefinition> {
    let mut m = HashMap::new();

    let equipment = vec![
        // ===== SWORDS (Venus + Jupiter) =====
        EquipmentDefinition {
            id: "wooden-sword".into(), name: "Wooden Sword".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Basic, cost: 50,
            stat_bonus: StatBonus { atk: 5, ..Default::default() },
            allowed_elements: vec![Element::Venus, Element::Jupiter],
            unlocks_ability: None,
            description: "A basic wooden training sword.".into(),
        },
        EquipmentDefinition {
            id: "bronze-sword".into(), name: "Bronze Sword".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Bronze, cost: 120,
            stat_bonus: StatBonus { atk: 9, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "A sturdy bronze sword.".into(),
        },
        EquipmentDefinition {
            id: "iron-sword".into(), name: "Iron Sword".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Iron, cost: 200,
            stat_bonus: StatBonus { atk: 14, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "A reliable iron sword.".into(),
        },
        EquipmentDefinition {
            id: "steel-sword".into(), name: "Steel Sword".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Steel, cost: 500,
            stat_bonus: StatBonus { atk: 22, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "A well-forged steel sword.".into(),
        },
        EquipmentDefinition {
            id: "silver-blade".into(), name: "Silver Blade".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Silver, cost: 1200,
            stat_bonus: StatBonus { atk: 32, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "A gleaming silver blade.".into(),
        },
        EquipmentDefinition {
            id: "mythril-blade".into(), name: "Mythril Blade".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Mythril, cost: 3000,
            stat_bonus: StatBonus { atk: 45, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "A blade forged from mythril ore.".into(),
        },
        EquipmentDefinition {
            id: "gaia-blade".into(), name: "Gaia Blade".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Legendary, cost: 7500,
            stat_bonus: StatBonus { atk: 58, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "A legendary blade imbued with the power of Gaia.".into(),
        },
        EquipmentDefinition {
            id: "sol-blade".into(), name: "Sol Blade".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Artifact, cost: 15000,
            stat_bonus: StatBonus { atk: 72, ..Default::default() },
            allowed_elements: vec![Element::Venus],
            unlocks_ability: None,
            description: "The ultimate artifact blade of radiant power.".into(),
        },

        // ===== AXES (Mars) =====
        EquipmentDefinition {
            id: "wooden-axe".into(), name: "Wooden Axe".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Basic, cost: 60,
            stat_bonus: StatBonus { atk: 7, spd: -1, ..Default::default() },
            allowed_elements: vec![Element::Mars],
            unlocks_ability: None,
            description: "A basic wooden axe.".into(),
        },
        EquipmentDefinition {
            id: "battle-axe".into(), name: "Battle Axe".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Iron, cost: 280,
            stat_bonus: StatBonus { atk: 18, spd: -2, ..Default::default() },
            allowed_elements: vec![Element::Mars],
            unlocks_ability: None,
            description: "A heavy iron battle axe.".into(),
        },
        EquipmentDefinition {
            id: "great-axe".into(), name: "Great Axe".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Steel, cost: 800,
            stat_bonus: StatBonus { atk: 30, spd: -3, ..Default::default() },
            allowed_elements: vec![Element::Mars],
            unlocks_ability: None,
            description: "A massive steel great axe.".into(),
        },
        EquipmentDefinition {
            id: "titans-axe".into(), name: "Titan's Axe".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Legendary, cost: 9000,
            stat_bonus: StatBonus { atk: 65, def: 10, spd: -2, ..Default::default() },
            allowed_elements: vec![Element::Mars],
            unlocks_ability: None,
            description: "A legendary axe wielded by ancient titans.".into(),
        },

        // ===== STAVES (Mercury) =====
        EquipmentDefinition {
            id: "wooden-staff".into(), name: "Wooden Staff".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Basic, cost: 45,
            stat_bonus: StatBonus { mag: 5, ..Default::default() },
            allowed_elements: vec![Element::Mercury],
            unlocks_ability: None,
            description: "A basic wooden staff.".into(),
        },
        EquipmentDefinition {
            id: "arcane-rod".into(), name: "Arcane Rod".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Iron, cost: 180,
            stat_bonus: StatBonus { mag: 12, pp: 5, ..Default::default() },
            allowed_elements: vec![Element::Mercury],
            unlocks_ability: None,
            description: "A rod infused with arcane energy.".into(),
        },
        EquipmentDefinition {
            id: "crystal-staff".into(), name: "Crystal Staff".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Silver, cost: 1400,
            stat_bonus: StatBonus { mag: 28, pp: 10, ..Default::default() },
            allowed_elements: vec![Element::Mercury],
            unlocks_ability: None,
            description: "A staff topped with a gleaming crystal.".into(),
        },

        // ===== BOWS (Jupiter) =====
        EquipmentDefinition {
            id: "short-bow".into(), name: "Short Bow".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Basic, cost: 55,
            stat_bonus: StatBonus { atk: 4, spd: 2, ..Default::default() },
            allowed_elements: vec![Element::Jupiter],
            unlocks_ability: None,
            description: "A basic short bow.".into(),
        },
        EquipmentDefinition {
            id: "storm-bow".into(), name: "Storm Bow".into(),
            slot: EquipmentSlot::Weapon, tier: EquipmentTier::Steel, cost: 600,
            stat_bonus: StatBonus { atk: 20, spd: 4, ..Default::default() },
            allowed_elements: vec![Element::Jupiter],
            unlocks_ability: None,
            description: "A bow crackling with storm energy.".into(),
        },

        // ===== ARMOR =====
        EquipmentDefinition {
            id: "leather-armor".into(), name: "Leather Armor".into(),
            slot: EquipmentSlot::Armor, tier: EquipmentTier::Basic, cost: 40,
            stat_bonus: StatBonus { def: 4, ..Default::default() },
            allowed_elements: vec![],
            unlocks_ability: None,
            description: "Basic leather armor.".into(),
        },
        EquipmentDefinition {
            id: "iron-armor".into(), name: "Iron Armor".into(),
            slot: EquipmentSlot::Armor, tier: EquipmentTier::Iron, cost: 250,
            stat_bonus: StatBonus { def: 12, spd: -1, ..Default::default() },
            allowed_elements: vec![Element::Venus, Element::Mars],
            unlocks_ability: None,
            description: "Heavy iron plate armor.".into(),
        },
        EquipmentDefinition {
            id: "mythril-robe".into(), name: "Mythril Robe".into(),
            slot: EquipmentSlot::Armor, tier: EquipmentTier::Mythril, cost: 2500,
            stat_bonus: StatBonus { def: 22, mag: 8, ..Default::default() },
            allowed_elements: vec![Element::Mercury, Element::Jupiter],
            unlocks_ability: None,
            description: "A lightweight robe woven with mythril threads.".into(),
        },

        // ===== ACCESSORIES =====
        EquipmentDefinition {
            id: "lucky-charm".into(), name: "Lucky Charm".into(),
            slot: EquipmentSlot::Accessory, tier: EquipmentTier::Basic, cost: 100,
            stat_bonus: StatBonus { ..Default::default() },
            allowed_elements: vec![],
            unlocks_ability: None,
            description: "A small charm that brings good fortune.".into(),
        },
        EquipmentDefinition {
            id: "speed-ring".into(), name: "Speed Ring".into(),
            slot: EquipmentSlot::Accessory, tier: EquipmentTier::Silver, cost: 800,
            stat_bonus: StatBonus { spd: 8, ..Default::default() },
            allowed_elements: vec![],
            unlocks_ability: None,
            description: "A ring that enhances the wearer's speed.".into(),
        },
        EquipmentDefinition {
            id: "power-amulet".into(), name: "Power Amulet".into(),
            slot: EquipmentSlot::Accessory, tier: EquipmentTier::Silver, cost: 900,
            stat_bonus: StatBonus { atk: 8, ..Default::default() },
            allowed_elements: vec![],
            unlocks_ability: None,
            description: "An amulet that amplifies physical strength.".into(),
        },
    ];

    for eq in equipment {
        m.insert(eq.id.clone(), eq);
    }

    m
}
