use bevy::prelude::*;
use rust_royale_core::arena::{ArenaGrid, TileType};
use rust_royale_core::components::{
    CardUI, ElixirUIText, Health, HealthValueText, MatchState, PlayerDeck, Position, Team,
    TowerFootprint, TowerType,
};
use rust_royale_core::constants::{ARENA_HEIGHT, ARENA_WIDTH, TILE_SIZE};

/// Uses Bevy's Gizmos to draw the 18x32 wireframe matrix
pub fn draw_debug_grid(
    mut gizmos: Gizmos,
    grid: Res<ArenaGrid>,
    towers: Query<(&Team, &TowerType, &TowerFootprint)>,
) {
    let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
    let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    let mut red_left_alive = false;
    let mut red_right_alive = false;

    let divider = ARENA_WIDTH / 2;

    for (t_team, t_type, footprint) in towers.iter() {
        if matches!(t_type, TowerType::Princess) && *t_team == Team::Red {
            if footprint.start_x < divider {
                red_left_alive = true;
            } else {
                red_right_alive = true;
            }
        }
    }

    let blue_max_y_left  = if red_left_alive  { 14 } else { 20 };
    let blue_max_y_right = if red_right_alive { 14 } else { 20 };

    for y in 0..ARENA_HEIGHT {
        for x in 0..ARENA_WIDTH {
            let tile = &grid.tiles[y * ARENA_WIDTH + x];

            let pos = Vec2::new(
                start_x + (x as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
                start_y + (y as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
            );

            let is_left_lane  = x < divider;
            let is_valid_depth = if is_left_lane {
                y as i32 <= blue_max_y_left
            } else {
                y as i32 <= blue_max_y_right
            };

            let color = match tile {
                TileType::River  => Color::rgb(0.0, 0.4, 0.8),
                TileType::Bridge => Color::rgb(0.5, 0.3, 0.1),
                TileType::Grass  => {
                    if is_valid_depth {
                        Color::rgb(0.2, 0.7, 0.2)
                    } else {
                        Color::rgb(0.1, 0.3, 0.1)
                    }
                }
                TileType::Tower => Color::rgb(0.6, 0.6, 0.2),
                TileType::Wall  => Color::rgb(0.3, 0.3, 0.3),
            };

            gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 0.95), color);
        }
    }
}

pub fn sync_visuals_system(
    // Separate queries for towers and troops so we can set different z values.
    // Towers at z=0, troops at z=1 — ensures troops always render above tower
    // sprites when they're adjacent or overlapping (e.g. knight attacking a tower).
    mut troop_query: Query<
        (&Position, &mut Transform),
        (With<Sprite>, Without<HealthValueText>, Without<TowerType>),
    >,
    mut tower_query: Query<
        (&Position, &mut Transform),
        (With<Sprite>, Without<HealthValueText>, With<TowerType>),
    >,
) {
    let total_width  = ARENA_WIDTH  as f32 * TILE_SIZE;
    let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;
    let start_x = -total_width  / 2.0;
    let start_y = -total_height / 2.0;

    // Towers at z=0
    for (pos, mut transform) in tower_query.iter_mut() {
        let float_x = pos.x as f32 / 1000.0;
        let float_y = pos.y as f32 / 1000.0;
        transform.translation.x = start_x + (float_x * TILE_SIZE);
        transform.translation.y = start_y + (float_y * TILE_SIZE);
        transform.translation.z = 0.0;
    }

    // Troops at z=1 — always on top of towers
    for (pos, mut transform) in troop_query.iter_mut() {
        let float_x = pos.x as f32 / 1000.0;
        let float_y = pos.y as f32 / 1000.0;
        transform.translation.x = start_x + (float_x * TILE_SIZE);
        transform.translation.y = start_y + (float_y * TILE_SIZE);
        transform.translation.z = 1.0;
    }
}

pub fn update_health_text_system(
    parent_query: Query<&Health, Changed<Health>>,
    mut text_query: Query<(&Parent, &mut Text), With<HealthValueText>>,
) {
    for (parent, mut text) in text_query.iter_mut() {
        let parent_entity = parent.get();
        if let Ok(health) = parent_query.get(parent_entity) {
            text.sections[0].value = health.0.to_string();
        }
    }
}

pub fn setup_ui(mut commands: Commands) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Loading HUD...\n",
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::new(
                "Blue Hand\n",
                TextStyle {
                    font_size: 24.0,
                    color: Color::CYAN,
                    ..default()
                },
            ),
            TextSection::new(
                "Red Hand",
                TextStyle {
                    font_size: 24.0,
                    color: Color::TOMATO,
                    ..default()
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        ElixirUIText,
    ));

    // Spawn the Card Bar Container for Blue (bottom of screen)
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(120.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(15.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for i in 0..4 {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(80.0),
                                height: Val::Px(100.0),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            border_color: Default::default(),
                            background_color: Color::rgb(0.2, 0.2, 0.2).into(),
                            ..default()
                        },
                        CardUI {
                            slot_index: i,
                            team: Team::Blue,
                        },
                    ))
                    .with_children(|card| {
                        card.spawn(TextBundle::from_section(
                            format!("Card {}", i + 1),
                            TextStyle {
                                font_size: 16.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ));
                    });
            }
        });
}

pub fn update_elixir_ui(
    match_state: Res<MatchState>,
    deck: Res<PlayerDeck>,
    mut query: Query<&mut Text, With<ElixirUIText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        let minutes = (match_state.clock_seconds / 60.0) as u32;
        let seconds  = (match_state.clock_seconds % 60.0) as u32;

        let blue_sel = deck.blue_selected;
        let red_sel = deck.red_selected;
        let selected_text = blue_sel
            .map(|i| format!("{}", i + 1))
            .unwrap_or_else(|| "None".to_string());

        text.sections[0].value = format!(
            "⏱ {}:{:02} | 👑 {}-{} | Selected Slot: {}\n",
            minutes, seconds, match_state.blue_crowns, match_state.red_crowns, selected_text
        );

        let mut blue_str = format!("💧 Blue {:.1}: ", match_state.blue_elixir);
        for i in 0..4 {
            let card = deck.blue.hand[i].as_deref().unwrap_or("---");
            if blue_sel == Some(i) {
                blue_str += &format!("[{}]{}* ", i + 1, card.to_uppercase());
            } else {
                blue_str += &format!("[{}]{} ", i + 1, card);
            }
        }
        text.sections[1].value = blue_str + "\n";

        let mut red_str = format!("🔴 Red  {:.1}: ", match_state.red_elixir);
        for i in 0..4 {
            let card = deck.red.hand[i].as_deref().unwrap_or("---");
            if red_sel == Some(i) {
                red_str += &format!("[{}]{}* ", i + 1, card.to_uppercase());
            } else {
                red_str += &format!("[{}]{} ", i + 1, card);
            }
        }
        text.sections[2].value = red_str;
    }
}

pub fn update_card_bar_system(
    deck: Res<PlayerDeck>,
    mut card_query: Query<(&CardUI, &mut BackgroundColor, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    for (card_ui, mut bg_color, children) in card_query.iter_mut() {
        if card_ui.team == Team::Blue {
            let card_name = deck.blue.hand[card_ui.slot_index]
                .as_deref()
                .unwrap_or("Empty");
            
            // Highlight selected card
            if deck.blue_selected == Some(card_ui.slot_index) {
                *bg_color = Color::rgb(0.5, 0.5, 0.2).into();
            } else {
                *bg_color = Color::rgb(0.2, 0.2, 0.2).into();
            }

            // Update text
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].value = card_name.to_string();
                }
            }
        }
    }
}