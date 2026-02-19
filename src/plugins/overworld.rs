//! Overworld exploration: tile map, player movement, NPCs, dialog system.
//!
//! The player entity uses `Sprite::from_color` for now (placeholder) and
//! moves on a grid. Camera follows the player. NPCs are interactable when
//! the player faces them and presses Enter.

use bevy::prelude::*;
use rand::Rng;

use super::core_plugin::GameState;
use super::ui::{ScreenTransition, start_transition};
use crate::battle::types::{
    BattlePhase, BattleUnit, EndBattleEvent, GrowthRates, StartBattleEvent, UnitSide,
};
use crate::components::world::*;
use crate::data::enemies::{self, EnemyDefinition};

// ── Constants ─────────────────────────────────────────────────────────
const TILE_SIZE: f32 = 32.0;
const MAP_WIDTH: i32 = 30;
const MAP_HEIGHT: i32 = 20;

// Tile colors
const GRASS: Color = Color::srgb(0.18, 0.42, 0.15);
const GRASS_ALT: Color = Color::srgb(0.15, 0.38, 0.13);
const PATH: Color = Color::srgb(0.55, 0.45, 0.3);
const WATER: Color = Color::srgb(0.15, 0.25, 0.55);
const WALL: Color = Color::srgb(0.35, 0.3, 0.25);
const BUILDING: Color = Color::srgb(0.5, 0.35, 0.2);
const DOOR: Color = Color::srgb(0.6, 0.45, 0.15);
const PLAYER_COLOR: Color = Color::srgb(0.85, 0.65, 0.13);
const NPC_COLOR: Color = Color::srgb(0.3, 0.5, 0.8);
const NPC_ALT_COLOR: Color = Color::srgb(0.7, 0.4, 0.25);

// UI colors
const GOLD: Color = Color::srgb(0.85, 0.65, 0.13);
const DIALOG_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.95);
const DIALOG_TEXT_COLOR: Color = Color::srgb(0.9, 0.85, 0.7);
const ENCOUNTER_CHANCE: f64 = 0.15;
const MIN_ENCOUNTER_ENEMIES: usize = 1;
const MAX_ENCOUNTER_ENEMIES: usize = 3;
const ENEMY_BATTLE_ID_BASE: u32 = 10_000;

// ── Marker components ─────────────────────────────────────────────────

#[derive(Component)]
struct OverworldRoot;

#[derive(Component)]
struct OverworldCamera;

#[derive(Component)]
struct DialogBox;

#[derive(Component)]
struct DialogText;

#[derive(Component)]
struct DialogHintText;

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Debug, Default)]
struct DialogState {
    active: bool,
    lines: Vec<String>,
    current_line: usize,
    speaker: String,
}

#[derive(Resource, Debug, Default)]
struct BattleReturnPosition {
    player_position: Option<GridPosition>,
}

/// Simple tile map for collision checking.
#[derive(Resource, Debug)]
struct TileMap {
    width: i32,
    height: i32,
    tiles: Vec<u8>,
    encounter_zones: Vec<bool>,
}

impl TileMap {
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    fn get(&self, x: i32, y: i32) -> u8 {
        let Some(idx) = self.idx(x, y) else {
            return 2; // wall
        };
        self.tiles[idx]
    }

    fn is_walkable(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        t != 2 && t != 3 && t != 4 // not wall, water, building-interior
    }

    fn is_encounter_zone(&self, x: i32, y: i32) -> bool {
        let Some(idx) = self.idx(x, y) else {
            return false;
        };
        self.encounter_zones[idx]
    }
}

// ── Plugin ────────────────────────────────────────────────────────────

pub struct OverworldPlugin;

impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogState>()
            .init_resource::<BattleReturnPosition>()
            .add_systems(OnEnter(GameState::Overworld), setup_overworld)
            .add_systems(
                Update,
                (
                    player_movement,
                    camera_follow_player,
                    player_interact,
                    dialog_input,
                    overworld_pause,
                )
                    .run_if(in_state(GameState::Overworld)),
            )
            .add_systems(
                Update,
                handle_battle_end.run_if(in_state(GameState::Battle)),
            )
            .add_systems(OnExit(GameState::Overworld), cleanup_overworld);
    }
}

// ── Map generation ────────────────────────────────────────────────────

fn generate_tile_map() -> TileMap {
    let mut tiles = vec![0u8; (MAP_WIDTH * MAP_HEIGHT) as usize];
    let mut encounter_zones = vec![false; (MAP_WIDTH * MAP_HEIGHT) as usize];

    // Border walls
    for x in 0..MAP_WIDTH {
        tiles[x as usize] = 2;
        tiles[((MAP_HEIGHT - 1) * MAP_WIDTH + x) as usize] = 2;
    }
    for y in 0..MAP_HEIGHT {
        tiles[(y * MAP_WIDTH) as usize] = 2;
        tiles[(y * MAP_WIDTH + MAP_WIDTH - 1) as usize] = 2;
    }

    // Main horizontal road
    for x in 1..MAP_WIDTH - 1 {
        tiles[(10 * MAP_WIDTH + x) as usize] = 1;
        tiles[(11 * MAP_WIDTH + x) as usize] = 1;
    }

    // Vertical crossroads
    for y in 1..MAP_HEIGHT - 1 {
        tiles[(y * MAP_WIDTH + 15) as usize] = 1;
    }

    // Pond (bottom-right)
    for y in 14..18 {
        for x in 22..27 {
            tiles[(y * MAP_WIDTH + x) as usize] = 3;
        }
    }

    // Building: Item Shop (top-left)
    for y in 3..7 {
        for x in 3..8 {
            tiles[(y * MAP_WIDTH + x) as usize] = 4;
        }
    }
    tiles[(6 * MAP_WIDTH + 5) as usize] = 5; // door

    // Building: Inn (top-right)
    for y in 3..7 {
        for x in 20..25 {
            tiles[(y * MAP_WIDTH + x) as usize] = 4;
        }
    }
    tiles[(6 * MAP_WIDTH + 22) as usize] = 5; // door

    // Building: Elder's house
    for y in 13..17 {
        for x in 8..13 {
            tiles[(y * MAP_WIDTH + x) as usize] = 4;
        }
    }
    tiles[(13 * MAP_WIDTH + 10) as usize] = 5; // door

    // Fence / walls
    for x in 3..8 {
        tiles[(8 * MAP_WIDTH + x) as usize] = 2;
    }

    // Encounter zones: tall grass and cave-like ground patches.
    let mut mark_encounter = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= MAP_WIDTH || y >= MAP_HEIGHT {
            return;
        }
        let idx = (y * MAP_WIDTH + x) as usize;
        if tiles[idx] == 0 || tiles[idx] == 1 {
            encounter_zones[idx] = true;
        }
    };

    // Tall grass (north of town)
    for y in 2..6 {
        for x in 9..14 {
            mark_encounter(x, y);
        }
    }

    // Cave-like patch (south-east plains)
    for y in 14..18 {
        for x in 16..21 {
            mark_encounter(x, y);
        }
    }

    TileMap {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        tiles,
        encounter_zones,
    }
}

// ── Setup ─────────────────────────────────────────────────────────────

fn setup_overworld(
    mut commands: Commands,
    mut dialog: ResMut<DialogState>,
    mut return_position: ResMut<BattleReturnPosition>,
) {
    *dialog = DialogState::default();

    let tilemap = generate_tile_map();

    // Overworld camera (separate from startup camera — we tag it to clean up)
    commands.spawn((
        OverworldRoot,
        OverworldCamera,
        Camera2d,
        Camera {
            order: 1, // render above the default camera
            ..default()
        },
    ));

    // Render tiles
    for y in 0..tilemap.height {
        for x in 0..tilemap.width {
            let tile = tilemap.get(x, y);
            let color = match tile {
                0 => {
                    if (x + y) % 2 == 0 {
                        GRASS
                    } else {
                        GRASS_ALT
                    }
                }
                1 => PATH,
                2 => WALL,
                3 => WATER,
                4 => BUILDING,
                5 => DOOR,
                _ => GRASS,
            };

            let mut tile_entity = commands.spawn((
                OverworldRoot,
                Sprite::from_color(color, Vec2::new(TILE_SIZE, TILE_SIZE)),
                Transform::from_xyz(x as f32 * TILE_SIZE, -(y as f32) * TILE_SIZE, 0.0),
            ));
            if tilemap.is_encounter_zone(x, y) {
                tile_entity.insert(EncounterZone);
            }
        }
    }

    // Player
    let start = return_position
        .player_position
        .take()
        .unwrap_or(GridPosition::new(15, 10));
    commands.spawn((
        OverworldRoot,
        Player,
        PlayerMovement::default(),
        start,
        Sprite::from_color(PLAYER_COLOR, Vec2::new(TILE_SIZE * 0.8, TILE_SIZE * 0.8)),
        Transform::from_xyz(
            start.x as f32 * TILE_SIZE,
            -(start.y as f32) * TILE_SIZE,
            10.0,
        ),
    ));

    // NPCs
    spawn_npc(
        &mut commands,
        "Elder Dora",
        GridPosition::new(10, 12),
        NPC_ALT_COLOR,
        vec![
            "Welcome to Vale Village, young adept.".into(),
            "The tower to the north holds great danger...".into(),
            "But also great treasure. Prepare yourself well.".into(),
        ],
    );
    spawn_npc(
        &mut commands,
        "Shopkeeper",
        GridPosition::new(5, 8),
        NPC_COLOR,
        vec![
            "Welcome! Take a look at my wares.".into(),
            "We have the finest potions in all the land!".into(),
        ],
    );
    spawn_npc(
        &mut commands,
        "Innkeeper",
        GridPosition::new(22, 8),
        NPC_COLOR,
        vec![
            "Rest here to recover your strength.".into(),
            "That'll be 20 gold. ...Just kidding, it's free for now!".into(),
        ],
    );
    spawn_npc(
        &mut commands,
        "Guard",
        GridPosition::new(15, 5),
        NPC_ALT_COLOR,
        vec![
            "The path north leads to the Corrupted Tower.".into(),
            "Only the bravest adventurers dare enter.".into(),
            "Make sure you have Djinn equipped before you go!".into(),
        ],
    );

    // Dialog UI (hidden initially)
    commands
        .spawn((
            OverworldRoot,
            DialogBox,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Px(40.0),
                right: Val::Px(40.0),
                height: Val::Px(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(DIALOG_BG),
            BorderColor(GOLD),
            Visibility::Hidden,
            GlobalZIndex(20),
        ))
        .with_children(|parent| {
            parent.spawn((
                DialogText,
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(DIALOG_TEXT_COLOR),
            ));
            parent.spawn((
                DialogHintText,
                Text::new("[Enter] to continue"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.55, 0.4, 0.7)),
                Node {
                    align_self: AlignSelf::FlexEnd,
                    ..default()
                },
            ));
        });

    commands.insert_resource(tilemap);
}

fn spawn_npc(
    commands: &mut Commands,
    name: &str,
    pos: GridPosition,
    color: Color,
    dialog: Vec<String>,
) {
    commands.spawn((
        OverworldRoot,
        Npc {
            name: name.to_string(),
            dialog,
        },
        pos,
        Sprite::from_color(color, Vec2::new(TILE_SIZE * 0.7, TILE_SIZE * 0.7)),
        Transform::from_xyz(pos.x as f32 * TILE_SIZE, -(pos.y as f32) * TILE_SIZE, 5.0),
    ));
}

// ── Systems ───────────────────────────────────────────────────────────

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tilemap: Res<TileMap>,
    dialog: Res<DialogState>,
    mut return_position: ResMut<BattleReturnPosition>,
    mut start_battle_events: EventWriter<StartBattleEvent>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_battle_phase: ResMut<NextState<BattlePhase>>,
    npc_query: Query<&GridPosition, (With<Npc>, Without<Player>)>,
    mut query: Query<(&mut GridPosition, &mut Transform, &mut PlayerMovement), With<Player>>,
) {
    if dialog.active {
        return;
    }

    for (mut grid_pos, mut transform, mut movement) in &mut query {
        movement.move_cooldown.tick(time.delta());
        if !movement.move_cooldown.finished() {
            continue;
        }

        let (mut dx, mut dy) = (0i32, 0i32);

        if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
            dy = -1;
            movement.facing = Facing::Up;
        } else if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
            dy = 1;
            movement.facing = Facing::Down;
        } else if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
            dx = -1;
            movement.facing = Facing::Left;
        } else if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
            dx = 1;
            movement.facing = Facing::Right;
        }

        if dx != 0 || dy != 0 {
            let nx = grid_pos.x + dx;
            let ny = grid_pos.y + dy;

            // Check tile walkability AND NPC collision
            let npc_blocking = npc_query
                .iter()
                .any(|npc_pos| npc_pos.x == nx && npc_pos.y == ny);

            if tilemap.is_walkable(nx, ny) && !npc_blocking {
                grid_pos.x = nx;
                grid_pos.y = ny;
                transform.translation.x = nx as f32 * TILE_SIZE;
                transform.translation.y = -(ny as f32) * TILE_SIZE;
                movement.move_cooldown.reset();

                if tilemap.is_encounter_zone(nx, ny) {
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(ENCOUNTER_CHANCE) {
                        let enemy_units = build_random_encounter(&mut rng);
                        if !enemy_units.is_empty() {
                            return_position.player_position = Some(*grid_pos);
                            start_battle_events.send(StartBattleEvent {
                                encounter_id: format!("overworld-{nx}-{ny}"),
                                enemy_units,
                            });
                            next_battle_phase.set(BattlePhase::CommandSelect);
                            next_game_state.set(GameState::Battle);
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn build_random_encounter(rng: &mut impl Rng) -> Vec<BattleUnit> {
    let mut all_enemies: Vec<EnemyDefinition> =
        enemies::build_enemy_registry().into_values().collect();

    if all_enemies.is_empty() {
        return Vec::new();
    }

    let encounter_count = rng
        .gen_range(MIN_ENCOUNTER_ENEMIES..=MAX_ENCOUNTER_ENEMIES)
        .min(all_enemies.len());

    let mut enemies_for_battle = Vec::with_capacity(encounter_count);
    for i in 0..encounter_count {
        let pick = rng.gen_range(0..all_enemies.len());
        let def = all_enemies.swap_remove(pick);
        enemies_for_battle.push(enemy_definition_to_battle_unit(
            &def,
            ENEMY_BATTLE_ID_BASE + i as u32,
        ));
    }

    enemies_for_battle
}

fn enemy_definition_to_battle_unit(definition: &EnemyDefinition, id: u32) -> BattleUnit {
    BattleUnit {
        id,
        name: definition.name.clone(),
        side: UnitSide::Enemy,
        element: definition.element,
        level: definition.level,
        hp: definition.hp,
        max_hp: definition.hp,
        pp: definition.pp,
        max_pp: definition.pp,
        atk: definition.atk,
        def: definition.def,
        mag: definition.mag,
        spd: definition.spd,
        luck: 5 + i32::from(definition.level / 2),
        status_effects: Vec::new(),
        ability_ids: definition
            .abilities
            .iter()
            .map(|ability| ability.ability_id.clone())
            .collect(),
        djinn_ids: Vec::new(),
        damage_taken: 0,
        damage_dealt: 0,
        xp: 0,
        growth_rates: GrowthRates::default(),
    }
}

fn camera_follow_player(
    player_query: Query<&Transform, (With<Player>, Without<OverworldCamera>)>,
    mut cam_query: Query<&mut Transform, (With<OverworldCamera>, Without<Player>)>,
) {
    let Ok(player_tf) = player_query.get_single() else {
        return;
    };
    let Ok(mut cam_tf) = cam_query.get_single_mut() else {
        return;
    };

    let target = Vec3::new(
        player_tf.translation.x,
        player_tf.translation.y,
        cam_tf.translation.z,
    );
    cam_tf.translation = cam_tf.translation.lerp(target, 0.1);
}

fn player_interact(
    keys: Res<ButtonInput<KeyCode>>,
    mut dialog: ResMut<DialogState>,
    player_query: Query<(&GridPosition, &PlayerMovement), With<Player>>,
    npc_query: Query<(&GridPosition, &Npc)>,
    mut dialog_box: Query<&mut Visibility, With<DialogBox>>,
    mut dialog_text: Query<&mut Text, With<DialogText>>,
) {
    if dialog.active {
        return;
    }
    if !keys.just_pressed(KeyCode::Enter) && !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok((player_pos, movement)) = player_query.get_single() else {
        return;
    };

    let (fx, fy) = match movement.facing {
        Facing::Up => (0, -1),
        Facing::Down => (0, 1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    };
    let face = GridPosition::new(player_pos.x + fx, player_pos.y + fy);

    for (npc_pos, npc) in &npc_query {
        if *npc_pos == face {
            dialog.active = true;
            dialog.speaker = npc.name.clone();
            dialog.lines = npc.dialog.clone();
            dialog.current_line = 0;

            if let Ok(mut vis) = dialog_box.get_single_mut() {
                *vis = Visibility::Visible;
            }
            if let Ok(mut text) = dialog_text.get_single_mut() {
                **text = format!("{}: {}", dialog.speaker, dialog.lines[0]);
            }
            break;
        }
    }
}

fn dialog_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut dialog: ResMut<DialogState>,
    mut dialog_box: Query<&mut Visibility, With<DialogBox>>,
    mut dialog_text: Query<&mut Text, With<DialogText>>,
) {
    if !dialog.active {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        dialog.current_line += 1;

        if dialog.current_line >= dialog.lines.len() {
            dialog.active = false;
            if let Ok(mut vis) = dialog_box.get_single_mut() {
                *vis = Visibility::Hidden;
            }
        } else {
            if let Ok(mut text) = dialog_text.get_single_mut() {
                **text = format!("{}: {}", dialog.speaker, dialog.lines[dialog.current_line]);
            }
        }
    }
}

fn overworld_pause(
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<DialogState>,
    mut transition: ResMut<ScreenTransition>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if dialog.active {
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        next_state.set(GameState::Inventory);
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        start_transition(&mut transition, GameState::Paused);
    }
}

fn cleanup_overworld(mut commands: Commands, query: Query<Entity, With<OverworldRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn handle_battle_end(
    mut end_events: EventReader<EndBattleEvent>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_battle_phase: ResMut<NextState<BattlePhase>>,
) {
    for event in end_events.read() {
        if event.victory {
            next_battle_phase.set(BattlePhase::Inactive);
            next_game_state.set(GameState::Overworld);
            break;
        }
    }
}
