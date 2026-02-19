use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Grid position on the overworld tile map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component, Reflect)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan distance to another grid position.
    #[allow(dead_code)]
    pub fn distance_to(&self, other: &GridPosition) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

/// Marker for the player entity on the overworld.
#[derive(Component, Debug, Default, Reflect)]
pub struct Player;

/// Movement direction the player is facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum Facing {
    #[default]
    Down,
    Up,
    Left,
    Right,
}

/// Player movement state on the overworld.
#[derive(Component, Debug, Reflect)]
pub struct PlayerMovement {
    pub facing: Facing,
    pub move_cooldown: Timer,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            facing: Facing::Down,
            move_cooldown: Timer::from_seconds(0.15, TimerMode::Once),
        }
    }
}

/// An NPC on the overworld that can be interacted with.
#[derive(Component, Debug, Reflect)]
pub struct Npc {
    pub name: String,
    pub dialog: Vec<String>,
}

#[derive(Component, Debug, Reflect, Clone)]
pub struct ShopKeeper {
    pub items: Vec<String>,
    pub equipment: Vec<String>,
}

/// Marker for solid tiles that block movement.
#[derive(Component, Debug, Default, Reflect)]
pub struct Solid;

/// Marker for tiles that trigger encounters.
#[derive(Component, Debug, Default, Reflect)]
pub struct EncounterZone;

/// A trigger area (e.g., building entrance, zone transition).
#[derive(Component, Debug, Reflect)]
#[allow(dead_code)]
pub struct Trigger {
    pub trigger_type: TriggerType,
}

/// What a trigger does when the player steps on it.
#[derive(Debug, Clone, Reflect)]
#[allow(dead_code)]
pub enum TriggerType {
    EnterBuilding(String),
    ZoneTransition(String),
    StartBattle,
    Shop,
}
