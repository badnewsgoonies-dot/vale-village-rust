# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Vale Village is a Golden Sun-inspired 2D pixel-art RPG built with **Bevy 0.15** and **Rust 2024 edition**. Turn-based combat, overworld exploration, djinn system, shops, and a tower dungeon. Full game design spec is in `GAME_SPEC.md`.

## Build & Development Commands

```bash
cargo check                                          # Fast compile validation
cargo run                                            # Launch the game (960x540 window)
cargo test                                           # Run all tests
cargo test turn_order                                # Run tests in a specific module
cargo fmt -- --check                                 # Verify formatting
cargo clippy --all-targets --all-features -D warnings # Lint (required before PR)
```

Dev profile uses `opt-level = 1` with dependencies at `opt-level = 3` for fast iteration.

## Architecture

### Plugin System

The app is composed of Bevy plugins registered in `src/main.rs`:

1. **CoreGamePlugin** (`plugins/core_plugin.rs`) — `GameState` state machine, `GameData` resource (all registries), `Party` resource
2. **SavePlugin** (`plugins/save.rs`) — RON-format save/load
3. **GameAudioPlugin** (`plugins/audio.rs`) — Music and SFX
4. **UiPlugin** (`plugins/ui.rs`) — Title screen, menus, dialogs, settings
5. **OverworldPlugin** (`plugins/overworld.rs`) — Tile-based movement, NPCs, encounters, buildings
6. **ShopPlugin** / **InventoryPlugin** (`plugins/shop.rs`, `plugins/inventory.rs`) — Buy/sell, equipment management
7. **BattleUiPlugin** (`plugins/battle_ui.rs`) — Battle visual layer (HP bars, menus)
8. **BattlePlugin** (`src/battle/plugin.rs`) — Core battle logic and ECS systems

### State Machine

Two Bevy `States` drive game flow:
- **`GameState`**: `Loading → MainMenu → Overworld → Battle → Shop → Inventory → Settings → Paused`
- **`BattlePhase`**: `Inactive → CommandSelect → AiSelect → Resolution → Victory | Defeat`

### Battle System — Pure Logic + ECS Bridge

The battle module (`src/battle/`) separates pure functional logic from ECS systems:

- **Pure logic** (testable without Bevy): `damage.rs`, `status.rs`, `turn_order.rs`, `ai.rs`, `djinn.rs`, `rewards.rs` — take `&BattleUnit` params, return computed values
- **ECS systems** (`systems.rs`): query entities, call pure functions, write results back
- **Types** (`types.rs`): `BattleUnit`, `BattleAction`, `BattlePhase`, `DjinnBattleState`, resources like `BattleStateRes`, `CommandSelectState`, `DjinnBattleRes`

This pattern is intentional — maintain it when adding battle features.

### Data Layer

`src/data/` contains static game definitions loaded via `build_*_registry()` functions into `GameData`:
- `abilities.rs` (~80+ abilities), `units.rs` (10 playable characters), `enemies.rs` (50 enemy types)
- `items.rs` (equipment + consumables), `djinn.rs` (djinn definitions)

### Components

`src/components/` holds ECS components:
- `stats.rs`: `UnitStats`, `Element`, `StatusEffect`
- `battle.rs`: Marker components (`InBattle`, `EnemyCombatant`, `PartyCombatant`)
- `world.rs`: `GridPosition`, `Player`, `Npc`, `Solid`, `EncounterZone`

### Key Design Decisions

- **Pixel-art rendering**: `ImagePlugin::default_nearest()` for nearest-neighbor sampling
- **Element system**: Venus > Jupiter > Mercury > Mars > Venus (1.25x advantage, 0.75x disadvantage)
- **Deterministic RNG**: Battle tests use `StdRng::seed_from_u64(42)` for reproducibility
- **`#[allow(dead_code)]`**: Used on intentional stubs reserved for future work — don't remove these

## Coding Conventions

- `rustfmt` defaults (4-space indent). Types: `PascalCase`. Functions/modules: `snake_case`. Constants: `SCREAMING_SNAKE_CASE`.
- Tests are co-located in implementation files using `#[cfg(test)]`.
- Conventional Commits: `feat:`, `chore:`, `refactor:`.
- Keep systems and helpers cohesive by feature — battle logic stays in `src/battle/`, app wiring in `plugins/`.
