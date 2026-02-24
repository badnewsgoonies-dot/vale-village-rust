use bevy::prelude::*;

use crate::data::items::EquipmentSlot;
use crate::plugins::core_plugin::{GameData, GameState, Party};

const INVENTORY_BG: Color = Color::srgb(0.05, 0.05, 0.12);
const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const GOLD: Color = Color::srgb(0.85, 0.65, 0.13);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);
const EQUIP_HIGHLIGHT: Color = Color::srgb(0.3, 0.8, 0.3);
const ERROR_TEXT: Color = Color::srgb(0.9, 0.3, 0.3);
const SWAP_SELECT_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);
#[allow(dead_code)]
const DJINN_COLOR: Color = Color::srgb(0.6, 0.4, 0.9);

#[derive(Component)]
struct InventoryRoot;

#[derive(Component)]
struct InventoryItemText {
    index: usize,
    line: String,
}

/// Marker for the unit selection list (shown in EquipSelectUnit mode).
#[derive(Component)]
struct UnitSelectText {
    index: usize,
    line: String,
}

/// Marker for the unit selection panel container.
#[derive(Component)]
struct UnitSelectPanel;

/// Marker for the party equipment summary panel.
#[derive(Component)]
struct PartyEquipPanel;

/// Marker for the hint text at the bottom.
#[derive(Component)]
struct HintText;

/// Marker for the status message text.
#[derive(Component)]
struct StatusMessageText;

/// Marker for party member text entries in the Party management tab.
#[derive(Component)]
struct PartyMemberText {
    index: usize,
    line: String,
}

/// Marker for the party management panel container (shown in Party tab).
#[derive(Component)]
struct PartyManagePanel;

/// Marker for the party member stats detail panel.
#[derive(Component)]
struct PartyStatsPanel;

/// Marker for the items tab content container.
#[derive(Component)]
struct ItemsTabContent;

/// Marker for the tab header text.
#[derive(Component)]
struct TabHeaderText;

#[derive(Debug, Clone, Copy, Default)]
struct InventoryCursor {
    selected: usize,
    count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum InventoryMode {
    #[default]
    Browse,
    EquipSelectUnit,
}

/// Which top-level tab the inventory screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum InventoryTab {
    #[default]
    Items,
    Party,
}

/// Sorted, deduplicated item IDs for stable cursor indexing.
#[derive(Resource, Debug, Clone, Default)]
struct InventoryItemList {
    ids: Vec<String>,
}

#[derive(Resource, Debug, Clone, Default)]
struct InventoryState {
    cursor: InventoryCursor,
    mode: InventoryMode,
    unit_cursor: usize,
    status_message: String,
    tab: InventoryTab,
    /// Cursor position in the party management list (indexes into combined active+bench list).
    party_cursor: usize,
    /// Total count of members in the party list (active + bench).
    party_count: usize,
    /// First selected member index for swap (None means no swap in progress).
    party_first_selected: Option<usize>,
}

/// Event fired when equipment changes require the UI to rebuild.
#[derive(Event)]
struct RebuildInventoryUi;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RebuildInventoryUi>()
            .add_systems(OnEnter(GameState::Inventory), setup_inventory_ui)
            .add_systems(
                Update,
                (
                    inventory_input,
                    rebuild_inventory_ui,
                    update_inventory_cursor_visual,
                )
                    .chain()
                    .run_if(in_state(GameState::Inventory)),
            )
            .add_systems(OnExit(GameState::Inventory), cleanup_inventory);
    }
}

/// Build the sorted, deduplicated list of inventory item IDs.
fn build_item_list(party: &Party) -> Vec<String> {
    let mut ids: Vec<String> = party.inventory.clone();
    ids.sort();
    ids.dedup();
    ids
}

/// Format a display line for an inventory item.
fn format_item_line(item_id: &str, party: &Party, data: &GameData) -> String {
    let amount = party.inventory.iter().filter(|id| *id == item_id).count();

    let (name, description) = if let Some(def) = data.items.get(item_id) {
        (def.name.clone(), def.description.clone())
    } else if let Some(def) = data.equipment.get(item_id) {
        (def.name.clone(), def.description.clone())
    } else {
        (
            format!("Unknown Item ({item_id})"),
            "No description available.".to_string(),
        )
    };

    // Check if this item is equipped on someone
    let equipped_on = find_equipped_unit(item_id, party);
    let equip_tag = if let Some(unit_id) = equipped_on {
        let unit_name = data
            .units
            .get(&unit_id)
            .map(|u| u.name.as_str())
            .unwrap_or(&unit_id);
        format!(" [E:{unit_name}]")
    } else {
        String::new()
    };

    format!("{name} x{amount}{equip_tag} - {description}")
}

/// Find which unit (if any) has this equipment_id equipped.
fn find_equipped_unit(equipment_id: &str, party: &Party) -> Option<String> {
    for (unit_id, slots) in &party.equipment {
        for eq_id in slots.values() {
            if eq_id == equipment_id {
                return Some(unit_id.clone());
            }
        }
    }
    None
}

/// Format a slot display for the party equipment summary.
fn format_slot(unit_id: &str, slot: EquipmentSlot, party: &Party, data: &GameData) -> String {
    let slot_name = format!("{:?}", slot);
    let eq_id = party
        .equipment
        .get(unit_id)
        .and_then(|slots| slots.get(&slot_name));
    let eq_name = eq_id
        .and_then(|id| data.equipment.get(id))
        .map(|def| def.name.as_str())
        .unwrap_or("--");
    format!("[{slot_name}: {eq_name}]")
}

/// Build the combined party member list: active members first, then bench members.
/// Returns (list of unit_id, count of active members).
fn build_party_member_list(party: &Party) -> (Vec<String>, usize) {
    let mut members = party.active.clone();
    let active_count = members.len();
    members.extend(party.bench.clone());
    (members, active_count)
}

/// Format a display line for a party member.
fn format_party_member_line(unit_id: &str, party: &Party, data: &GameData) -> String {
    let (name, element, role) = if let Some(def) = data.units.get(unit_id) {
        (
            def.name.as_str(),
            format!("{:?}", def.element),
            def.role.as_str(),
        )
    } else {
        (unit_id, "???".to_string(), "Unknown")
    };

    let level = party
        .unit_levels
        .get(unit_id)
        .map(|(lvl, _)| *lvl)
        .unwrap_or(1);

    format!("{name}  Lv.{level}  {element}  {role}")
}

/// Compute the stats for a unit at its current level.
fn compute_unit_stats(unit_id: &str, party: &Party, data: &GameData) -> String {
    let Some(def) = data.units.get(unit_id) else {
        return format!("Unknown unit: {unit_id}");
    };

    let (level, _xp) = party.unit_levels.get(unit_id).copied().unwrap_or((1, 0));

    let levels_gained = (level as i32) - 1;
    let hp = def.base_hp + def.growth.hp * levels_gained;
    let pp = def.base_pp + def.growth.pp * levels_gained;
    let atk = def.base_atk + def.growth.atk * levels_gained;
    let def_stat = def.base_def + def.growth.def * levels_gained;
    let mag = def.base_mag + def.growth.mag * levels_gained;
    let spd = def.base_spd + def.growth.spd * levels_gained;

    let element = format!("{:?}", def.element);

    format!(
        "{}\nLevel: {}  Element: {}\nRole: {}\nHP: {}  PP: {}\nATK: {}  DEF: {}  MAG: {}  SPD: {}",
        def.name, level, element, def.role, hp, pp, atk, def_stat, mag, spd
    )
}

fn setup_inventory_ui(mut commands: Commands, party: Res<Party>, data: Res<GameData>) {
    let item_ids = build_item_list(&party);
    let count = item_ids.len();
    let party_count = party.active.len() + party.bench.len();

    commands.insert_resource(InventoryItemList {
        ids: item_ids.clone(),
    });
    commands.insert_resource(InventoryState {
        cursor: InventoryCursor { selected: 0, count },
        mode: InventoryMode::Browse,
        unit_cursor: 0,
        status_message: String::new(),
        tab: InventoryTab::Items,
        party_cursor: 0,
        party_count,
        party_first_selected: None,
    });

    spawn_inventory_ui(&mut commands, &party, &data, &item_ids, 0);
}

/// Shared UI spawning logic used by both setup and rebuild.
fn spawn_inventory_ui(
    commands: &mut Commands,
    party: &Party,
    data: &GameData,
    item_ids: &[String],
    selected: usize,
) {
    // Read current state to determine which tab is active.
    // We don't have access to resources here, so we use a marker approach:
    // both tabs are always spawned, visibility is controlled by update_inventory_cursor_visual.
    let (party_members, _active_count) = build_party_member_list(party);

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
            // Title + Tab header
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(24.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("Inventory"),
                    TextFont {
                        font_size: 54.0,
                        ..default()
                    },
                    TextColor(BRIGHT_GOLD),
                ));

                header.spawn((
                    TabHeaderText,
                    Text::new("[Tab: Items | Party]"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
            });

            // Gold display
            root.spawn((
                Text::new(format!("Gold: {}", party.gold)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(GOLD),
            ));

            // === Items Tab Content ===
            root.spawn((
                ItemsTabContent,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    flex_grow: 1.0,
                    ..default()
                },
            ))
            .with_children(|items_tab| {
                // Main content area: item list (left) + party equipment (right)
                items_tab
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(16.0),
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|content| {
                        // Left panel: item list
                        content
                            .spawn((
                                Node {
                                    width: Val::Percent(55.0),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(14.0)),
                                    row_gap: Val::Px(6.0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BackgroundColor(PANEL_BG),
                            ))
                            .with_children(|list| {
                                if item_ids.is_empty() {
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

                                for (index, item_id) in item_ids.iter().enumerate() {
                                    let line = format_item_line(item_id, party, data);
                                    let is_selected = index == selected;
                                    let prefix = if is_selected { "> " } else { "  " };

                                    list.spawn((
                                        InventoryItemText {
                                            index,
                                            line: line.clone(),
                                        },
                                        Text::new(format!("{prefix}{line}")),
                                        TextFont {
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(if is_selected { BRIGHT_GOLD } else { DIM_TEXT }),
                                    ));
                                }
                            });

                        // Right panel: party equipment summary
                        content
                            .spawn((
                                PartyEquipPanel,
                                Node {
                                    width: Val::Percent(45.0),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(14.0)),
                                    row_gap: Val::Px(6.0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BackgroundColor(PANEL_BG),
                            ))
                            .with_children(|panel| {
                                panel.spawn((
                                    Text::new("Party Equipment:"),
                                    TextFont {
                                        font_size: 22.0,
                                        ..default()
                                    },
                                    TextColor(BRIGHT_GOLD),
                                ));

                                for unit_id in &party.active {
                                    let unit_name = data
                                        .units
                                        .get(unit_id)
                                        .map(|u| u.name.as_str())
                                        .unwrap_or(unit_id.as_str());

                                    let weapon =
                                        format_slot(unit_id, EquipmentSlot::Weapon, party, data);
                                    let armor =
                                        format_slot(unit_id, EquipmentSlot::Armor, party, data);
                                    let shield =
                                        format_slot(unit_id, EquipmentSlot::Shield, party, data);
                                    let accessory =
                                        format_slot(unit_id, EquipmentSlot::Accessory, party, data);

                                    panel.spawn((
                                        Text::new(format!(
                                            "  {unit_name}: {weapon} {armor} {shield} {accessory}"
                                        )),
                                        TextFont {
                                            font_size: 16.0,
                                            ..default()
                                        },
                                        TextColor(DIM_TEXT),
                                    ));
                                }
                            });
                    });

                // Unit selection panel (hidden by default, shown in EquipSelectUnit mode)
                items_tab
                    .spawn((
                        UnitSelectPanel,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(6.0),
                            display: Display::None,
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            Text::new("Select party member:"),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(BRIGHT_GOLD),
                        ));

                        for (index, unit_id) in party.active.iter().enumerate() {
                            let unit_name = data
                                .units
                                .get(unit_id)
                                .map(|u| u.name.as_str())
                                .unwrap_or(unit_id.as_str());
                            let is_selected = index == 0;
                            let prefix = if is_selected { "> " } else { "  " };
                            let line = unit_name.to_string();

                            panel.spawn((
                                UnitSelectText {
                                    index,
                                    line: line.clone(),
                                },
                                Text::new(format!("{prefix}{line}")),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(if is_selected {
                                    EQUIP_HIGHLIGHT
                                } else {
                                    DIM_TEXT
                                }),
                            ));
                        }
                    });
            });

            // === Party Tab Content ===
            root.spawn((
                PartyManagePanel,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    flex_grow: 1.0,
                    display: Display::None, // Hidden by default (Items tab shown first)
                    ..default()
                },
            ))
            .with_children(|party_tab| {
                // Left panel: party member list
                party_tab
                    .spawn((
                        Node {
                            width: Val::Percent(50.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ))
                    .with_children(|list| {
                        list.spawn((
                            Text::new("Party Members:"),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(BRIGHT_GOLD),
                        ));

                        if party_members.is_empty() {
                            list.spawn((
                                Text::new("No party members"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(DIM_TEXT),
                            ));
                            return;
                        }

                        let mut index = 0;
                        // Active members
                        for unit_id in &party.active {
                            let line = format_party_member_line(unit_id, party, data);
                            let is_selected = index == 0;
                            let prefix = if is_selected { "> " } else { "  " };

                            list.spawn((
                                PartyMemberText {
                                    index,
                                    line: line.clone(),
                                },
                                Text::new(format!("{prefix}{line}")),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(if is_selected { BRIGHT_GOLD } else { DIM_TEXT }),
                            ));
                            index += 1;
                        }

                        // Separator
                        if !party.bench.is_empty() {
                            list.spawn((
                                Text::new("--- Bench ---"),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(GOLD),
                            ));

                            // Bench members
                            for unit_id in &party.bench {
                                let line = format_party_member_line(unit_id, party, data);
                                let is_selected = index == 0;
                                let prefix = if is_selected { "> " } else { "  " };

                                list.spawn((
                                    PartyMemberText {
                                        index,
                                        line: line.clone(),
                                    },
                                    Text::new(format!("{prefix}{line}")),
                                    TextFont {
                                        font_size: 20.0,
                                        ..default()
                                    },
                                    TextColor(if is_selected { BRIGHT_GOLD } else { DIM_TEXT }),
                                ));
                                index += 1;
                            }
                        }
                    });

                // Right panel: selected member stats
                party_tab
                    .spawn((
                        PartyStatsPanel,
                        Node {
                            width: Val::Percent(50.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(6.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            Text::new("Unit Details:"),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(BRIGHT_GOLD),
                        ));

                        // Show stats for the first member if available
                        let stats_text = if let Some(first_id) = party_members.first() {
                            compute_unit_stats(first_id, party, data)
                        } else {
                            "No unit selected.".to_string()
                        };

                        panel.spawn((
                            Text::new(stats_text),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(DIM_TEXT),
                        ));
                    });
            });

            // Status message
            root.spawn((
                StatusMessageText,
                Text::new(""),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(EQUIP_HIGHLIGHT),
            ));

            // Hint text
            root.spawn((
                HintText,
                Text::new(
                    "[Tab] Switch Tab  [Up/Down] Move  [Enter] Equip  [U] Unequip  [Escape] Close",
                ),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
            ));
        });
}

fn inventory_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory_state: ResMut<InventoryState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut party: ResMut<Party>,
    data: Res<GameData>,
    item_list: Res<InventoryItemList>,
    mut rebuild_events: EventWriter<RebuildInventoryUi>,
) {
    match inventory_state.tab {
        InventoryTab::Items => {
            inventory_input_items(
                &keys,
                &mut inventory_state,
                &mut next_state,
                &mut party,
                &data,
                &item_list,
                &mut rebuild_events,
            );
        }
        InventoryTab::Party => {
            inventory_input_party(
                &keys,
                &mut inventory_state,
                &mut next_state,
                &mut party,
                &data,
                &mut rebuild_events,
            );
        }
    }
}

fn inventory_input_items(
    keys: &ButtonInput<KeyCode>,
    inventory_state: &mut InventoryState,
    next_state: &mut NextState<GameState>,
    party: &mut Party,
    data: &GameData,
    item_list: &InventoryItemList,
    rebuild_events: &mut EventWriter<RebuildInventoryUi>,
) {
    match inventory_state.mode {
        InventoryMode::Browse => {
            let count = inventory_state.cursor.count;

            if count > 0 {
                if keys.just_pressed(KeyCode::ArrowUp) {
                    inventory_state.status_message.clear();
                    if inventory_state.cursor.selected == 0 {
                        inventory_state.cursor.selected = count - 1;
                    } else {
                        inventory_state.cursor.selected -= 1;
                    }
                } else if keys.just_pressed(KeyCode::ArrowDown) {
                    inventory_state.status_message.clear();
                    inventory_state.cursor.selected = (inventory_state.cursor.selected + 1) % count;
                }
            }

            // Enter: attempt to equip selected item
            if keys.just_pressed(KeyCode::Enter) && count > 0 {
                let selected = inventory_state.cursor.selected;
                if let Some(item_id) = item_list.ids.get(selected) {
                    if data.equipment.contains_key(item_id) {
                        // It is equipment, switch to unit selection mode
                        if party.active.is_empty() {
                            inventory_state.status_message =
                                "No party members to equip.".to_string();
                        } else {
                            inventory_state.mode = InventoryMode::EquipSelectUnit;
                            inventory_state.unit_cursor = 0;
                            inventory_state.status_message.clear();
                        }
                    } else {
                        inventory_state.status_message = "Not an equippable item.".to_string();
                    }
                }
            }

            // U: unequip selected item
            if keys.just_pressed(KeyCode::KeyU) && count > 0 {
                let selected = inventory_state.cursor.selected;
                if let Some(item_id) = item_list.ids.get(selected).cloned() {
                    if data.equipment.contains_key(&item_id) {
                        if let Some(unit_id) = find_equipped_unit(&item_id, party) {
                            // Find the slot and remove
                            let slot_name = {
                                let mut found_slot = None;
                                if let Some(slots) = party.equipment.get(&unit_id) {
                                    for (sn, eq_id) in slots {
                                        if *eq_id == item_id {
                                            found_slot = Some(sn.clone());
                                            break;
                                        }
                                    }
                                }
                                found_slot
                            };
                            if let Some(slot_name) = slot_name {
                                if let Some(slots) = party.equipment.get_mut(&unit_id) {
                                    slots.remove(&slot_name);
                                }
                                party.inventory.push(item_id.clone());
                                let eq_name = data
                                    .equipment
                                    .get(&item_id)
                                    .map(|d| d.name.as_str())
                                    .unwrap_or(&item_id);
                                let unit_name = data
                                    .units
                                    .get(&unit_id)
                                    .map(|u| u.name.as_str())
                                    .unwrap_or(&unit_id);
                                inventory_state.status_message =
                                    format!("Unequipped {eq_name} from {unit_name}.");
                                rebuild_events.send(RebuildInventoryUi);
                            }
                        } else {
                            inventory_state.status_message =
                                "Item is not currently equipped.".to_string();
                        }
                    } else {
                        inventory_state.status_message = "Not an equippable item.".to_string();
                    }
                }
            }

            // Tab: switch to Party tab
            if keys.just_pressed(KeyCode::Tab) {
                inventory_state.tab = InventoryTab::Party;
                inventory_state.status_message.clear();
                inventory_state.party_first_selected = None;
                let party_count = party.active.len() + party.bench.len();
                inventory_state.party_count = party_count;
                if inventory_state.party_cursor >= party_count {
                    inventory_state.party_cursor = 0;
                }
                // Trigger rebuild to update stats panel for current cursor
                rebuild_events.send(RebuildInventoryUi);
                return;
            }

            // Escape: close inventory
            if keys.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Overworld);
            }
        }
        InventoryMode::EquipSelectUnit => {
            let unit_count = party.active.len();

            if unit_count > 0 {
                if keys.just_pressed(KeyCode::ArrowUp) {
                    if inventory_state.unit_cursor == 0 {
                        inventory_state.unit_cursor = unit_count - 1;
                    } else {
                        inventory_state.unit_cursor -= 1;
                    }
                } else if keys.just_pressed(KeyCode::ArrowDown) {
                    inventory_state.unit_cursor = (inventory_state.unit_cursor + 1) % unit_count;
                }
            }

            // Enter: equip the item on the selected unit
            if keys.just_pressed(KeyCode::Enter) {
                let selected_item_idx = inventory_state.cursor.selected;
                let selected_unit_idx = inventory_state.unit_cursor;

                if let Some(item_id) = item_list.ids.get(selected_item_idx).cloned()
                    && let Some(eq_def) = data.equipment.get(&item_id)
                    && let Some(unit_id) = party.active.get(selected_unit_idx).cloned()
                {
                    let unit_element = data
                        .units
                        .get(&unit_id)
                        .map(|u| u.element)
                        .unwrap_or_default();

                    // Check element compatibility
                    if !eq_def.allowed_elements.is_empty()
                        && !eq_def.allowed_elements.contains(&unit_element)
                    {
                        let unit_name = data
                            .units
                            .get(&unit_id)
                            .map(|u| u.name.as_str())
                            .unwrap_or(&unit_id);
                        inventory_state.status_message = format!(
                            "{unit_name} cannot equip {} (element incompatible).",
                            eq_def.name
                        );
                    } else {
                        let slot_name = format!("{:?}", eq_def.slot);

                        // If the unit already has something in that slot, unequip it
                        let old_eq_id = party
                            .equipment
                            .get(&unit_id)
                            .and_then(|slots| slots.get(&slot_name))
                            .cloned();
                        if let Some(old_id) = old_eq_id {
                            party.inventory.push(old_id);
                        }

                        // Remove the new item from inventory (first occurrence only)
                        if let Some(pos) = party.inventory.iter().position(|id| *id == item_id) {
                            party.inventory.remove(pos);
                        }

                        // Record in equipment map
                        party
                            .equipment
                            .entry(unit_id.clone())
                            .or_default()
                            .insert(slot_name, item_id.clone());

                        let unit_name = data
                            .units
                            .get(&unit_id)
                            .map(|u| u.name.as_str())
                            .unwrap_or(&unit_id);
                        inventory_state.status_message =
                            format!("Equipped {} on {unit_name}.", eq_def.name);

                        // Go back to browse mode
                        inventory_state.mode = InventoryMode::Browse;
                        rebuild_events.send(RebuildInventoryUi);
                    }
                }
            }

            // Escape: cancel unit selection, go back to browse
            if keys.just_pressed(KeyCode::Escape) {
                inventory_state.mode = InventoryMode::Browse;
                inventory_state.status_message.clear();
            }
        }
    }
}

fn inventory_input_party(
    keys: &ButtonInput<KeyCode>,
    inventory_state: &mut InventoryState,
    next_state: &mut NextState<GameState>,
    party: &mut Party,
    data: &GameData,
    rebuild_events: &mut EventWriter<RebuildInventoryUi>,
) {
    let party_count = inventory_state.party_count;

    // Tab: switch back to Items tab
    if keys.just_pressed(KeyCode::Tab) {
        inventory_state.tab = InventoryTab::Items;
        inventory_state.status_message.clear();
        inventory_state.party_first_selected = None;
        rebuild_events.send(RebuildInventoryUi);
        return;
    }

    // Escape: if a swap is in progress, cancel it; otherwise close inventory
    if keys.just_pressed(KeyCode::Escape) {
        if inventory_state.party_first_selected.is_some() {
            inventory_state.party_first_selected = None;
            inventory_state.status_message = "Swap cancelled.".to_string();
        } else {
            next_state.set(GameState::Overworld);
        }
        return;
    }

    if party_count == 0 {
        return;
    }

    // Arrow key navigation
    if keys.just_pressed(KeyCode::ArrowUp) {
        inventory_state.status_message.clear();
        if inventory_state.party_cursor == 0 {
            inventory_state.party_cursor = party_count - 1;
        } else {
            inventory_state.party_cursor -= 1;
        }
        rebuild_events.send(RebuildInventoryUi);
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        inventory_state.status_message.clear();
        inventory_state.party_cursor = (inventory_state.party_cursor + 1) % party_count;
        rebuild_events.send(RebuildInventoryUi);
    }

    // Enter: select member for swap
    if keys.just_pressed(KeyCode::Enter) {
        let current = inventory_state.party_cursor;
        let active_count = party.active.len();

        if let Some(first) = inventory_state.party_first_selected {
            // Second selection: perform the swap
            if first == current {
                // Same member selected twice, cancel swap
                inventory_state.party_first_selected = None;
                inventory_state.status_message = "Swap cancelled (same member).".to_string();
                return;
            }

            // Prevent swapping that would leave active party empty
            // This can happen if both selections are from active and active has only 1 member
            // but that's really a swap within active which is fine.
            // The real concern is: cannot move the last active member to bench.
            let first_is_active = first < active_count;
            let second_is_active = current < active_count;

            if first_is_active && second_is_active {
                // Both in active: swap positions within active
                party.active.swap(first, current);
                let name_a = data
                    .units
                    .get(&party.active[first])
                    .map(|u| u.name.as_str())
                    .unwrap_or("???");
                let name_b = data
                    .units
                    .get(&party.active[current])
                    .map(|u| u.name.as_str())
                    .unwrap_or("???");
                inventory_state.status_message =
                    format!("Swapped {name_a} and {name_b} positions.");
            } else if !first_is_active && !second_is_active {
                // Both on bench: swap positions within bench
                let bench_a = first - active_count;
                let bench_b = current - active_count;
                party.bench.swap(bench_a, bench_b);
                let name_a = data
                    .units
                    .get(&party.bench[bench_a])
                    .map(|u| u.name.as_str())
                    .unwrap_or("???");
                let name_b = data
                    .units
                    .get(&party.bench[bench_b])
                    .map(|u| u.name.as_str())
                    .unwrap_or("???");
                inventory_state.status_message = format!("Swapped {name_a} and {name_b} on bench.");
            } else {
                // One active, one bench: swap between active and bench
                let (active_idx, bench_idx) = if first_is_active {
                    (first, current - active_count)
                } else {
                    (current, first - active_count)
                };

                // Check: cannot leave active empty (active must have at least 1)
                // This swap replaces one active with one bench, so active count stays the same.
                // So this is always safe as long as active_count >= 1 (which it must be).

                let active_id = party.active[active_idx].clone();
                let bench_id = party.bench[bench_idx].clone();

                party.active[active_idx] = bench_id.clone();
                party.bench[bench_idx] = active_id.clone();

                let name_active = data
                    .units
                    .get(&active_id)
                    .map(|u| u.name.as_str())
                    .unwrap_or("???");
                let name_bench = data
                    .units
                    .get(&bench_id)
                    .map(|u| u.name.as_str())
                    .unwrap_or("???");
                inventory_state.status_message =
                    format!("Swapped {name_bench} (to active) and {name_active} (to bench).");
            }

            inventory_state.party_first_selected = None;
            // Update party count (shouldn't change but be safe)
            inventory_state.party_count = party.active.len() + party.bench.len();
            rebuild_events.send(RebuildInventoryUi);
        } else {
            // First selection
            let (members, _) = build_party_member_list(party);
            let unit_name = members
                .get(current)
                .and_then(|id| data.units.get(id).map(|u| u.name.as_str()))
                .unwrap_or("???");
            inventory_state.party_first_selected = Some(current);
            inventory_state.status_message =
                format!("Selected {unit_name}. Choose another member to swap.");
        }
    }
}

/// When a RebuildInventoryUi event fires, tear down the UI and rebuild it.
fn rebuild_inventory_ui(
    mut commands: Commands,
    query: Query<Entity, With<InventoryRoot>>,
    mut events: EventReader<RebuildInventoryUi>,
    party: Res<Party>,
    data: Res<GameData>,
    old_state: Res<InventoryState>,
) {
    if events.read().next().is_none() {
        return;
    }
    // Drain remaining events
    for _ev in events.read() {}

    // Despawn old UI
    for entity in &query {
        commands.entity(entity).despawn();
    }

    // Rebuild item list
    let item_ids = build_item_list(&party);
    let count = item_ids.len();

    // Preserve cursor position, clamping if needed
    let new_selected = if count == 0 {
        0
    } else {
        old_state.cursor.selected.min(count - 1)
    };

    let party_count = party.active.len() + party.bench.len();
    let new_party_cursor = if party_count == 0 {
        0
    } else {
        old_state.party_cursor.min(party_count - 1)
    };

    let status_msg = old_state.status_message.clone();

    commands.insert_resource(InventoryItemList {
        ids: item_ids.clone(),
    });
    commands.insert_resource(InventoryState {
        cursor: InventoryCursor {
            selected: new_selected,
            count,
        },
        mode: old_state.mode,
        unit_cursor: old_state.unit_cursor,
        status_message: status_msg,
        tab: old_state.tab,
        party_cursor: new_party_cursor,
        party_count,
        party_first_selected: old_state.party_first_selected,
    });

    // Respawn the full UI with current state
    spawn_inventory_ui(&mut commands, &party, &data, &item_ids, new_selected);
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_inventory_cursor_visual(
    inventory_state: Res<InventoryState>,
    party: Res<Party>,
    data: Res<GameData>,
    mut item_query: Query<(&InventoryItemText, &mut Text, &mut TextColor)>,
    mut unit_query: Query<(&UnitSelectText, &mut Text, &mut TextColor), Without<InventoryItemText>>,
    mut panel_query: Query<
        &mut Node,
        (
            With<UnitSelectPanel>,
            Without<PartyManagePanel>,
            Without<ItemsTabContent>,
        ),
    >,
    mut hint_query: Query<
        (&HintText, &mut Text, &mut TextColor),
        (
            Without<InventoryItemText>,
            Without<UnitSelectText>,
            Without<PartyMemberText>,
        ),
    >,
    mut status_query: Query<
        (&StatusMessageText, &mut Text, &mut TextColor),
        (
            Without<InventoryItemText>,
            Without<UnitSelectText>,
            Without<HintText>,
            Without<PartyMemberText>,
        ),
    >,
    mut party_member_query: Query<
        (&PartyMemberText, &mut Text, &mut TextColor),
        (
            Without<InventoryItemText>,
            Without<UnitSelectText>,
            Without<HintText>,
            Without<StatusMessageText>,
        ),
    >,
    mut party_panel_query: Query<
        &mut Node,
        (
            With<PartyManagePanel>,
            Without<UnitSelectPanel>,
            Without<ItemsTabContent>,
        ),
    >,
    mut items_tab_query: Query<
        &mut Node,
        (
            With<ItemsTabContent>,
            Without<UnitSelectPanel>,
            Without<PartyManagePanel>,
        ),
    >,
    mut tab_header_query: Query<
        (&TabHeaderText, &mut Text, &mut TextColor),
        (
            Without<InventoryItemText>,
            Without<UnitSelectText>,
            Without<HintText>,
            Without<StatusMessageText>,
            Without<PartyMemberText>,
        ),
    >,
    mut stats_panel_query: Query<Entity, With<PartyStatsPanel>>,
    mut commands: Commands,
) {
    if !inventory_state.is_changed() {
        return;
    }

    let is_items_tab = inventory_state.tab == InventoryTab::Items;

    // Show/hide tab content
    for mut node in &mut items_tab_query {
        node.display = if is_items_tab {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut party_panel_query {
        node.display = if is_items_tab {
            Display::None
        } else {
            Display::Flex
        };
    }

    // Update tab header text
    for (_header, mut text, _color) in &mut tab_header_query {
        **text = match inventory_state.tab {
            InventoryTab::Items => "[Items] | Party".to_string(),
            InventoryTab::Party => "Items | [Party]".to_string(),
        };
    }

    // Update item list cursor
    for (item, mut text, mut color) in &mut item_query {
        let is_selected = is_items_tab
            && inventory_state.mode == InventoryMode::Browse
            && item.index == inventory_state.cursor.selected;
        let prefix = if is_selected { "> " } else { "  " };

        **text = format!("{prefix}{}", item.line);
        *color = TextColor(if is_selected { BRIGHT_GOLD } else { DIM_TEXT });
    }

    // Update unit selection cursor
    for (unit_text, mut text, mut color) in &mut unit_query {
        let is_selected = is_items_tab
            && inventory_state.mode == InventoryMode::EquipSelectUnit
            && unit_text.index == inventory_state.unit_cursor;
        let prefix = if is_selected { "> " } else { "  " };

        **text = format!("{prefix}{}", unit_text.line);
        *color = TextColor(if is_selected {
            EQUIP_HIGHLIGHT
        } else {
            DIM_TEXT
        });
    }

    // Show/hide unit selection panel
    for mut node in &mut panel_query {
        node.display = if is_items_tab && inventory_state.mode == InventoryMode::EquipSelectUnit {
            Display::Flex
        } else {
            Display::None
        };
    }

    // Update party member list cursor and highlights
    for (member, mut text, mut color) in &mut party_member_query {
        let is_cursor = !is_items_tab && member.index == inventory_state.party_cursor;
        let is_first_selected =
            !is_items_tab && inventory_state.party_first_selected == Some(member.index);
        let prefix = if is_cursor { "> " } else { "  " };

        **text = format!("{prefix}{}", member.line);
        if is_first_selected {
            *color = TextColor(SWAP_SELECT_COLOR);
        } else if is_cursor {
            *color = TextColor(BRIGHT_GOLD);
        } else {
            *color = TextColor(DIM_TEXT);
        }
    }

    // Update party stats panel content
    if !is_items_tab {
        let (members, _) = build_party_member_list(&party);
        if let Some(unit_id) = members.get(inventory_state.party_cursor) {
            let stats_text = compute_unit_stats(unit_id, &party, &data);

            // Rebuild stats panel children
            for entity in &mut stats_panel_query {
                commands.entity(entity).despawn_descendants();
                commands.entity(entity).with_children(|panel| {
                    panel.spawn((
                        Text::new("Unit Details:"),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(BRIGHT_GOLD),
                    ));

                    panel.spawn((
                        Text::new(stats_text.clone()),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(DIM_TEXT),
                    ));
                });
            }
        }
    }

    // Update hint text based on mode
    for (_hint, mut text, _color) in &mut hint_query {
        **text = match inventory_state.tab {
            InventoryTab::Items => match inventory_state.mode {
                InventoryMode::Browse => {
                    "[Tab] Switch Tab  [Up/Down] Move  [Enter] Equip  [U] Unequip  [Escape] Close"
                        .to_string()
                }
                InventoryMode::EquipSelectUnit => {
                    "[Up/Down] Select Unit  [Enter] Confirm  [Escape] Cancel".to_string()
                }
            },
            InventoryTab::Party => {
                if inventory_state.party_first_selected.is_some() {
                    "[Up/Down] Move  [Enter] Swap  [Escape] Cancel Swap  [Tab] Switch Tab"
                        .to_string()
                } else {
                    "[Tab] Switch Tab  [Up/Down] Move  [Enter] Select for Swap  [Escape] Close"
                        .to_string()
                }
            }
        };
    }

    // Update status message
    for (_status, mut text, mut color) in &mut status_query {
        **text = inventory_state.status_message.clone();
        if inventory_state.status_message.contains("cannot")
            || inventory_state.status_message.contains("Not ")
            || inventory_state.status_message.contains("No party")
            || inventory_state.status_message.contains("not currently")
            || inventory_state.status_message.contains("cancelled")
        {
            *color = TextColor(ERROR_TEXT);
        } else {
            *color = TextColor(EQUIP_HIGHLIGHT);
        }
    }
}

fn cleanup_inventory(mut commands: Commands, query: Query<Entity, With<InventoryRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<InventoryState>();
    commands.remove_resource::<InventoryItemList>();
}
