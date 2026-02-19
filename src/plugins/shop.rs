//! Shop plugin: buy items and equipment with gold.

use bevy::prelude::*;

use crate::plugins::core_plugin::{GameData, GameState, Party};

const SHOP_BG: Color = Color::srgba(0.02, 0.02, 0.08, 0.98);
const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const SHOP_TITLE: Color = Color::srgb(0.92, 0.83, 0.45);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DIM_TEXT: Color = Color::srgb(0.62, 0.62, 0.7);
const GOLD_TEXT: Color = Color::srgb(0.85, 0.65, 0.13);
const GREEN_TEXT: Color = Color::srgb(0.3, 0.8, 0.3);
const RED_TEXT: Color = Color::srgb(0.8, 0.3, 0.3);

#[derive(Resource, Debug, Clone, Default)]
pub struct CurrentShop {
    pub items: Vec<String>,
    pub equipment: Vec<String>,
}

#[derive(Component)]
struct ShopRoot;

#[derive(Component)]
struct ShopItemEntry {
    index: usize,
    item_id: String,
    cost: u32,
    #[allow(dead_code)]
    is_equipment: bool,
}

#[derive(Component)]
struct ShopGoldText;

#[derive(Component)]
struct ShopMessageText;

#[derive(Resource, Debug)]
struct ShopState {
    cursor: usize,
    total_items: usize,
    message: String,
    message_timer: Timer,
}

impl Default for ShopState {
    fn default() -> Self {
        Self {
            cursor: 0,
            total_items: 0,
            message: String::new(),
            message_timer: Timer::from_seconds(2.0, TimerMode::Once),
        }
    }
}

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentShop>()
            .add_systems(OnEnter(GameState::Shop), setup_shop_ui)
            .add_systems(
                Update,
                (shop_input, update_shop_message)
                    .chain()
                    .run_if(in_state(GameState::Shop)),
            )
            .add_systems(OnExit(GameState::Shop), cleanup_shop);
    }
}

fn setup_shop_ui(
    mut commands: Commands,
    current_shop: Res<CurrentShop>,
    game_data: Res<GameData>,
    party: Res<Party>,
) {
    // Build shop item list
    let mut shop_entries: Vec<(String, String, u32, bool, String)> = Vec::new(); // (id, name, cost, is_equip, desc)

    for item_id in &current_shop.items {
        if let Some(def) = game_data.items.get(item_id) {
            shop_entries.push((
                item_id.clone(),
                def.name.clone(),
                def.cost,
                false,
                def.description.clone(),
            ));
        }
    }
    for equip_id in &current_shop.equipment {
        if let Some(def) = game_data.equipment.get(equip_id) {
            shop_entries.push((
                equip_id.clone(),
                def.name.clone(),
                def.cost,
                true,
                format!("{} [{}]", def.description, format_stat_bonus(def)),
            ));
        }
    }

    let total = shop_entries.len();
    commands.insert_resource(ShopState {
        cursor: 0,
        total_items: total,
        ..Default::default()
    });

    commands
        .spawn((
            ShopRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(SHOP_BG),
            GlobalZIndex(30),
        ))
        .with_children(|root| {
            // Title row
            root.spawn((Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Shop"),
                        TextFont {
                            font_size: 42.0,
                            ..default()
                        },
                        TextColor(SHOP_TITLE),
                    ));
                    row.spawn((
                        ShopGoldText,
                        Text::new(format!("Gold: {}", party.gold)),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(GOLD_TEXT),
                    ));
                });

            // Message area
            root.spawn((
                ShopMessageText,
                Text::new(""),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(GREEN_TEXT),
                Node {
                    height: Val::Px(24.0),
                    ..default()
                },
            ));

            // Item list
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(4.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
            ))
            .with_children(|list| {
                if shop_entries.is_empty() {
                    list.spawn((
                        Text::new("Nothing for sale here."),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(DIM_TEXT),
                    ));
                } else {
                    for (i, (id, name, cost, is_equip, desc)) in shop_entries.iter().enumerate() {
                        let is_sel = i == 0;
                        let affordable = party.gold >= *cost;
                        let cost_color = if affordable { GOLD_TEXT } else { RED_TEXT };
                        let prefix = if is_sel { "> " } else { "  " };

                        list.spawn((
                            ShopItemEntry {
                                index: i,
                                item_id: id.clone(),
                                cost: *cost,
                                is_equipment: *is_equip,
                            },
                            Text::new(format!("{prefix}{name} - {cost}g - {desc}")),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(if is_sel { BRIGHT_GOLD } else { cost_color }),
                        ));
                    }
                }
            });

            // Hint
            root.spawn((
                Text::new("[Up/Down] Select  [Enter] Buy  [Escape] Leave"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
            ));
        });
}

fn format_stat_bonus(def: &crate::data::items::EquipmentDefinition) -> String {
    let b = &def.stat_bonus;
    let mut parts = Vec::new();
    if b.atk != 0 {
        parts.push(format!("ATK {:+}", b.atk));
    }
    if b.def != 0 {
        parts.push(format!("DEF {:+}", b.def));
    }
    if b.mag != 0 {
        parts.push(format!("MAG {:+}", b.mag));
    }
    if b.spd != 0 {
        parts.push(format!("SPD {:+}", b.spd));
    }
    if b.hp != 0 {
        parts.push(format!("HP {:+}", b.hp));
    }
    if b.pp != 0 {
        parts.push(format!("PP {:+}", b.pp));
    }
    if parts.is_empty() {
        "No bonus".into()
    } else {
        parts.join(", ")
    }
}

#[allow(clippy::too_many_arguments)]
fn shop_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut shop_state: ResMut<ShopState>,
    mut party: ResMut<Party>,
    mut next_state: ResMut<NextState<GameState>>,
    mut entries: Query<(&ShopItemEntry, &mut Text, &mut TextColor)>,
    mut gold_text: Query<&mut Text, (With<ShopGoldText>, Without<ShopItemEntry>)>,
) {
    let total = shop_state.total_items;

    if total > 0 {
        if keys.just_pressed(KeyCode::ArrowUp) {
            shop_state.cursor = if shop_state.cursor == 0 {
                total - 1
            } else {
                shop_state.cursor - 1
            };
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            shop_state.cursor = (shop_state.cursor + 1) % total;
        }

        // Update visual
        for (entry, mut text, mut color) in &mut entries {
            let is_sel = entry.index == shop_state.cursor;
            let affordable = party.gold >= entry.cost;
            let prefix = if is_sel { "> " } else { "  " };

            // Reconstruct the display text from the entry data
            let base = text
                .as_str()
                .trim_start_matches("> ")
                .trim_start_matches("  ");
            **text = format!("{prefix}{base}");

            if is_sel {
                *color = TextColor(BRIGHT_GOLD);
            } else if affordable {
                *color = TextColor(DIM_TEXT);
            } else {
                *color = TextColor(RED_TEXT);
            }
        }

        // Buy item
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
            let selected: Option<(String, u32)> = entries
                .iter()
                .find(|(e, _, _)| e.index == shop_state.cursor)
                .map(|(e, _, _)| (e.item_id.clone(), e.cost));

            if let Some((item_id, cost)) = selected {
                if party.gold >= cost {
                    party.gold -= cost;
                    party.inventory.push(item_id.clone());
                    shop_state.message = format!("Bought {}!", item_id);
                    shop_state.message_timer.reset();

                    // Update gold display
                    if let Ok(mut gt) = gold_text.get_single_mut() {
                        **gt = format!("Gold: {}", party.gold);
                    }
                } else {
                    shop_state.message = "Not enough gold!".into();
                    shop_state.message_timer.reset();
                }
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Overworld);
    }
}

fn update_shop_message(
    time: Res<Time>,
    mut shop_state: ResMut<ShopState>,
    mut text_query: Query<&mut Text, With<ShopMessageText>>,
) {
    shop_state.message_timer.tick(time.delta());

    if let Ok(mut text) = text_query.get_single_mut() {
        if shop_state.message_timer.finished() {
            **text = String::new();
        } else {
            **text = shop_state.message.clone();
        }
    }
}

fn cleanup_shop(
    mut commands: Commands,
    query: Query<Entity, With<ShopRoot>>,
    mut current_shop: ResMut<CurrentShop>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    current_shop.items.clear();
    current_shop.equipment.clear();
    commands.remove_resource::<ShopState>();
}
