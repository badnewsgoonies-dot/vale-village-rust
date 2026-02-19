use bevy::prelude::*;

use crate::plugins::core_plugin::GameState;

const SHOP_BG: Color = Color::srgba(0.02, 0.02, 0.08, 0.98);
const SHOP_TITLE: Color = Color::srgb(0.92, 0.83, 0.45);
const SHOP_HINT: Color = Color::srgb(0.62, 0.62, 0.7);

#[derive(Resource, Debug, Clone, Default)]
pub struct CurrentShop {
    pub items: Vec<String>,
    pub equipment: Vec<String>,
}

#[derive(Component)]
struct ShopRoot;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentShop>()
            .add_systems(OnEnter(GameState::Shop), setup_shop_ui)
            .add_systems(Update, shop_input.run_if(in_state(GameState::Shop)))
            .add_systems(OnExit(GameState::Shop), cleanup_shop);
    }
}

fn setup_shop_ui(mut commands: Commands) {
    commands
        .spawn((
            ShopRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(SHOP_BG),
            GlobalZIndex(30),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Shop"),
                TextFont {
                    font_size: 58.0,
                    ..default()
                },
                TextColor(SHOP_TITLE),
            ));

            root.spawn((
                Text::new("Press Esc to exit"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(SHOP_HINT),
            ));
        });
}

fn shop_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Overworld);
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
}
