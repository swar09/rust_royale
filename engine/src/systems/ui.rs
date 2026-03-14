use bevy::prelude::*;
use rust_royale_core::arena::{ArenaGrid, TileType};
use rust_royale_core::components::{
    ElixirUIText, MatchState, PhysicalBody, Position, TargetingProfile, Team,
};
use rust_royale_core::constants::{ARENA_HEIGHT, ARENA_WIDTH, TILE_SIZE};

/// Uses Bevy's Gizmos to draw the 18x32 wireframe matrix
pub fn draw_debug_grid(mut gizmos: Gizmos, grid: Res<ArenaGrid>) {
    let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
    let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    // Draw the Background Tiles
    for y in 0..ARENA_HEIGHT {
        for x in 0..ARENA_WIDTH {
            let color = match grid.tiles[y * ARENA_WIDTH + x] {
                TileType::Grass => Color::DARK_GREEN,
                TileType::River => Color::BLUE,
                TileType::Bridge => Color::GRAY,
                TileType::Tower => Color::GOLD,
                TileType::Wall => Color::DARK_GRAY,
            };

            let pos = Vec2::new(
                start_x + (x as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
                start_y + (y as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
            );

            // Draw a slightly smaller rect to see the grid lines
            gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 0.9), color);
        }
    }
}

pub fn draw_entities(
    mut gizmos: Gizmos,
    query: Query<(
        &Position,
        &Team,
        Option<&TargetingProfile>,
        Option<&PhysicalBody>,
    )>,
) {
    let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
    let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

    // We start from the bottom left corner
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    for (pos, team, profile, body) in query.iter() {
        // 1. Convert fixed-point (e.g., 1500) back to float grid coords (1.5)
        let float_x = pos.x as f32 / 1000.0;
        let float_y = pos.y as f32 / 1000.0;

        // 2. Multiply by tile size to get screen pixels
        let screen_x = start_x + (float_x * TILE_SIZE);
        let screen_y = start_y + (float_y * TILE_SIZE);

        let color = match team {
            Team::Blue => Color::CYAN,
            Team::Red => Color::TOMATO,
        };

        if let Some(prof) = profile {
            if prof.is_building {
                // To get the true size in pixels, we look at the 'radius' (which is footprint / 2)
                let visual_width_tiles = if let Some(b) = body {
                    // physical body radius is stored as (footprint * 1000) / 2
                    // We want the total width in tiles: (radius * 2) / 1000
                    (b.radius * 2) as f32 / 1000.0
                } else {
                    3.0 // Fallback
                };

                gizmos.rect_2d(
                    Vec2::new(screen_x, screen_y),
                    0.0,
                    Vec2::splat(TILE_SIZE * visual_width_tiles),
                    color,
                );
                continue;
            }
        }

        // Draw the walking troops as a filled circle!
        gizmos.circle_2d(Vec2::new(screen_x, screen_y), TILE_SIZE * 0.4, color);
    }
}

pub fn setup_ui(mut commands: Commands) {
    // We create a Text node and instantly give it our Marker component
    commands.spawn((
        TextBundle::from_section(
            "Elixir: 0.0", // Dummy starting text
            TextStyle {
                font_size: 40.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        }),
        ElixirUIText, // <-- THE MAKER TAG!
    ));
}

pub fn update_elixir_ui(
    match_state: Res<MatchState>, // Read the match state
    // Find exactly ONE mutable text component that also has our marker tag
    mut query: Query<&mut Text, With<ElixirUIText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        let minutes = (match_state.clock_seconds / 60.0) as u32;
        let seconds = (match_state.clock_seconds % 60.0) as u32;
        text.sections[0].value = format!(
            "⏱ {}:{:02} | 💧 Blue: {:.1} | 🔴 Red: {:.1} | 👑 {}-{}",
            minutes,
            seconds,
            match_state.blue_elixir,
            match_state.red_elixir,
            match_state.blue_crowns,
            match_state.red_crowns
        );
    }
}
