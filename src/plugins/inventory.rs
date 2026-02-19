use bevy::prelude::*;

use crate::plugins::core_plugin::{GameData, GameState, Party};

const INVENTORY_BG: Color = Color::srgb(0.05, 0.05, 0.12);
const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.15, 0.92);
const GOLD: Color = Color::srgb(0.85, 0.65, 0.13);
const BRIGHT_GOLD: Color = Color::srgb(1.0, 0.84, 0.0);
const DIM_TEXT: Color = Color::srgb(0.6, 0.55, 0.4);

#[derive(Component)]
struct InventoryRoot;

#[derive(Component)]
struct InventoryItemText {
    index: usize,
    line: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct InventoryCursor {
    selected: usize,
    count: usize,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
struct InventoryState {
    cursor: InventoryCursor,
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Inventory), setup_inventory_ui)
            .add_systems(
                Update,
                (inventory_input, update_inventory_cursor_visual)
                    .chain()
                    .run_if(in_state(GameState::Inventory)),
            )
            .add_systems(OnExit(GameState::Inventory), cleanup_inventory);
    }
}

fn setup_inventory_ui(mut commands: Commands, party: Res<Party>, data: Res<GameData>) {
    let mut item_lines = party
        .inventory
        .iter()
        .filter(|(_, amount)| **amount > 0)
        .map(|(item_id, amount)| {
            let (name, description) = data
                .items
                .get(item_id)
                .map(|def| (def.name.clone(), def.description.clone()))
                .unwrap_or_else(|| {
                    (
                        format!("Unknown Item ({item_id})"),
                        "No description available.".to_string(),
                    )
                });

            (
                name.to_lowercase(),
                format!("{name} x{amount} - {description}"),
            )
        })
        .collect::<Vec<_>>();

    item_lines.sort_by(|a, b| a.0.cmp(&b.0));

    let count = item_lines.len();
    commands.insert_resource(InventoryState {
        cursor: InventoryCursor { selected: 0, count },
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

                for (index, (_, line)) in item_lines.iter().enumerate() {
                    let is_selected = index == 0;
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

            root.spawn((
                Text::new("[Arrow Up/Down] Move  [Tab/Escape] Close"),
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
) {
    let count = inventory_state.cursor.count;

    if count > 0 {
        if keys.just_pressed(KeyCode::ArrowUp) {
            if inventory_state.cursor.selected == 0 {
                inventory_state.cursor.selected = count - 1;
            } else {
                inventory_state.cursor.selected -= 1;
            }
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            inventory_state.cursor.selected = (inventory_state.cursor.selected + 1) % count;
        }
    }

    if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Overworld);
    }
}

fn update_inventory_cursor_visual(
    inventory_state: Res<InventoryState>,
    mut item_query: Query<(&InventoryItemText, &mut Text, &mut TextColor)>,
) {
    if !inventory_state.is_changed() {
        return;
    }

    for (item, mut text, mut color) in &mut item_query {
        let selected = item.index == inventory_state.cursor.selected;
        let prefix = if selected { "> " } else { "  " };

        **text = format!("{prefix}{}", item.line);
        *color = TextColor(if selected { BRIGHT_GOLD } else { DIM_TEXT });
    }
}

fn cleanup_inventory(mut commands: Commands, query: Query<Entity, With<InventoryRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<InventoryState>();
}
