use bevy::prelude::*;

use crate::components::world::*;
use super::core::{start_transition, GameState, ScreenTransition};

// ── Constants ─────────────────────────────────────────────────────────
const TILE_SIZE: f32 = 32.0;
const MAP_WIDTH: i32 = 30;
const MAP_HEIGHT: i32 = 20;
const PLAYER_SPEED: f32 = 120.0;

// Colors for the tile map
const GRASS_COLOR: Color = Color::srgb(0.18, 0.42, 0.15);
const GRASS_COLOR_ALT: Color = Color::srgb(0.15, 0.38, 0.13);
const PATH_COLOR: Color = Color::srgb(0.55, 0.45, 0.3);
const WATER_COLOR: Color = Color::srgb(0.15, 0.25, 0.55);
const WALL_COLOR: Color = Color::srgb(0.35, 0.3, 0.25);
const BUILDING_COLOR: Color = Color::srgb(0.5, 0.35, 0.2);
const DOOR_COLOR: Color = Color::srgb(0.6, 0.45, 0.15);
const PLAYER_COLOR: Color = Color::srgb(0.85, 0.65, 0.13);
const NPC_COLOR: Color = Color::srgb(0.3, 0.5, 0.8);

// ── Marker components ─────────────────────────────────────────────────

#[derive(Component)]
pub struct OverworldRoot;

#[derive(Component)]
pub struct OverworldCamera;

#[derive(Component)]
pub struct MapTile;

#[derive(Component)]
pub struct DialogBox;

#[derive(Component)]
pub struct DialogText;

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Debug)]
pub struct DialogState {
    pub active: bool,
    pub lines: Vec<String>,
    pub current_line: usize,
    pub speaker: String,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            active: false,
            lines: Vec::new(),
            current_line: 0,
            speaker: String::new(),
        }
    }
}

/// Tile data for collision/interaction.
#[derive(Resource, Debug)]
pub struct TileMap {
    pub width: i32,
    pub height: i32,
    /// 0 = grass, 1 = path, 2 = wall/solid, 3 = water, 4 = building, 5 = door
    pub tiles: Vec<u8>,
}

impl TileMap {
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 2; // out of bounds = solid
        }
        self.tiles[(y * self.width + x) as usize]
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        let tile = self.get(x, y);
        tile != 2 && tile != 3 && tile != 4 // not wall, water, or building walls
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
        tiles[(0 * MAP_WIDTH + x) as usize] = 2;
        tiles[((MAP_HEIGHT - 1) * MAP_WIDTH + x) as usize] = 2;
    }
    for y in 0..MAP_HEIGHT {
        tiles[(y * MAP_WIDTH + 0) as usize] = 2;
        tiles[(y * MAP_WIDTH + MAP_WIDTH - 1) as usize] = 2;
    }

    // Main path (horizontal road through the village)
    for x in 1..MAP_WIDTH - 1 {
        tiles[(10 * MAP_WIDTH + x) as usize] = 1;
        tiles[(11 * MAP_WIDTH + x) as usize] = 1;
    }
    // Vertical path (crossroads)
    for y in 1..MAP_HEIGHT - 1 {
        tiles[(y * MAP_WIDTH + 15) as usize] = 1;
    }

    // Water pond (bottom-right)
    for y in 14..18 {
        for x in 22..27 {
            tiles[(y * MAP_WIDTH + x) as usize] = 3;
        }
    }

    // Building 1 — Item Shop (top-left area)
    for y in 3..7 {
        for x in 3..8 {
            tiles[(y * MAP_WIDTH + x) as usize] = 4;
        }
    }
    tiles[(6 * MAP_WIDTH + 5) as usize] = 5; // door

    // Building 2 — Inn (top-right area)
    for y in 3..7 {
        for x in 20..25 {
            tiles[(y * MAP_WIDTH + x) as usize] = 4;
        }
    }
    tiles[(6 * MAP_WIDTH + 22) as usize] = 5; // door

    // Building 3 — Elder's house (left of crossroads)
    for y in 13..17 {
        for x in 8..13 {
            tiles[(y * MAP_WIDTH + x) as usize] = 4;
        }
    }
    tiles[(13 * MAP_WIDTH + 10) as usize] = 5; // door

    // Some decorative walls / fences
    for x in 3..8 {
        tiles[(8 * MAP_WIDTH + x) as usize] = 2;
    }

    // Alternate grass pattern for visual interest
    // (handled in rendering by checkerboard)

    TileMap {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        tiles,
    }
}

// ── Setup ─────────────────────────────────────────────────────────────

fn setup_overworld(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut dialog: ResMut<DialogState>,
) {
    // Reset dialog
    *dialog = DialogState::default();

    let tilemap = generate_tile_map();

    // Camera
    commands.spawn((
        OverworldRoot,
        OverworldCamera,
        Camera2d,
    ));

    // Render tiles
    for y in 0..tilemap.height {
        for x in 0..tilemap.width {
            let tile = tilemap.get(x, y);
            let color = match tile {
                0 => {
                    if (x + y) % 2 == 0 {
                        GRASS_COLOR
                    } else {
                        GRASS_COLOR_ALT
                    }
                }
                1 => PATH_COLOR,
                2 => WALL_COLOR,
                3 => WATER_COLOR,
                4 => BUILDING_COLOR,
                5 => DOOR_COLOR,
                _ => GRASS_COLOR,
            };

            commands.spawn((
                OverworldRoot,
                MapTile,
                Sprite::from_color(color, Vec2::new(TILE_SIZE, TILE_SIZE)),
                Transform::from_xyz(
                    x as f32 * TILE_SIZE,
                    -(y as f32) * TILE_SIZE, // Y-down in world coords
                    0.0,
                ),
            ));
        }
    }

    // Try to load player sprite, fall back to colored rectangle
    let player_start = GridPosition::new(15, 10);
    commands.spawn((
        OverworldRoot,
        Player,
        PlayerMovement::default(),
        player_start,
        Sprite::from_color(PLAYER_COLOR, Vec2::new(TILE_SIZE * 0.8, TILE_SIZE * 0.8)),
        Transform::from_xyz(
            player_start.x as f32 * TILE_SIZE,
            -(player_start.y as f32) * TILE_SIZE,
            10.0, // above tiles
        ),
    ));

    // NPCs
    spawn_npc(
        &mut commands,
        "Elder Dora",
        GridPosition::new(10, 12),
        vec![
            "Welcome to Vale Village, young adept.".to_string(),
            "The tower to the north holds great danger...".to_string(),
            "But also great treasure. Prepare yourself well.".to_string(),
        ],
    );

    spawn_npc(
        &mut commands,
        "Shopkeeper",
        GridPosition::new(5, 8),
        vec![
            "Welcome! Take a look at my wares.".to_string(),
            "We have the finest potions in the land!".to_string(),
        ],
    );

    spawn_npc(
        &mut commands,
        "Innkeeper",
        GridPosition::new(22, 8),
        vec![
            "Rest here to recover your strength.".to_string(),
            "That'll be 20 gold. ...Just kidding, it's free for now!".to_string(),
        ],
    );

    spawn_npc(
        &mut commands,
        "Guard",
        GridPosition::new(15, 5),
        vec![
            "The path north leads to the Corrupted Tower.".to_string(),
            "Only the bravest adventurers dare enter.".to_string(),
            "Make sure you have Djinn equipped before you go!".to_string(),
        ],
    );

    // Spawn dialog box (hidden initially)
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
            BackgroundColor(Color::srgba(0.04, 0.04, 0.15, 0.95)),
            BorderColor(Color::srgb(0.85, 0.65, 0.13)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                DialogText,
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.7)),
            ));

            parent.spawn((
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
    dialog: Vec<String>,
) {
    commands.spawn((
        OverworldRoot,
        Npc {
            name: name.to_string(),
            dialog,
        },
        pos,
        Sprite::from_color(NPC_COLOR, Vec2::new(TILE_SIZE * 0.7, TILE_SIZE * 0.7)),
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

        let mut dx = 0i32;
        let mut dy = 0i32;

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
            let new_x = grid_pos.x + dx;
            let new_y = grid_pos.y + dy;

            if tilemap.is_walkable(new_x, new_y) {
                grid_pos.x = new_x;
                grid_pos.y = new_y;
                transform.translation.x = new_x as f32 * TILE_SIZE;
                transform.translation.y = -(new_y as f32) * TILE_SIZE;
                movement.move_cooldown.reset();
            }
        }
    }
}

fn camera_follow_player(
    player_query: Query<&Transform, (With<Player>, Without<OverworldCamera>)>,
    mut camera_query: Query<&mut Transform, (With<OverworldCamera>, Without<Player>)>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let Ok(mut cam_tf) = camera_query.single_mut() else {
        return;
    };

    // Smooth camera follow
    let target = Vec3::new(player_tf.translation.x, player_tf.translation.y, cam_tf.translation.z);
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

    let Ok((player_pos, movement)) = player_query.single() else {
        return;
    };

    // Check the tile the player is facing
    let (fx, fy) = match movement.facing {
        Facing::Up => (0, -1),
        Facing::Down => (0, 1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    };
    let face_pos = GridPosition::new(player_pos.x + fx, player_pos.y + fy);

    // Check if any NPC is at that position
    for (npc_pos, npc) in &npc_query {
        if *npc_pos == face_pos {
            dialog.active = true;
            dialog.speaker = npc.name.clone();
            dialog.lines = npc.dialog.clone();
            dialog.current_line = 0;

            // Show dialog box
            if let Ok(mut vis) = dialog_box.single_mut() {
                *vis = Visibility::Visible;
            }

            // Set first line
            if let Ok(mut text) = dialog_text.single_mut() {
                let line = &dialog.lines[0];
                **text = format!("{}: {}", dialog.speaker, line);
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
            // Close dialog
            dialog.active = false;
            if let Ok(mut vis) = dialog_box.single_mut() {
                *vis = Visibility::Hidden;
            }
        } else {
            // Show next line
            if let Ok(mut text) = dialog_text.single_mut() {
                let line = &dialog.lines[dialog.current_line];
                **text = format!("{}: {}", dialog.speaker, line);
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
        start_transition(&mut transition, GameState::PauseMenu);
    }
}

fn cleanup_overworld(
    mut commands: Commands,
    query: Query<Entity, With<OverworldRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
