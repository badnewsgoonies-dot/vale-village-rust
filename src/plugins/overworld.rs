//! Overworld exploration: tile map, player movement, NPCs, dialog system.
//!
//! The player entity uses `Sprite::from_color` for now (placeholder) and
//! moves on a grid. Camera follows the player. NPCs are interactable when
//! the player faces them and presses Enter.

use bevy::prelude::*;
use rand::Rng;

use super::core_plugin::{GameData, GameState, Party, story};
use super::shop::CurrentShop;
use super::sprites::SpriteHandles;
use super::tower::{
    TowerBattleActive, TowerState, build_floor_definitions, generate_floor_encounter,
};
use super::ui::{ScreenTransition, start_transition};
use crate::battle::types::{
    BattlePhase, BattleUnit, EndBattleEvent, GrowthRates, StartBattleEvent, UnitSide,
};
use crate::components::world::*;
use crate::data::enemies::{self, EnemyDefinition, get_enemies_by_tier};

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
const TOWER: Color = Color::srgb(0.4, 0.3, 0.5);
const TALL_GRASS: Color = Color::srgb(0.12, 0.35, 0.10);
const CAVE_GROUND: Color = Color::srgb(0.35, 0.30, 0.25);
const RECRUIT_NPC_COLOR: Color = Color::srgb(0.2, 0.7, 0.3);

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

#[derive(Component)]
struct TowerDoor;

/// Marks an NPC as a recruitable party member.
#[derive(Component, Debug)]
struct RecruitNpc {
    unit_id: String,
}

/// Marks the Elder NPC whose dialog changes based on story progress.
#[derive(Component, Debug)]
struct ElderNpc;

/// Marks the Fortune Teller NPC whose hints change based on story progress.
#[derive(Component, Debug)]
struct FortuneTellerNpc;

/// Marks an NPC as an innkeeper who can heal the party for gold.
#[derive(Component, Debug)]
struct InnKeeper {
    cost: u32,
}

// ── Resources ─────────────────────────────────────────────────────────

#[derive(Resource, Debug, Default)]
struct DialogState {
    active: bool,
    lines: Vec<String>,
    current_line: usize,
    speaker: String,
    speaker_entity: Option<Entity>,
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
        // Walkable: grass (0), path (1), door (5), tall grass (7), cave ground (8)
        // Not walkable: wall (2), water (3), building-interior (4), tower (6)
        t != 2 && t != 3 && t != 4 && t != 6
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

    // Building: Tower of Trials (right side of map, 3 wide x 4 tall)
    for y in 2..6 {
        for x in 25..28 {
            tiles[(y * MAP_WIDTH + x) as usize] = 6; // tower wall
        }
    }
    tiles[(5 * MAP_WIDTH + 26) as usize] = 5; // tower door

    // Fence / walls
    for x in 3..8 {
        tiles[(8 * MAP_WIDTH + x) as usize] = 2;
    }

    // Encounter zones: tall grass, cave-like ground, and forest edge patches.
    // Each zone gets a distinct tile type so players can visually identify danger areas.
    let mut mark_encounter_with_tile = |x: i32, y: i32, tile_type: u8| {
        if x < 0 || y < 0 || x >= MAP_WIDTH || y >= MAP_HEIGHT {
            return;
        }
        let idx = (y * MAP_WIDTH + x) as usize;
        if tiles[idx] == 0 || tiles[idx] == 1 {
            encounter_zones[idx] = true;
            tiles[idx] = tile_type;
        }
    };

    // Tall grass (north of town) — tile type 7
    for y in 2..6 {
        for x in 9..14 {
            mark_encounter_with_tile(x, y, 7);
        }
    }

    // Cave-like patch (south-east plains) — tile type 8
    for y in 14..18 {
        for x in 16..21 {
            mark_encounter_with_tile(x, y, 8);
        }
    }

    // Forest edge (west side of map) — tile type 7
    for y in 12..17 {
        for x in 2..6 {
            mark_encounter_with_tile(x, y, 7);
        }
    }

    TileMap {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        tiles,
        encounter_zones,
    }
}

// ── Sprite helpers ────────────────────────────────────────────────────

/// Maps an NPC name to a sprite handle key in `SpriteHandles::units`.
/// Returns `None` for NPCs that have no matching sprite.
fn npc_sprite_key(npc_name: &str) -> Option<&'static str> {
    match npc_name {
        "Karis" => Some("karis"),
        "Tyrell" => Some("tyrell"),
        "Amiti" => Some("mystic"),
        "Elder Dora" => Some("sentinel"),
        "Shopkeeper" => Some("ranger"),
        "Innkeeper" => Some("ranger"),
        "Guard" => Some("sentinel"),
        "Tower Guard" => Some("sentinel"),
        "Scholar Liam" => Some("mystic"),
        "Little Mia" => Some("karis"),
        "Blacksmith" => Some("war-mage"),
        "Fortune Teller" => Some("stormcaller"),
        "Wandering Merchant" => Some("ranger"),
        _ => None,
    }
}

/// Build a `Sprite` from a loaded image handle with a custom size, or fall
/// back to a coloured rectangle when the handle is not available.
fn make_sprite(handle: Option<&Handle<Image>>, fallback_color: Color, size: Vec2) -> Sprite {
    if let Some(h) = handle {
        Sprite {
            image: h.clone(),
            custom_size: Some(size),
            ..default()
        }
    } else {
        Sprite::from_color(fallback_color, size)
    }
}

// ── Setup ─────────────────────────────────────────────────────────────

fn setup_overworld(
    mut commands: Commands,
    mut dialog: ResMut<DialogState>,
    mut return_position: ResMut<BattleReturnPosition>,
    sprite_handles: Res<SpriteHandles>,
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
                6 => TOWER,
                7 => TALL_GRASS,
                8 => CAVE_GROUND,
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
            // Mark the tower door tile
            if x == 26 && y == 5 && tile == 5 {
                tile_entity.insert(TowerDoor);
            }
        }
    }

    // Player
    let start = return_position
        .player_position
        .take()
        .unwrap_or(GridPosition::new(15, 10));
    let player_size = Vec2::new(TILE_SIZE * 0.8, TILE_SIZE * 0.8);
    let player_sprite = make_sprite(sprite_handles.units.get("adept"), PLAYER_COLOR, player_size);
    commands.spawn((
        OverworldRoot,
        Player,
        PlayerMovement::default(),
        start,
        player_sprite,
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
        None,
        npc_sprite_key("Elder Dora").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc(
        &mut commands,
        "Shopkeeper",
        GridPosition::new(5, 8),
        NPC_COLOR,
        vec![
            "Welcome! I just received a fresh shipment from the Angara trade route.".into(),
            "Got potions, antidotes, and some Mercury Mist Elixirs — perfect for cleansing poison.".into(),
            "If you're planning to brave the Tower of Trials, stock up on healing items. You'll burn through them fast.".into(),
            "I also carry starter weapons for any new companions you recruit. Every adept needs a good blade.".into(),
            "A word of advice: the creatures on the upper floors resist certain elements. Bring a balanced party.".into(),
        ],
        Some(ShopKeeper {
            items: vec![
                "potion".into(),
                "antidote".into(),
                "jupiter-hermes-water".into(),
                "mercury-mist-elixir".into(),
            ],
            equipment: vec![
                "wooden-sword".into(),
                "wooden-axe".into(),
                "wooden-staff".into(),
                "short-bow".into(),
            ],
        }),
        npc_sprite_key("Shopkeeper").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc_full(
        &mut commands,
        "Innkeeper",
        GridPosition::new(22, 5),
        NPC_COLOR,
        vec![
            "Welcome to the Golden Sun Inn! You look weary. Come, sit by the fire.".into(),
            "A traveler passed through last week, pale as a ghost. He muttered about a shadow on the tower's top floor.".into(),
            "Some folk say there are monsters in the southern caves that haven't been seen in a hundred years.".into(),
            "I've also heard tell of a great beast that guards the tower's deepest secret. Even the Elder won't speak of it.".into(),
            "Rest here and I'll have your whole party feeling good as new. Your party has been fully restored!".into(),
        ],
        None,
        None,
        Some(InnKeeper { cost: 25 }),
        npc_sprite_key("Innkeeper").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc(
        &mut commands,
        "Guard",
        GridPosition::new(15, 5),
        NPC_ALT_COLOR,
        vec![
            "Halt, traveler. A word of warning before you go further.".into(),
            "The tall grass north and west of the village is crawling with wild creatures. Step in there unprepared and you won't last long.".into(),
            "The Tower of Trials lies to the east — you can see its peak from here. Many brave souls have entered. Fewer have returned.".into(),
            "Make sure you have Djinn equipped and your party at full strength before you stray from the roads.".into(),
            "If you stick to the stone paths, you'll be safe. The creatures don't venture onto them.".into(),
        ],
        None,
        npc_sprite_key("Guard").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc(
        &mut commands,
        "Tower Guard",
        GridPosition::new(26, 6),
        NPC_ALT_COLOR,
        vec![
            "This is the Tower of Trials, erected by the Alchemy adepts who founded Vale Village centuries ago.".into(),
            "It was built as a proving ground — ten floors of increasing danger, each guarded by creatures that feed on Psynergy.".into(),
            "I've watched seasoned warriors flee from the fifth floor with terror in their eyes. Something unnatural lurks there.".into(),
            "But the rewards are real. Rare Djinn nest within, and ancient artifacts lie waiting for those strong enough to claim them.".into(),
            "My advice? Bring a full party, stock up on potions and antidotes, and don't be ashamed to retreat if things go wrong.".into(),
        ],
        None,
        npc_sprite_key("Tower Guard").and_then(|k| sprite_handles.units.get(k)),
    );

    // Flavor NPCs
    spawn_npc(
        &mut commands,
        "Scholar Liam",
        GridPosition::new(10, 18),
        NPC_ALT_COLOR,
        vec![
            "Ah, a fellow seeker of knowledge! I've devoted my life to studying the elemental arts and the history of Alchemy.".into(),
            "There are four primal elements: Venus, the earth; Jupiter, the wind; Mercury, the water; and Mars, the flame.".into(),
            "Each element holds dominion over another. Venus overcomes Jupiter, Jupiter overcomes Mercury, Mercury overcomes Mars, and Mars overcomes Venus.".into(),
            "Djinn are ancient elemental spirits that bond with adepts. When set in battle, they can unleash devastating summons that shake the very ground.".into(),
            "Vale Village was founded by Alchemy adepts who sealed a great power within the tower. But the seal weakens with each passing year. That is why the creatures grow bolder.".into(),
        ],
        None,
        npc_sprite_key("Scholar Liam").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc(
        &mut commands,
        "Little Mia",
        GridPosition::new(21, 13),
        NPC_COLOR,
        vec![
            "Hey, hey! Are you an adept? A real one? When I grow up, I want to be just like you!".into(),
            "I snuck out to the caves south of town yesterday... please don't tell my mom!".into(),
            "I saw strange glowing creatures down there. One looked like a tiny spirit made of golden light — it was so pretty!".into(),
            "Scholar Liam says Djinn hide in places with strong Psynergy. Maybe that's what I saw? I should tell him!".into(),
            "Oh! I almost forgot — I found a weird symbol carved into the cave wall. It looked exactly like the rune on the tower door. Spooky, right?".into(),
        ],
        None,
        npc_sprite_key("Little Mia").and_then(|k| sprite_handles.units.get(k)),
    );

    // Blacksmith (near the shop)
    spawn_npc(
        &mut commands,
        "Blacksmith",
        GridPosition::new(18, 12),
        NPC_ALT_COLOR,
        vec![
            "The name's Garet. I've worked this forge for twenty years, ever since my old man taught me the trade.".into(),
            "A good blade needs strong metal and stronger will. Same goes for the adept who wields it.".into(),
            "The shop sells decent starter gear, but if you want real firepower, you need something forged with Psynergy ore.".into(),
            "If you find rare materials in the tower — elemental shards, ancient ingots — bring them to me. I can craft weapons the ancients would envy.".into(),
            "And keep your gear maintained! A chipped sword in the heat of battle is a death sentence.".into(),
        ],
        None,
        npc_sprite_key("Blacksmith").and_then(|k| sprite_handles.units.get(k)),
    );

    // Fortune Teller (dynamic dialog based on story flags)
    {
        let ft_entity = spawn_npc_full(
            &mut commands,
            "Fortune Teller",
            GridPosition::new(6, 14),
            NPC_ALT_COLOR,
            vec!["The stars whisper of your destiny, child of Alchemy...".into()],
            None,
            None,
            None,
            npc_sprite_key("Fortune Teller").and_then(|k| sprite_handles.units.get(k)),
        );
        commands.entity(ft_entity).insert(FortuneTellerNpc);
    }

    // Wandering Merchant
    spawn_npc(
        &mut commands,
        "Wandering Merchant",
        GridPosition::new(22, 8),
        NPC_COLOR,
        vec![
            "Greetings, friend! The name's Obaba. I've traveled from lands far beyond the eastern mountains.".into(),
            "In the markets of Lemuria, I once saw Djinn sold in crystal cages. Barbaric, yes, but the power they granted was undeniable.".into(),
            "There are rare treasures hidden on the deepest floors of towers like yours — artifacts from the age of Alchemy.".into(),
            "I once met a merchant who sold a single potion for a thousand gold. He claimed it could bring the dead back to life. I believed him.".into(),
            "If you ever venture beyond Vale Village, seek the port city of Champa. Their wares make our little shop look like a market stall.".into(),
        ],
        None,
        npc_sprite_key("Wandering Merchant").and_then(|k| sprite_handles.units.get(k)),
    );

    // Recruitment NPCs
    spawn_npc_full(
        &mut commands,
        "Karis",
        GridPosition::new(12, 8),
        RECRUIT_NPC_COLOR,
        vec![
            "I am Karis, a Wind Seer.".into(),
            "The winds call me to join your quest!".into(),
            "Karis joined the party!".into(),
        ],
        None,
        Some(RecruitNpc {
            unit_id: "wind_seer".into(),
        }),
        None,
        npc_sprite_key("Karis").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc_full(
        &mut commands,
        "Tyrell",
        GridPosition::new(18, 14),
        RECRUIT_NPC_COLOR,
        vec![
            "Name's Tyrell. I fight with fire!".into(),
            "Let me come along. You'll need the firepower!".into(),
            "Tyrell joined the party!".into(),
        ],
        None,
        Some(RecruitNpc {
            unit_id: "flame_user".into(),
        }),
        None,
        npc_sprite_key("Tyrell").and_then(|k| sprite_handles.units.get(k)),
    );
    spawn_npc_full(
        &mut commands,
        "Amiti",
        GridPosition::new(8, 16),
        RECRUIT_NPC_COLOR,
        vec![
            "I am Amiti, an Aqua Monk.".into(),
            "I shall lend my healing waters to your cause.".into(),
            "Amiti joined the party!".into(),
        ],
        None,
        Some(RecruitNpc {
            unit_id: "aqua_monk".into(),
        }),
        None,
        npc_sprite_key("Amiti").and_then(|k| sprite_handles.units.get(k)),
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
    shopkeeper: Option<ShopKeeper>,
    sprite_handle: Option<&Handle<Image>>,
) {
    spawn_npc_full(
        commands,
        name,
        pos,
        color,
        dialog,
        shopkeeper,
        None,
        None,
        sprite_handle,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_npc_full(
    commands: &mut Commands,
    name: &str,
    pos: GridPosition,
    color: Color,
    dialog: Vec<String>,
    shopkeeper: Option<ShopKeeper>,
    recruit: Option<RecruitNpc>,
    innkeeper: Option<InnKeeper>,
    sprite_handle: Option<&Handle<Image>>,
) -> Entity {
    let npc_size = Vec2::new(TILE_SIZE * 0.7, TILE_SIZE * 0.7);
    let sprite = make_sprite(sprite_handle, color, npc_size);
    let mut npc = commands.spawn((
        OverworldRoot,
        Npc {
            name: name.to_string(),
            dialog,
        },
        pos,
        sprite,
        Transform::from_xyz(pos.x as f32 * TILE_SIZE, -(pos.y as f32) * TILE_SIZE, 5.0),
    ));

    if let Some(shopkeeper) = shopkeeper {
        npc.insert(shopkeeper);
    }
    if let Some(recruit) = recruit {
        npc.insert(recruit);
    }
    if let Some(innkeeper) = innkeeper {
        npc.insert(innkeeper);
    }
    npc.id()
}

// ── Systems ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tilemap: Res<TileMap>,
    dialog: Res<DialogState>,
    tower_state: Res<TowerState>,
    mut tower_battle_active: ResMut<TowerBattleActive>,
    mut party: ResMut<Party>,
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

                // Tower door interaction: stepping onto the tower door
                // triggers a tower floor encounter.
                if nx == 26 && ny == 5 && tilemap.get(nx, ny) == 5 && tower_state.is_active {
                    let floors = build_floor_definitions();
                    let floor_idx = (tower_state.current_floor as usize).saturating_sub(1);
                    if let Some(floor_def) = floors.get(floor_idx) {
                        let mut rng = rand::thread_rng();
                        let encounter_pairs = generate_floor_encounter(floor_def, &mut rng);
                        let registry = enemies::build_enemy_registry();
                        let enemy_units: Vec<BattleUnit> = encounter_pairs
                            .iter()
                            .enumerate()
                            .filter_map(|(i, (enemy_id, level_bonus))| {
                                registry.get(enemy_id).map(|def| {
                                    let mut unit = enemy_definition_to_battle_unit(
                                        def,
                                        ENEMY_BATTLE_ID_BASE + i as u32,
                                    );
                                    unit.level = (unit.level as i32 + level_bonus).max(1) as u8;
                                    unit
                                })
                            })
                            .collect();
                        if !enemy_units.is_empty() {
                            return_position.player_position = Some(*grid_pos);
                            tower_battle_active.0 = true;
                            party.set_flag(story::TOWER_ENTERED, true);
                            start_battle_events.send(StartBattleEvent {
                                encounter_id: format!("tower-floor-{}", tower_state.current_floor),
                                enemy_units,
                            });
                            next_battle_phase.set(BattlePhase::CommandSelect);
                            next_game_state.set(GameState::Battle);
                            break;
                        }
                    }
                }

                if tilemap.is_encounter_zone(nx, ny) {
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(ENCOUNTER_CHANCE) {
                        let avg_level = get_average_party_level(&party);
                        let enemy_units = build_random_encounter(&mut rng, avg_level);
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

/// Compute the average level of party members from `Party::unit_levels`.
/// Returns 1 if the map is empty (e.g. at game start).
fn get_average_party_level(party: &Party) -> u8 {
    if party.unit_levels.is_empty() {
        return 1;
    }
    let total: u32 = party
        .unit_levels
        .values()
        .map(|(level, _xp)| *level as u32)
        .sum();
    let count = party.unit_levels.len() as u32;
    (total / count).max(1) as u8
}

fn build_random_encounter(rng: &mut impl Rng, party_level: u8) -> Vec<BattleUnit> {
    // Gather the tier-appropriate enemy pool based on the party's average level.
    let mut all_enemies: Vec<EnemyDefinition> = match party_level {
        1..=4 => get_enemies_by_tier(1),
        5..=8 => {
            // 60% tier 1, 40% tier 2
            let mut pool = get_enemies_by_tier(1);
            pool.extend(get_enemies_by_tier(2));
            let picked: Vec<EnemyDefinition> = pool
                .into_iter()
                .filter(|e| {
                    if e.tier == 1 {
                        rng.gen_bool(0.6)
                    } else {
                        rng.gen_bool(0.4)
                    }
                })
                .collect();
            if picked.is_empty() {
                // Fallback: at least return tier-1 enemies so encounters aren't empty.
                get_enemies_by_tier(1)
            } else {
                picked
            }
        }
        9..=12 => {
            // 60% tier 2, 40% tier 3
            let mut pool = get_enemies_by_tier(2);
            pool.extend(get_enemies_by_tier(3));
            let picked: Vec<EnemyDefinition> = pool
                .into_iter()
                .filter(|e| {
                    if e.tier == 2 {
                        rng.gen_bool(0.6)
                    } else {
                        rng.gen_bool(0.4)
                    }
                })
                .collect();
            if picked.is_empty() {
                get_enemies_by_tier(2)
            } else {
                picked
            }
        }
        _ => {
            // 13+: 40% tier 2, 60% tier 3
            let mut pool = get_enemies_by_tier(2);
            pool.extend(get_enemies_by_tier(3));
            let picked: Vec<EnemyDefinition> = pool
                .into_iter()
                .filter(|e| {
                    if e.tier == 2 {
                        rng.gen_bool(0.4)
                    } else {
                        rng.gen_bool(0.6)
                    }
                })
                .collect();
            if picked.is_empty() {
                get_enemies_by_tier(3)
            } else {
                picked
            }
        }
    };

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

#[allow(clippy::type_complexity)]
fn player_interact(
    keys: Res<ButtonInput<KeyCode>>,
    mut dialog: ResMut<DialogState>,
    party: Res<Party>,
    player_query: Query<(&GridPosition, &PlayerMovement), With<Player>>,
    npc_query: Query<(
        Entity,
        &GridPosition,
        &Npc,
        Option<&RecruitNpc>,
        Option<&InnKeeper>,
        Option<&ElderNpc>,
        Option<&FortuneTellerNpc>,
    )>,
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

    for (npc_entity, npc_pos, npc, recruit, innkeeper, elder, fortune_teller) in &npc_query {
        if *npc_pos == face {
            dialog.active = true;
            dialog.speaker = npc.name.clone();
            dialog.speaker_entity = Some(npc_entity);
            dialog.current_line = 0;

            // Elder NPC: dynamically choose dialog based on story flags.
            if elder.is_some() {
                dialog.lines = elder_dialog_lines(&party);
            // Fortune Teller NPC: dynamically choose hints based on story flags.
            } else if fortune_teller.is_some() {
                dialog.lines = fortune_teller_dialog_lines(&party);
            // If this is an innkeeper NPC, check if the player can afford to rest.
            } else if let Some(inn) = innkeeper {
                if party.gold < inn.cost {
                    dialog.lines = vec![
                        format!("You need {} gold to rest here.", inn.cost),
                        "Come back when you have enough coin.".into(),
                    ];
                } else {
                    dialog.lines = npc.dialog.clone();
                }
            // If this is a recruit NPC whose unit is already in the party,
            // show the "already recruited" dialog instead.
            } else if let Some(recruit) = recruit {
                let already_in_party = party.active.contains(&recruit.unit_id)
                    || party.bench.contains(&recruit.unit_id);
                if already_in_party {
                    dialog.lines = vec!["Welcome back! Keep fighting the good fight.".into()];
                } else {
                    dialog.lines = npc.dialog.clone();
                }
            } else {
                dialog.lines = npc.dialog.clone();
            }

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

#[allow(clippy::too_many_arguments)]
fn dialog_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut dialog: ResMut<DialogState>,
    game_data: Res<GameData>,
    shopkeepers: Query<&ShopKeeper>,
    recruit_npcs: Query<&RecruitNpc>,
    innkeepers: Query<&InnKeeper>,
    elder_npcs: Query<&ElderNpc>,
    mut party: ResMut<Party>,
    mut current_shop: ResMut<CurrentShop>,
    mut next_game_state: ResMut<NextState<GameState>>,
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
            if let Some(npc_entity) = dialog.speaker_entity.take() {
                // Check for Elder NPC interaction — set story flag
                if elder_npcs.get(npc_entity).is_ok() {
                    party.set_flag(story::TALKED_TO_ELDER, true);
                }

                // Check for shopkeeper interaction
                if let Ok(shopkeeper) = shopkeepers.get(npc_entity) {
                    current_shop.items = shopkeeper.items.clone();
                    current_shop.equipment = shopkeeper.equipment.clone();
                    next_game_state.set(GameState::Shop);
                }

                // Check for recruitment interaction
                if let Ok(recruit) = recruit_npcs.get(npc_entity) {
                    let already_in_party = party.active.contains(&recruit.unit_id)
                        || party.bench.contains(&recruit.unit_id);
                    if !already_in_party {
                        if party.active.len() < 4 {
                            party.active.push(recruit.unit_id.clone());
                        } else {
                            party.bench.push(recruit.unit_id.clone());
                        }
                        // Set story flag for this recruitment
                        let flag = match recruit.unit_id.as_str() {
                            "wind_seer" => Some(story::RECRUITED_KARIS),
                            "flame_user" => Some(story::RECRUITED_TYRELL),
                            "aqua_monk" => Some(story::RECRUITED_AMITI),
                            _ => None,
                        };
                        if let Some(f) = flag {
                            party.set_flag(f, true);
                        }
                    }
                }

                // Check for innkeeper interaction — heal the full party
                if let Ok(inn) = innkeepers.get(npc_entity)
                    && party.gold >= inn.cost
                {
                    party.gold -= inn.cost;
                    let all_units: Vec<String> = party
                        .active
                        .iter()
                        .chain(party.bench.iter())
                        .cloned()
                        .collect();
                    for unit_id in &all_units {
                        if let Some(def) = game_data.units.get(unit_id) {
                            let level = party
                                .unit_levels
                                .get(unit_id)
                                .map(|(lvl, _xp)| *lvl)
                                .unwrap_or(1);
                            let max_hp = def.base_hp + def.growth.hp * (level as i32 - 1);
                            let max_pp = def.base_pp + def.growth.pp * (level as i32 - 1);
                            party.unit_hp_pp.insert(unit_id.clone(), (max_hp, max_pp));
                        }
                    }
                }
            }
        } else if let Ok(mut text) = dialog_text.get_single_mut() {
            **text = format!("{}: {}", dialog.speaker, dialog.lines[dialog.current_line]);
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
        // Defeat: transition handled by BattlePhase::Defeat screen in battle_ui.
        // We still need to consume the event so it doesn't fire repeatedly.
    }
}

// ── Elder NPC dialog logic (pure function) ───────────────────────────

/// Returns the dialog lines the Elder should say based on the party's story flags.
///
/// Progression tiers (checked from most advanced to least):
/// 1. Tower completed  → congratulations
/// 2. Tower entered    → encouragement to push deeper
/// 3. All 3 recruited  → direct the party to the Tower
/// 4. Default          → suggest recruiting allies
fn elder_dialog_lines(party: &Party) -> Vec<String> {
    if party.has_flag(story::TOWER_COMPLETED) {
        vec!["You have conquered the Tower! You are a true hero of Vale Village.".into()]
    } else if party.has_flag(story::TOWER_ENTERED) {
        vec!["Push deeper into the tower. Floor 5 is a turning point.".into()]
    } else if party.has_flag(story::RECRUITED_KARIS)
        && party.has_flag(story::RECRUITED_TYRELL)
        && party.has_flag(story::RECRUITED_AMITI)
    {
        vec!["Your party is strong! The Tower of Trials awaits to the east.".into()]
    } else {
        vec![
            "Welcome, young adept. Seek allies for your journey. Talk to the warriors nearby."
                .into(),
        ]
    }
}

// ── Fortune Teller NPC dialog logic (pure function) ──────────────────

/// Returns the dialog lines the Fortune Teller should say based on story flags.
///
/// Progression tiers (checked from most advanced to least):
/// 1. Tower completed  → prophecy fulfilled, hint at future adventures
/// 2. Tower floor 5    → warn about the final guardian
/// 3. Tower entered    → cryptic encouragement to press on
/// 4. First battle won → hint about the tower's nature
/// 5. Talked to Elder  → nudge toward recruitment
/// 6. Default          → mysterious introduction and general guidance
fn fortune_teller_dialog_lines(party: &Party) -> Vec<String> {
    if party.has_flag(story::TOWER_COMPLETED) {
        vec![
            "The cards fall silent... the great trial is behind you.".into(),
            "But do not grow complacent. The stars reveal new shadows gathering beyond the mountains.".into(),
            "Your destiny is far from complete, child of Alchemy. Greater trials await in distant lands.".into(),
        ]
    } else if party.has_flag(story::TOWER_FLOOR_5) {
        vec![
            "I see darkness... a guardian of immense power bars your path on the final floors.".into(),
            "It feeds on the fear of those who challenge it. Steel your heart, or it will consume you.".into(),
            "The key to victory lies in the balance of elements. No single force can overcome it alone.".into(),
        ]
    } else if party.has_flag(story::TOWER_ENTERED) {
        vec![
            "The tower has tasted your resolve... and found it wanting? No — not yet.".into(),
            "I see floors yet unclimbed, enemies yet unvanquished. Press onward, but do not rush."
                .into(),
            "The spirits within grow restless. They sense your Psynergy. Use it wisely.".into(),
        ]
    } else if party.has_flag(story::FIRST_BATTLE_WON) {
        vec![
            "Ah, I sense the mark of battle upon you. You have tasted combat and survived.".into(),
            "The tower to the east calls to those with such strength. Its stones hum with ancient Psynergy.".into(),
            "But beware — the tower tests more than muscle. It tests the bonds between allies.".into(),
        ]
    } else if party.has_flag(story::TALKED_TO_ELDER) {
        vec![
            "The Elder has spoken to you... good. Her wisdom is not to be taken lightly.".into(),
            "I see companions in your future — a wind seer, a flame wielder, and a water sage."
                .into(),
            "Seek them out before you face the tower. Alone, even the strongest adept will falter."
                .into(),
        ]
    } else {
        vec![
            "Welcome, traveler... I have been expecting you. The stars told me of your coming.".into(),
            "I am a reader of fates. The Psynergy that flows through this land speaks to me in whispers.".into(),
            "Your path is shrouded in mist, but I see a great trial ahead. Speak to the Elder — she will set you on your way.".into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elder_dialog_default_no_flags() {
        let party = Party::default();
        let lines = elder_dialog_lines(&party);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Seek allies"));
    }

    #[test]
    fn elder_dialog_after_partial_recruit() {
        let mut party = Party::default();
        party.set_flag(story::RECRUITED_KARIS, true);
        party.set_flag(story::RECRUITED_TYRELL, true);
        // Amiti not yet recruited — should still show default
        let lines = elder_dialog_lines(&party);
        assert!(lines[0].contains("Seek allies"));
    }

    #[test]
    fn elder_dialog_all_recruited() {
        let mut party = Party::default();
        party.set_flag(story::RECRUITED_KARIS, true);
        party.set_flag(story::RECRUITED_TYRELL, true);
        party.set_flag(story::RECRUITED_AMITI, true);
        let lines = elder_dialog_lines(&party);
        assert!(lines[0].contains("Tower of Trials"));
    }

    #[test]
    fn elder_dialog_tower_entered() {
        let mut party = Party::default();
        party.set_flag(story::RECRUITED_KARIS, true);
        party.set_flag(story::RECRUITED_TYRELL, true);
        party.set_flag(story::RECRUITED_AMITI, true);
        party.set_flag(story::TOWER_ENTERED, true);
        let lines = elder_dialog_lines(&party);
        assert!(lines[0].contains("Floor 5"));
    }

    #[test]
    fn elder_dialog_tower_completed() {
        let mut party = Party::default();
        party.set_flag(story::TOWER_COMPLETED, true);
        let lines = elder_dialog_lines(&party);
        assert!(lines[0].contains("conquered the Tower"));
    }

    #[test]
    fn elder_dialog_tower_completed_overrides_entered() {
        let mut party = Party::default();
        party.set_flag(story::TOWER_ENTERED, true);
        party.set_flag(story::TOWER_COMPLETED, true);
        let lines = elder_dialog_lines(&party);
        // Completed takes priority over entered
        assert!(lines[0].contains("conquered the Tower"));
    }

    // ── Fortune Teller dialog tests ──────────────────────────────────

    #[test]
    fn fortune_teller_default_no_flags() {
        let party = Party::default();
        let lines = fortune_teller_dialog_lines(&party);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("expecting you"));
        assert!(lines[2].contains("Elder"));
    }

    #[test]
    fn fortune_teller_after_talked_to_elder() {
        let mut party = Party::default();
        party.set_flag(story::TALKED_TO_ELDER, true);
        let lines = fortune_teller_dialog_lines(&party);
        assert_eq!(lines.len(), 3);
        assert!(lines[1].contains("companions"));
    }

    #[test]
    fn fortune_teller_after_first_battle() {
        let mut party = Party::default();
        party.set_flag(story::FIRST_BATTLE_WON, true);
        let lines = fortune_teller_dialog_lines(&party);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("mark of battle"));
    }

    #[test]
    fn fortune_teller_tower_entered() {
        let mut party = Party::default();
        party.set_flag(story::TOWER_ENTERED, true);
        let lines = fortune_teller_dialog_lines(&party);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("tower has tasted"));
    }

    #[test]
    fn fortune_teller_tower_floor_5() {
        let mut party = Party::default();
        party.set_flag(story::TOWER_FLOOR_5, true);
        let lines = fortune_teller_dialog_lines(&party);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("guardian"));
        assert!(lines[2].contains("balance of elements"));
    }

    #[test]
    fn fortune_teller_tower_completed() {
        let mut party = Party::default();
        party.set_flag(story::TOWER_COMPLETED, true);
        let lines = fortune_teller_dialog_lines(&party);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("great trial is behind you"));
        assert!(lines[2].contains("Greater trials"));
    }

    #[test]
    fn fortune_teller_completed_overrides_floor_5() {
        let mut party = Party::default();
        party.set_flag(story::TOWER_FLOOR_5, true);
        party.set_flag(story::TOWER_COMPLETED, true);
        let lines = fortune_teller_dialog_lines(&party);
        // Completed takes priority over floor 5
        assert!(lines[0].contains("great trial is behind you"));
    }

    #[test]
    fn fortune_teller_tower_entered_overrides_first_battle() {
        let mut party = Party::default();
        party.set_flag(story::FIRST_BATTLE_WON, true);
        party.set_flag(story::TOWER_ENTERED, true);
        let lines = fortune_teller_dialog_lines(&party);
        // Tower entered takes priority over first battle
        assert!(lines[0].contains("tower has tasted"));
    }
}
