//! Turn-based battle system.
//!
//! This module contains the complete battle system for Vale Village, including:
//! - Data structures (types, state, actions)
//! - Pure algorithms (damage, status, turn order, AI, rewards, djinn)
//! - Bevy ECS systems (command select, resolution, victory/defeat)
//! - Plugin registration
//!
//! ## Architecture
//!
//! Pure logic lives in dedicated modules and takes `&BattleUnit` / `&mut BattleUnit`
//! parameters. Bevy systems in `systems.rs` bridge between ECS queries and pure logic.
//! This separation allows the battle math to be unit-tested without a Bevy App.
//!
//! ## Stub Types
//!
//! `types.rs` contains temporary type definitions (Element, Stats, etc.) that mirror
//! what another agent is building in `src/components/` and `src/data/`. When those
//! modules land, replace the stubs with re-exports.

pub mod types;
pub mod damage;
pub mod status;
pub mod turn_order;
pub mod ai;
pub mod rewards;
pub mod djinn;
pub mod systems;
pub mod plugin;

pub use plugin::BattlePlugin;
