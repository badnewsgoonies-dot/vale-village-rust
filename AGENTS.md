# Repository Guidelines

## Project Structure & Module Organization
This is a single-crate Rust game (`vale_village`) built with Bevy. Core code lives in `src/`:
- `src/main.rs`: app entrypoint, plugin wiring, window setup.
- `src/plugins/`: gameplay feature plugins (`overworld`, `battle_ui`, `inventory`, `shop`, `save`, `audio`, `ui`).
- `src/battle/`: turn-order, AI, status, damage, rewards, and battle systems.
- `src/components/`: ECS components and stat/world data types.
- `src/data/`: static game data (abilities, units, enemies, items, djinn).

Runtime assets are under `assets/` (sprites, backgrounds, icons, audio-related content). Design notes are in `GAME_SPEC.md`.

## Build, Test, and Development Commands
- `cargo check` — fast compile validation while iterating.
- `cargo run` — launch the game window locally.
- `cargo test` — run unit tests (currently battle-focused).
- `cargo fmt` / `cargo fmt -- --check` — format code / verify formatting.
- `cargo clippy --all-targets --all-features -D warnings` — strict lint pass before PR.

## Coding Style & Naming Conventions
Use default Rust style with `rustfmt` (4-space indentation, trailing commas where rustfmt inserts them).  
Naming patterns:
- modules/files: `snake_case` (`turn_order.rs`)
- functions/tests: `snake_case` (`test_player_before_enemy_on_tie`)
- types/traits/enums: `PascalCase`
- constants: `SCREAMING_SNAKE_CASE`

Keep systems and helpers cohesive by feature (battle logic in `src/battle/`, app wiring in plugins).

## Testing Guidelines
Use Rust’s built-in test framework (`#[cfg(test)]`, `#[test]`) and keep tests close to implementation files. Prefer deterministic tests by seeding RNG (existing tests use `StdRng::seed_from_u64`).  
Add/adjust tests whenever battle math, turn ordering, status effects, rewards, or djinn behavior changes.

## Commit & Pull Request Guidelines
Follow the repository’s Conventional Commit style seen in history:
- `feat: ...`
- `chore: ...`
- `refactor: ...`

PRs should include:
- clear summary of gameplay/technical impact,
- linked issue/task (if applicable),
- test/verification notes (`cargo test`, `cargo check`, lint/format status),
- screenshots or short clips for UI/visual changes.
