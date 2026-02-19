use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The four elements plus neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Reflect)]
pub enum Element {
    Venus,   // Earth
    Mars,    // Fire
    Mercury, // Water
    Jupiter, // Wind
    #[default]
    Neutral,
}

impl Element {
    /// Returns the elemental damage modifier when `self` attacks `defender`.
    pub fn modifier_against(&self, defender: &Element) -> f32 {
        match (self, defender) {
            // Advantage cycle: Venus > Jupiter > Mercury > Mars > Venus
            (Element::Venus, Element::Jupiter)
            | (Element::Jupiter, Element::Mercury)
            | (Element::Mercury, Element::Mars)
            | (Element::Mars, Element::Venus) => 1.25,
            // Reverse = disadvantage
            (Element::Jupiter, Element::Venus)
            | (Element::Mercury, Element::Jupiter)
            | (Element::Mars, Element::Mercury)
            | (Element::Venus, Element::Mars) => 0.75,
            _ => 1.0,
        }
    }
}

/// Core stats shared by player characters and enemies.
#[derive(Debug, Clone, Serialize, Deserialize, Component, Reflect)]
pub struct UnitStats {
    pub hp: i32,
    pub max_hp: i32,
    pub pp: i32,
    pub max_pp: i32,
    pub atk: i32,
    pub def: i32,
    pub spd: i32,
    pub luck: i32,
    pub level: u8,
    pub xp: u32,
    pub element: Element,
}

impl Default for UnitStats {
    fn default() -> Self {
        Self {
            hp: 100,
            max_hp: 100,
            pp: 30,
            max_pp: 30,
            atk: 15,
            def: 10,
            spd: 10,
            luck: 5,
            level: 1,
            xp: 0,
            element: Element::Neutral,
        }
    }
}

/// Status effects that can be applied to units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum StatusEffect {
    Poison,
    Burn,
    Freeze,
    Stun,
    Paralyze,
    Blind,
    AtkUp,
    DefUp,
    SpdUp,
    AtkDown,
    DefDown,
    SpdDown,
}

/// An active status effect on a unit, with remaining duration.
#[derive(Debug, Clone, Serialize, Deserialize, Component, Reflect)]
pub struct ActiveStatusEffect {
    pub effect: StatusEffect,
    pub remaining_turns: u8,
}
