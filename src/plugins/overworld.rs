//! Overworld exploration: tile map, player movement, NPCs, dialog system.
//!
//! The player entity uses `Sprite::from_color` for now (placeholder) and
//! moves on a grid. Camera follows the player. NPCs are interactable when
//! the player faces them and presses Enter.

use bevy::prelude::*;

use crate::components::world::*;
use super::core_plugin::GameState;
use super::ui::{start_transition, ScreenTransition};

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

/// Simple tile map for collision checking.
#[derive(Resource, Debug)]
struct TileMap {
    width: i32,
    height: i32,
    tiles: Vec<u8>,
}

impl TileMap {
    fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 2; // wall
        }
        self.tiles[(y * self.width + x) as usize]
    }

    fn is_walkable(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        t != 2 && t != 3 && t != 4 // not wall, water, building-interior
    }
}

// ── Plugin ────────────────────────────────────────────────────────────

pub struct OverworldPlugin;

impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogState>()
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
            .add_systems(OnExit(GameState::Overworld), cleanup_overworld);
    }
}

// ── Map generation ────────────────────────────────────────────────────

fn generate_tile_map() -> TileMap {
    let mut tiles = vec![0u8; (MAP_WIDTH * MAP_HEIGHT) as usize];

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

    TileMap {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        tiles,
    }
}

// ── Setup ─────────────────────────────────────────────────────────────

fn setup_overworld(mut commands: Commands, mut dialog: ResMut<DialogState>) {
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
                    if (x + y) % 2 == 0 { GRASS } else { GRASS_ALT }
                }
                1 => PATH,
                2 => WALL,
                3 => WATER,
                4 => BUILDING,
                5 => DOOR,
                _ => GRASS,
            };

            commands.spawn((
                OverworldRoot,
                Sprite::from_color(color, Vec2::new(TILE_SIZE, TILE_SIZE)),
                Transform::from_xyz(
                    x as f32 * TILE_SIZE,
                    -(y as f32) * TILE_SIZE,
                    0.0,
                ),
            ));
        }
    }

    // Player
    let start = GridPosition::new(15, 10);
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
    spawn_npc(&mut commands, "Elder Dora", GridPosition::new(10, 12), NPC_ALT_COLOR, vec![
        "Welcome to Vale Village, young adept.".into(),
        "The tower to the north holds great danger...".into(),
        "But also great treasure. Prepare yourself well.".into(),
    ]);
    spawn_npc(&mut commands, "Shopkeeper", GridPosition::new(5, 8), NPC_COLOR, vec![
        "Welcome! Take a look at my wares.".into(),
        "We have the finest potions in all the land!".into(),
    ]);
    spawn_npc(&mut commands, "Innkeeper", GridPosition::new(22, 8), NPC_COLOR, vec![
        "Rest here to recover your strength.".into(),
        "That'll be 20 gold. ...Just kidding, it's free for now!".into(),
    ]);
    spawn_npc(&mut commands, "Guard", GridPosition::new(15, 5), NPC_ALT_COLOR, vec![
        "The path north leads to the Corrupted Tower.".into(),
        "Only the bravest adventurers dare enter.".into(),
        "Make sure you have Djinn equipped before you go!".into(),
    ]);

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
                TextFont { font_size: 20.0, ..default() },
                TextColor(DIALOG_TEXT_COLOR),
            ));
            parent.spawn((
                DialogHintText,
                Text::new("[Enter] to continue"),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.6, 0.55, 0.4, 0.7)),
                Node { align_self: AlignSelf::FlexEnd, ..default() },
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
        Transform::from_xyz(
            pos.x as f32 * TILE_SIZE,
            -(pos.y as f32) * TILE_SIZE,
            5.0,
        ),
    ));
}

// ── Systems ───────────────────────────────────────────────────────────

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tilemap: Res<TileMap>,
    dialog: Res<DialogState>,
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
            }
        }
    }
}

fn camera_follow_player(
    player_query: Query<&Transform, (With<Player>, Without<OverworldCamera>)>,
    mut cam_query: Query<&mut Transform, (With<OverworldCamera>, Without<Player>)>,
) {
    let Ok(player_tf) = player_query.get_single() else { return };
    let Ok(mut cam_tf) = cam_query.get_single_mut() else { return };

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

    let Ok((player_pos, movement)) = player_query.get_single() else { return };

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
                **text = format!(
                    "{}: {}",
                    dialog.speaker, dialog.lines[dialog.current_line]
                );
            }
        }
    }
}

fn overworld_pause(
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<DialogState>,
    mut transition: ResMut<ScreenTransition>,
) {
    if dialog.active {
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
