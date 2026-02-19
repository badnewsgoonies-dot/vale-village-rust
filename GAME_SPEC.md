# Vale Village — Rust/Bevy Game Specification

## Overview
Golden Sun-inspired 2D pixel-art RPG. Turn-based combat, overworld exploration, djinn system, shops, tower dungeon. Target: Steam release.

## Tech Stack
- **Engine**: Bevy 0.15 (latest stable)
- **Rendering**: bevy_sprite, pixel-art scaling (nearest-neighbor)
- **Audio**: bevy_audio (or rodio)
- **UI**: bevy_ui (or bevy_egui for menus)
- **Serialization**: serde + ron for save files
- **Steam**: steamworks-rs crate

## Core Game Systems

### Elements (4 + Neutral)
- Venus (Earth), Mars (Fire), Mercury (Water), Jupiter (Wind), Neutral
- Advantage cycle: Venus > Jupiter > Mercury > Mars > Venus
- Element advantage = 1.25x damage, disadvantage = 0.75x

### Unit Stats
```rust
struct UnitStats {
    hp: i32, max_hp: i32,
    pp: i32, max_pp: i32,  // Psynergy Points (mana)
    atk: i32, def: i32,
    spd: i32, luck: i32,
    level: u8,  // 1-20
    xp: u32,
    element: Element,
}
```

### Abilities
- Types: physical, psynergy (elemental magic), healing, buff, debuff
- Fields: id, name, type, mana_cost, base_power, targets (single/all/party), element, unlock_level
- ~80+ abilities total (12 base + expansions per unit)
- AI hints: priority, target preference, avoid_overkill

### Units (Player Characters, 10 total)
- Adept (Venus/starter), War Mage (Mars), Mystic (Mercury), Ranger (Jupiter)
- Recruits: Blaze, Sentinel, Karis, Tyrell, Stormcaller, Felix
- Each has element, base stats, ability list that unlocks by level
- Party size: 4 active

### Enemies (50 types)
- Enslaved Beasts (12), Slavers (20), Legendary Enslaved (9), Bosses (9)
- ALL enemies have an element (no neutral)
- Each has: stats, abilities, loot table, AI behavior

### Djinn System (Golden Sun signature mechanic)
- Djinn are elemental creatures that attach to units
- States: Set (passive boost) → Standby (ready to summon) → Recovery (cooldown)
- Set: grants stat bonuses
- Unleash: powerful single-use ability, moves djinn to Standby
- Summon: combine standby djinn for massive attacks

### Battle System (Turn-Based)
1. **Command Phase**: Player selects actions for each party member (Fight/Djinn/Item/Defend/Flee)
2. **Resolution Phase**: Actions execute in speed order
3. **Damage Formula**: `(atk * ability_power / def) * element_modifier * random(0.9..1.1)`
4. **Status Effects**: Poison, Burn, Freeze, Stun, Paralyze, Blind + buffs/debuffs
5. **Tick Order**: DOT damage → healing → duration decrement → remove expired
6. **Victory**: XP/gold/item rewards, level-up checks

### Overworld
- Tile-based 2D map (sprite-based, not tilemap — use sprite sheets)
- Player character walks with arrow keys / WASD / gamepad
- NPCs with dialog interaction (Enter/A button)
- Buildings to enter (houses, shops, tower)
- Random encounters (or encounter zones)

### Tower Dungeon
- Multi-floor dungeon with increasing difficulty
- Floor rewards and boss battles
- Team selection before entering

### Shops
- Buy/sell equipment and items
- Equipment has: stat bonuses, ability grants, element affinity
- Items: healing potions, status cures, revives

### Save System
- Save to file (serde + ron format)
- Save slots (3)
- Auto-save on floor transitions

## Sprites (Placeholder)
- Located in `assets/sprites/`
- Subdirectories: backgrounds, battle, buildings, icons, overworld, placeholders, psynergy, scenery, sprite-sheets, text
- Mix of PNG and GIF — use PNG for Bevy, convert GIF to sprite sheets if needed
- Golden Sun art style — pixel art, 16x16 to 64x64 tiles

## Screens / Scenes
1. **Title Screen**: Logo, New Game, Continue, Settings, Quit
2. **Overworld**: Exploration, NPC interaction, shop access
3. **Battle**: Turn-based combat UI
4. **Shop**: Buy/sell interface
5. **Inventory**: Equipment management
6. **Settings**: Audio volume, display (fullscreen/windowed), controls
7. **Pause Menu**: Resume, Save, Load, Settings, Quit to Title

## Controls
- Keyboard: Arrow keys / WASD movement, Enter confirm, Escape cancel/menu, Tab inventory
- Gamepad: D-pad/stick movement, A confirm, B cancel, Start menu
- Mouse: Click UI buttons

## Module Structure (Bevy Plugins)
```
src/
  main.rs              — App setup, plugin registration
  plugins/
    core.rs            — Game state, resources, events
    battle.rs          — Battle system plugin
    overworld.rs       — Overworld movement, NPCs, encounters
    ui.rs              — All UI screens (menus, HUD, dialogs)
    audio.rs           — Music and SFX
    save.rs            — Save/load system
    steam.rs           — Steam SDK integration (stubs)
  data/
    mod.rs             — Data loading
    abilities.rs       — Ability definitions
    units.rs           — Unit/character definitions
    enemies.rs         — Enemy definitions
    items.rs           — Item/equipment definitions
    djinn.rs           — Djinn definitions
  components/
    mod.rs             — ECS components
    stats.rs           — UnitStats, Element, StatusEffect
    battle.rs          — BattleState, TurnOrder, Action
    world.rs           — Position, Sprite, NPC, Trigger
```
