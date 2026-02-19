use bevy::prelude::*;
use std::collections::HashMap;

use crate::data::items::{EquipmentDefinition, EquipmentSlot};
use crate::plugins::core_plugin::{GameData, GameState, Party};

const INVENTORY_BG: Color = Color::srgb(0.05, 0.05, 0.12);
const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const GOLD: Color = Color::srgb(0.85, 0.65, 0.13);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);
const GREEN_TEXT: Color = Color::srgb(0.3, 0.8, 0.3);

#[derive(Component)]
struct InventoryRoot;

#[derive(Component)]
struct InventoryItemText {
    index: usize,
    line: String,
    #[allow(dead_code)]
    item_id: String,
    is_equipment: bool,
}

#[derive(Component)]
struct InventoryMessageText;

#[derive(Component)]
struct EquipPanelRoot;

#[derive(Component)]
struct EquipMemberEntry {
    index: usize,
    #[allow(dead_code)]
    unit_id: String,
}

#[derive(Component)]
#[allow(dead_code)]
struct EquipStatPreview;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventoryPhase {
    ItemList,
    EquipMemberSelect,
}

#[derive(Resource, Debug, Clone)]
struct InventoryState {
    cursor: usize,
    count: usize,
    phase: InventoryPhase,
    equip_cursor: usize,
    equip_count: usize,
    selected_equip_id: Option<String>,
    message: String,
    message_timer: Timer,
    /// Deduplicated item list for stable indexing
    item_ids: Vec<String>,
}

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            cursor: 0,
            count: 0,
            phase: InventoryPhase::ItemList,
            equip_cursor: 0,
            equip_count: 0,
            selected_equip_id: None,
            message: String::new(),
            message_timer: Timer::from_seconds(2.0, TimerMode::Once),
            item_ids: Vec::new(),
        }
    }
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Inventory), setup_inventory_ui)
            .add_systems(
                Update,
                (
                    inventory_input,
                    update_inventory_cursor_visual,
                    update_inventory_message,
                )
                    .chain()
                    .run_if(in_state(GameState::Inventory)),
            )
            .add_systems(OnExit(GameState::Inventory), cleanup_inventory);
    }
}

fn setup_inventory_ui(mut commands: Commands, party: Res<Party>, data: Res<GameData>) {
    let mut inventory_counts: HashMap<String, u32> = HashMap::new();
    for item_id in &party.inventory {
        *inventory_counts.entry(item_id.clone()).or_insert(0) += 1;
    }

    let mut item_ids = party.inventory.clone();
    item_ids.sort();
    item_ids.dedup();

    let mut item_lines: Vec<(String, String, String, bool)> = item_ids
        .iter()
        .filter_map(|item_id| {
            let amount = inventory_counts.get(item_id).copied().unwrap_or(0);
            if amount == 0 {
                return None;
            }

            let (name, description, is_equip) = if let Some(def) = data.items.get(item_id) {
                (def.name.clone(), def.description.clone(), false)
            } else if let Some(def) = data.equipment.get(item_id) {
                let bonus = format_stat_bonus(def);
                (
                    def.name.clone(),
                    format!("{} [{}]", def.description, bonus),
                    true,
                )
            } else {
                (
                    format!("Unknown Item ({item_id})"),
                    "No description available.".to_string(),
                    false,
                )
            };

            Some((
                name.to_lowercase(),
                format!("{name} x{amount} - {description}"),
                item_id.clone(),
                is_equip,
            ))
        })
        .collect();

    item_lines.sort_by(|a, b| a.0.cmp(&b.0));

    let count = item_lines.len();
    let ordered_ids: Vec<String> = item_lines.iter().map(|(_, _, id, _)| id.clone()).collect();

    commands.insert_resource(InventoryState {
        cursor: 0,
        count,
        item_ids: ordered_ids,
        ..Default::default()
    });

    commands
        .spawn((
            InventoryRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(INVENTORY_BG),
            GlobalZIndex(30),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Inventory"),
                TextFont {
                    font_size: 54.0,
                    ..default()
                },
                TextColor(BRIGHT_GOLD),
            ));

            root.spawn((
                Text::new(format!("Gold: {}", party.gold)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(GOLD),
            ));

            // Message area
            root.spawn((
                InventoryMessageText,
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

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
            ))
            .with_children(|list| {
                if item_lines.is_empty() {
                    list.spawn((
                        Text::new("No items"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(DIM_TEXT),
                    ));
                    return;
                }

                for (index, (_, line, item_id, is_equip)) in item_lines.iter().enumerate() {
                    let is_selected = index == 0;
                    let prefix = if is_selected { "> " } else { "  " };
                    let equip_marker = if *is_equip { " [Equip]" } else { "" };

                    list.spawn((
                        InventoryItemText {
                            index,
                            line: line.clone(),
                            item_id: item_id.clone(),
                            is_equipment: *is_equip,
                        },
                        Text::new(format!("{prefix}{line}{equip_marker}")),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(if is_selected { BRIGHT_GOLD } else { DIM_TEXT }),
                    ));
                }
            });

            root.spawn((
                Text::new("[Up/Down] Move  [Enter] Equip (equipment items)  [Tab/Escape] Close"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
            ));
        });
}

fn format_stat_bonus(def: &EquipmentDefinition) -> String {
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

fn slot_name(slot: &EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Weapon => "weapon",
        EquipmentSlot::Armor => "armor",
        EquipmentSlot::Accessory => "accessory",
        EquipmentSlot::Shield => "shield",
    }
}

#[allow(clippy::too_many_arguments)]
fn inventory_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory_state: ResMut<InventoryState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut party: ResMut<Party>,
    data: Res<GameData>,
    equip_panels: Query<Entity, With<EquipPanelRoot>>,
) {
    match inventory_state.phase {
        InventoryPhase::ItemList => {
            let count = inventory_state.count;

            if count > 0 {
                if keys.just_pressed(KeyCode::ArrowUp) {
                    if inventory_state.cursor == 0 {
                        inventory_state.cursor = count - 1;
                    } else {
                        inventory_state.cursor -= 1;
                    }
                } else if keys.just_pressed(KeyCode::ArrowDown) {
                    inventory_state.cursor = (inventory_state.cursor + 1) % count;
                }

                // Enter to equip if it's equipment
                if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                    let selected_item = inventory_state
                        .item_ids
                        .get(inventory_state.cursor)
                        .cloned();
                    if let Some(item_id) = selected_item {
                        if data.equipment.contains_key(&item_id) {
                            // Open equip member selection
                            inventory_state.selected_equip_id = Some(item_id.clone());
                            inventory_state.phase = InventoryPhase::EquipMemberSelect;
                            inventory_state.equip_cursor = 0;
                            inventory_state.equip_count = party.active.len();

                            // Spawn equip panel
                            let equip_def = data.equipment.get(&item_id);
                            commands
                                .spawn((
                                    EquipPanelRoot,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        right: Val::Px(20.0),
                                        top: Val::Px(100.0),
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect::all(Val::Px(16.0)),
                                        border: UiRect::all(Val::Px(2.0)),
                                        min_width: Val::Px(240.0),
                                        row_gap: Val::Px(6.0),
                                        ..default()
                                    },
                                    BackgroundColor(PANEL_BG),
                                    BorderColor(GOLD),
                                    GlobalZIndex(35),
                                ))
                                .with_children(|panel| {
                                    let title = equip_def
                                        .map(|d| format!("Equip: {}", d.name))
                                        .unwrap_or_else(|| "Equip".into());
                                    panel.spawn((
                                        Text::new(title),
                                        TextFont {
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(BRIGHT_GOLD),
                                        Node {
                                            margin: UiRect::bottom(Val::Px(8.0)),
                                            ..default()
                                        },
                                    ));

                                    for (i, unit_id) in party.active.iter().enumerate() {
                                        let unit_name = data
                                            .units
                                            .get(unit_id)
                                            .map(|u| u.name.clone())
                                            .unwrap_or_else(|| unit_id.clone());

                                        // Show current equipment in same slot
                                        let current_equip = equip_def.and_then(|eq| {
                                            party
                                                .equipment
                                                .get(unit_id)
                                                .and_then(|slots| slots.get(slot_name(&eq.slot)))
                                                .and_then(|eid| data.equipment.get(eid))
                                                .map(|e| e.name.clone())
                                        });

                                        let label = if let Some(current) = current_equip {
                                            format!("{} (has: {})", unit_name, current)
                                        } else {
                                            unit_name
                                        };

                                        let is_sel = i == 0;
                                        panel.spawn((
                                            EquipMemberEntry {
                                                index: i,
                                                unit_id: unit_id.clone(),
                                            },
                                            Text::new(if is_sel {
                                                format!("> {}", label)
                                            } else {
                                                format!("  {}", label)
                                            }),
                                            TextFont {
                                                font_size: 18.0,
                                                ..default()
                                            },
                                            TextColor(if is_sel { BRIGHT_GOLD } else { DIM_TEXT }),
                                        ));
                                    }

                                    panel.spawn((
                                        Text::new("[Up/Down] Select  [Enter] Equip  [Esc] Back"),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(DIM_TEXT),
                                        Node {
                                            margin: UiRect::top(Val::Px(8.0)),
                                            ..default()
                                        },
                                    ));
                                });
                        } else {
                            inventory_state.message = "Not equipment.".into();
                            inventory_state.message_timer.reset();
                        }
                    }
                }
            }

            if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Overworld);
            }
        }
        InventoryPhase::EquipMemberSelect => {
            let count = inventory_state.equip_count;

            if count > 0 {
                if keys.just_pressed(KeyCode::ArrowUp) {
                    if inventory_state.equip_cursor == 0 {
                        inventory_state.equip_cursor = count - 1;
                    } else {
                        inventory_state.equip_cursor -= 1;
                    }
                } else if keys.just_pressed(KeyCode::ArrowDown) {
                    inventory_state.equip_cursor = (inventory_state.equip_cursor + 1) % count;
                }
            }

            if keys.just_pressed(KeyCode::Escape) {
                // Go back to item list
                for entity in &equip_panels {
                    commands.entity(entity).despawn_recursive();
                }
                inventory_state.phase = InventoryPhase::ItemList;
                inventory_state.selected_equip_id = None;
            }

            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                if let Some(equip_id) = inventory_state.selected_equip_id.clone()
                    && let Some(unit_id) = party.active.get(inventory_state.equip_cursor).cloned()
                    && let Some(equip_def) = data.equipment.get(&equip_id)
                {
                    let slot_key = slot_name(&equip_def.slot).to_string();
                    let equip_name = equip_def.name.clone();

                    // Unequip previous item in same slot (return to inventory)
                    let old_item = party
                        .equipment
                        .entry(unit_id.clone())
                        .or_default()
                        .remove(&slot_key);
                    if let Some(old_id) = old_item {
                        party.inventory.push(old_id);
                    }

                    // Remove the new equipment from inventory
                    if let Some(pos) = party.inventory.iter().position(|id| *id == equip_id) {
                        party.inventory.remove(pos);
                    }

                    // Equip it
                    party
                        .equipment
                        .entry(unit_id.clone())
                        .or_default()
                        .insert(slot_key, equip_id.clone());

                    let unit_name = data
                        .units
                        .get(&unit_id)
                        .map(|u| u.name.clone())
                        .unwrap_or_else(|| unit_id.clone());

                    inventory_state.message = format!("Equipped {} on {}!", equip_name, unit_name);
                    inventory_state.message_timer.reset();
                }

                // Close equip panel and return to item list
                for entity in &equip_panels {
                    commands.entity(entity).despawn_recursive();
                }
                inventory_state.phase = InventoryPhase::ItemList;
                inventory_state.selected_equip_id = None;
            }
        }
    }
}

fn update_inventory_cursor_visual(
    inventory_state: Res<InventoryState>,
    mut item_query: Query<(&InventoryItemText, &mut Text, &mut TextColor)>,
    mut equip_query: Query<
        (&EquipMemberEntry, &mut Text, &mut TextColor),
        Without<InventoryItemText>,
    >,
) {
    if !inventory_state.is_changed() {
        return;
    }

    for (item, mut text, mut color) in &mut item_query {
        let selected = item.index == inventory_state.cursor;
        let prefix = if selected { "> " } else { "  " };
        let equip_marker = if item.is_equipment { " [Equip]" } else { "" };
        **text = format!("{prefix}{}{equip_marker}", item.line);
        *color = TextColor(if selected { BRIGHT_GOLD } else { DIM_TEXT });
    }

    for (entry, mut text, mut color) in &mut equip_query {
        let selected = entry.index == inventory_state.equip_cursor;
        let base = text
            .as_str()
            .trim_start_matches("> ")
            .trim_start_matches("  ");
        let prefix = if selected { "> " } else { "  " };
        **text = format!("{prefix}{base}");
        *color = TextColor(if selected { BRIGHT_GOLD } else { DIM_TEXT });
    }
}

fn update_inventory_message(
    time: Res<Time>,
    mut inventory_state: ResMut<InventoryState>,
    mut text_query: Query<&mut Text, With<InventoryMessageText>>,
) {
    inventory_state.message_timer.tick(time.delta());

    if let Ok(mut text) = text_query.get_single_mut() {
        if inventory_state.message_timer.finished() {
            **text = String::new();
        } else {
            **text = inventory_state.message.clone();
        }
    }
}

fn cleanup_inventory(mut commands: Commands, query: Query<Entity, With<InventoryRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    // Also despawn any equip panels
    // (they are separate entities, not children of InventoryRoot)

    commands.remove_resource::<InventoryState>();
}
