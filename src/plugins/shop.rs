use bevy::prelude::*;

use crate::plugins::core_plugin::{GameData, GameState, Party};

const SHOP_BG: Color = Color::srgba(0.02, 0.02, 0.08, 0.98);
const SHOP_TITLE: Color = Color::srgb(0.92, 0.83, 0.45);
const SHOP_HINT: Color = Color::srgb(0.62, 0.62, 0.7);
const SHOP_GOLD_TEXT: Color = Color::srgb(0.85, 0.65, 0.13);
const SHOP_ITEM_TEXT: Color = Color::srgb(0.78, 0.78, 0.82);
const SHOP_ITEM_SELECTED: Color = Color::srgb(1.0, 0.84, 0.0);
const SHOP_TAB_ACTIVE: Color = Color::srgb(1.0, 0.84, 0.0);
#[allow(dead_code)]
const SHOP_TAB_INACTIVE: Color = Color::srgb(0.45, 0.45, 0.52);
const SHOP_MESSAGE_COLOR: Color = Color::srgb(0.4, 0.9, 0.4);
const SHOP_ERROR_COLOR: Color = Color::srgb(0.9, 0.35, 0.35);
const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone, Default)]
pub struct CurrentShop {
    pub items: Vec<String>,
    pub equipment: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ShopTab {
    #[default]
    Buy,
    Sell,
}

#[derive(Resource, Debug, Clone, Default)]
struct ShopState {
    tab: ShopTab,
    cursor: usize,
    message: String,
    message_timer: f32,
    message_is_error: bool,
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct ShopRoot;

#[derive(Component)]
struct ShopGoldText;

#[derive(Component)]
struct ShopTabText;

#[derive(Component)]
struct ShopItemList;

#[derive(Component)]
struct ShopItemText {
    #[allow(dead_code)]
    index: usize,
}

#[derive(Component)]
struct ShopMessageText;

#[derive(Component)]
struct ShopHintText;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentShop>()
            .add_systems(OnEnter(GameState::Shop), setup_shop_ui)
            .add_systems(
                Update,
                (shop_input, update_shop_ui, tick_shop_message)
                    .chain()
                    .run_if(in_state(GameState::Shop)),
            )
            .add_systems(OnExit(GameState::Shop), cleanup_shop);
    }
}

// ---------------------------------------------------------------------------
// Helpers: build item lists for display
// ---------------------------------------------------------------------------

/// An entry in the shop item list for display purposes.
struct ShopEntry {
    /// The item or equipment ID.
    id: String,
    /// Display name.
    name: String,
    /// Price to show (buy price or sell price).
    price: u32,
}

fn build_buy_entries(shop: &CurrentShop, data: &GameData) -> Vec<ShopEntry> {
    let mut entries = Vec::new();

    for item_id in &shop.items {
        if let Some(def) = data.items.get(item_id) {
            entries.push(ShopEntry {
                id: def.id.clone(),
                name: def.name.clone(),
                price: def.cost,
            });
        }
    }

    for eq_id in &shop.equipment {
        if let Some(def) = data.equipment.get(eq_id) {
            entries.push(ShopEntry {
                id: def.id.clone(),
                name: def.name.clone(),
                price: def.cost,
            });
        }
    }

    entries
}

fn build_sell_entries(party: &Party, data: &GameData) -> Vec<ShopEntry> {
    let mut entries = Vec::new();

    for item_id in &party.inventory {
        let (name, cost) = if let Some(def) = data.items.get(item_id) {
            (def.name.clone(), def.cost)
        } else if let Some(def) = data.equipment.get(item_id) {
            (def.name.clone(), def.cost)
        } else {
            (format!("Unknown ({item_id})"), 0)
        };

        entries.push(ShopEntry {
            id: item_id.clone(),
            name,
            price: cost / 2,
        });
    }

    entries
}

// ---------------------------------------------------------------------------
// Setup: build the shop UI
// ---------------------------------------------------------------------------

fn setup_shop_ui(
    mut commands: Commands,
    party: Res<Party>,
    data: Res<GameData>,
    shop: Res<CurrentShop>,
) {
    let state = ShopState::default();
    let entries = build_buy_entries(&shop, &data);

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
            // Title
            root.spawn((
                Text::new("Shop"),
                TextFont {
                    font_size: 54.0,
                    ..default()
                },
                TextColor(SHOP_TITLE),
            ));

            // Gold display
            root.spawn((
                ShopGoldText,
                Text::new(format!("Gold: {}", party.gold)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(SHOP_GOLD_TEXT),
            ));

            // Tab indicators
            root.spawn((
                ShopTabText,
                Text::new(format_tab_text(state.tab)),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(SHOP_TAB_ACTIVE),
            ));

            // Item list panel
            root.spawn((
                ShopItemList,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(4.0),
                    min_height: Val::Px(200.0),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
            ))
            .with_children(|list| {
                spawn_item_entries(list, &entries, 0);
            });

            // Message area
            root.spawn((
                ShopMessageText,
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(SHOP_MESSAGE_COLOR),
            ));

            // Hint bar
            root.spawn((
                ShopHintText,
                Text::new("[Up/Down] Browse  [Enter] Buy/Sell  [Tab] Switch  [Esc] Exit"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(SHOP_HINT),
            ));
        });

    commands.insert_resource(state);
}

fn format_tab_text(tab: ShopTab) -> String {
    match tab {
        ShopTab::Buy => "[Buy]  Sell".to_string(),
        ShopTab::Sell => " Buy  [Sell]".to_string(),
    }
}

fn spawn_item_entries(parent: &mut ChildBuilder, entries: &[ShopEntry], selected: usize) {
    if entries.is_empty() {
        parent.spawn((
            Text::new("-- No items --"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(SHOP_HINT),
        ));
        return;
    }

    for (index, entry) in entries.iter().enumerate() {
        let is_selected = index == selected;
        let prefix = if is_selected { "> " } else { "  " };
        let line = format!("{prefix}{} - {}g", entry.name, entry.price);

        parent.spawn((
            ShopItemText { index },
            Text::new(line),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(if is_selected {
                SHOP_ITEM_SELECTED
            } else {
                SHOP_ITEM_TEXT
            }),
        ));
    }
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

fn shop_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut shop_state: ResMut<ShopState>,
    mut party: ResMut<Party>,
    data: Res<GameData>,
    shop: Res<CurrentShop>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Escape: exit shop
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Overworld);
        return;
    }

    // Tab: switch between Buy and Sell
    if keys.just_pressed(KeyCode::Tab) {
        shop_state.tab = match shop_state.tab {
            ShopTab::Buy => ShopTab::Sell,
            ShopTab::Sell => ShopTab::Buy,
        };
        shop_state.cursor = 0;
        return;
    }

    // Determine how many entries we have for navigation
    let entry_count = match shop_state.tab {
        ShopTab::Buy => build_buy_entries(&shop, &data).len(),
        ShopTab::Sell => build_sell_entries(&party, &data).len(),
    };

    // Arrow Up/Down: navigate
    if entry_count > 0 {
        if keys.just_pressed(KeyCode::ArrowUp) {
            if shop_state.cursor == 0 {
                shop_state.cursor = entry_count - 1;
            } else {
                shop_state.cursor -= 1;
            }
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            shop_state.cursor = (shop_state.cursor + 1) % entry_count;
        }
    }

    // Enter: execute buy or sell
    if keys.just_pressed(KeyCode::Enter) {
        match shop_state.tab {
            ShopTab::Buy => {
                let entries = build_buy_entries(&shop, &data);
                if let Some(entry) = entries.get(shop_state.cursor) {
                    if party.gold >= entry.price {
                        party.gold -= entry.price;
                        party.inventory.push(entry.id.clone());
                        shop_state.message = format!("Bought {}!", entry.name);
                        shop_state.message_timer = 2.0;
                        shop_state.message_is_error = false;
                    } else {
                        shop_state.message = "Not enough gold!".to_string();
                        shop_state.message_timer = 2.0;
                        shop_state.message_is_error = true;
                    }
                }
            }
            ShopTab::Sell => {
                let entries = build_sell_entries(&party, &data);
                if let Some(entry) = entries.get(shop_state.cursor) {
                    let sell_price = entry.price;
                    let item_name = entry.name.clone();
                    let item_id = entry.id.clone();

                    // Find and remove the first occurrence of this item from inventory
                    if let Some(pos) = party.inventory.iter().position(|id| *id == item_id) {
                        party.inventory.remove(pos);
                        party.gold += sell_price;
                        shop_state.message = format!("Sold {item_name} for {sell_price}g");
                        shop_state.message_timer = 2.0;
                        shop_state.message_is_error = false;

                        // Adjust cursor if it's now past the end of the list
                        let new_count = party.inventory.len();
                        if new_count == 0 {
                            shop_state.cursor = 0;
                        } else if shop_state.cursor >= new_count {
                            shop_state.cursor = new_count - 1;
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Message timer tick
// ---------------------------------------------------------------------------

fn tick_shop_message(time: Res<Time>, mut shop_state: ResMut<ShopState>) {
    if shop_state.message_timer > 0.0 {
        shop_state.message_timer -= time.delta_secs();
        if shop_state.message_timer <= 0.0 {
            shop_state.message.clear();
            shop_state.message_timer = 0.0;
        }
    }
}

// ---------------------------------------------------------------------------
// UI update system: refresh display each frame when state changes
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_shop_ui(
    shop_state: Res<ShopState>,
    party: Res<Party>,
    data: Res<GameData>,
    shop: Res<CurrentShop>,
    mut commands: Commands,
    mut gold_query: Query<
        &mut Text,
        (
            With<ShopGoldText>,
            Without<ShopTabText>,
            Without<ShopMessageText>,
            Without<ShopItemText>,
        ),
    >,
    mut tab_query: Query<
        &mut Text,
        (
            With<ShopTabText>,
            Without<ShopGoldText>,
            Without<ShopMessageText>,
            Without<ShopItemText>,
        ),
    >,
    mut message_query: Query<
        (&mut Text, &mut TextColor),
        (
            With<ShopMessageText>,
            Without<ShopGoldText>,
            Without<ShopTabText>,
            Without<ShopItemText>,
        ),
    >,
    item_list_query: Query<Entity, With<ShopItemList>>,
    item_text_query: Query<Entity, With<ShopItemText>>,
) {
    // Only update when something changed
    if !shop_state.is_changed() && !party.is_changed() {
        return;
    }

    // Update gold text
    for mut text in &mut gold_query {
        **text = format!("Gold: {}", party.gold);
    }

    // Update tab text
    for mut text in &mut tab_query {
        **text = format_tab_text(shop_state.tab);
    }

    // Update message text
    for (mut text, mut color) in &mut message_query {
        **text = shop_state.message.clone();
        *color = TextColor(if shop_state.message_is_error {
            SHOP_ERROR_COLOR
        } else {
            SHOP_MESSAGE_COLOR
        });
    }

    // Rebuild the item list entries
    let entries = match shop_state.tab {
        ShopTab::Buy => build_buy_entries(&shop, &data),
        ShopTab::Sell => build_sell_entries(&party, &data),
    };

    // Despawn old item text entities
    for entity in &item_text_query {
        commands.entity(entity).despawn();
    }

    // Spawn new item entries under the list container
    if let Ok(list_entity) = item_list_query.get_single() {
        commands.entity(list_entity).with_children(|list| {
            spawn_item_entries(list, &entries, shop_state.cursor);
        });
    }
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

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

#[cfg(test)]
/// Calculate the sell price for an item given its buy price.
/// Sell price is always half the buy cost (integer division).
fn calculate_sell_price(buy_cost: u32) -> u32 {
    buy_cost / 2
}

#[cfg(test)]
/// Check whether a party can afford an item at the given price.
fn can_afford(gold: u32, price: u32) -> bool {
    gold >= price
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::items::{build_equipment_registry, build_item_registry};
    use std::collections::HashMap;

    /// Helper: build a `GameData` populated only with item and equipment
    /// registries (other registries are empty).
    fn test_game_data() -> GameData {
        GameData {
            abilities: HashMap::new(),
            units: HashMap::new(),
            enemies: HashMap::new(),
            items: build_item_registry(),
            equipment: build_equipment_registry(),
            djinn: HashMap::new(),
        }
    }

    #[test]
    fn test_current_shop_default_is_empty() {
        let shop = CurrentShop::default();
        assert!(shop.items.is_empty(), "Default shop should have no items");
        assert!(
            shop.equipment.is_empty(),
            "Default shop should have no equipment"
        );
    }

    #[test]
    fn test_shop_state_default() {
        let state = ShopState::default();
        assert_eq!(state.tab, ShopTab::Buy, "Default tab should be Buy");
        assert_eq!(state.cursor, 0, "Default cursor should be 0");
        assert!(state.message.is_empty(), "Default message should be empty");
        assert_eq!(
            state.message_timer, 0.0,
            "Default message timer should be 0"
        );
        assert!(
            !state.message_is_error,
            "Default message_is_error should be false"
        );
    }

    #[test]
    fn test_calculate_sell_price_half_of_buy() {
        assert_eq!(calculate_sell_price(100), 50);
        assert_eq!(calculate_sell_price(200), 100);
        assert_eq!(calculate_sell_price(0), 0);
    }

    #[test]
    fn test_calculate_sell_price_rounds_down_odd() {
        // 99 / 2 = 49 (integer division rounds down)
        assert_eq!(calculate_sell_price(99), 49);
        assert_eq!(calculate_sell_price(1), 0);
    }

    #[test]
    fn test_can_afford() {
        assert!(can_afford(100, 50));
        assert!(can_afford(100, 100));
        assert!(!can_afford(50, 100));
        assert!(can_afford(0, 0));
        assert!(!can_afford(0, 1));
    }

    #[test]
    fn test_format_tab_text_buy() {
        let text = format_tab_text(ShopTab::Buy);
        assert!(
            text.contains("[Buy]"),
            "Buy tab text should highlight Buy: got '{text}'"
        );
    }

    #[test]
    fn test_format_tab_text_sell() {
        let text = format_tab_text(ShopTab::Sell);
        assert!(
            text.contains("[Sell]"),
            "Sell tab text should highlight Sell: got '{text}'"
        );
    }

    #[test]
    fn test_build_buy_entries_uses_registries() {
        let data = test_game_data();

        let shop = CurrentShop {
            items: vec!["potion".to_string()],
            equipment: vec!["wooden-sword".to_string()],
        };

        let entries = build_buy_entries(&shop, &data);
        assert_eq!(entries.len(), 2, "Should have one item and one equipment");

        let potion_entry = entries.iter().find(|e| e.id == "potion").unwrap();
        assert_eq!(potion_entry.name, "Potion");
        assert_eq!(potion_entry.price, 50);

        let sword_entry = entries.iter().find(|e| e.id == "wooden-sword").unwrap();
        assert_eq!(sword_entry.name, "Wooden Sword");
        assert_eq!(sword_entry.price, 50);
    }

    #[test]
    fn test_build_sell_entries_half_price() {
        let data = test_game_data();

        let party = Party {
            inventory: vec!["potion".to_string(), "wooden-sword".to_string()],
            gold: 1000,
            ..Default::default()
        };

        let entries = build_sell_entries(&party, &data);
        assert_eq!(entries.len(), 2, "Should have two sell entries");

        // Potion buy cost is 50, sell price should be 25
        let potion_entry = entries.iter().find(|e| e.id == "potion").unwrap();
        assert_eq!(potion_entry.price, 25, "Sell price should be half of buy");

        // Wooden Sword buy cost is 50, sell price should be 25
        let sword_entry = entries.iter().find(|e| e.id == "wooden-sword").unwrap();
        assert_eq!(sword_entry.price, 25, "Sell price should be half of buy");
    }

    #[test]
    fn test_sell_price_reasonable_for_all_items() {
        let items = build_item_registry();
        for (id, item) in &items {
            let sell = calculate_sell_price(item.cost);
            assert!(
                sell <= item.cost,
                "Sell price for '{id}' ({sell}) should not exceed buy cost ({})",
                item.cost,
            );
        }
    }

    #[test]
    fn test_sell_price_reasonable_for_all_equipment() {
        let equipment = build_equipment_registry();
        for (id, equip) in &equipment {
            let sell = calculate_sell_price(equip.cost);
            assert!(
                sell <= equip.cost,
                "Sell price for '{id}' ({sell}) should not exceed buy cost ({})",
                equip.cost,
            );
        }
    }
}
