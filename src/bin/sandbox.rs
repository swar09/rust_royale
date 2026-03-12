use bevy::prelude::*;
use std::fs;

use rust_royale::arena::ArenaGrid;
use rust_royale::components::{PlayerState, SpawnRequest, Team};
use rust_royale::constants::{ARENA_HEIGHT, ARENA_WIDTH, TILE_SIZE};
use rust_royale::stats::{GameStats, GlobalStats};
use rust_royale::systems::{
    combat_damage_system, draw_debug_grid, draw_entities, mouse_interaction,
    physics_movement_system, setup_camera, spawn_entity_system, targeting_system, window_controls,
};

// --- CUSTOM SANDBOX SYSTEM: Dual-Wielding Spawners! ---
fn sandbox_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut spawn_events: EventWriter<SpawnRequest>,
) {
    let left_click = buttons.just_pressed(MouseButton::Left);
    let right_click = buttons.just_pressed(MouseButton::Right);

    if left_click || right_click {
        let window = window_query.single();
        let (camera, camera_transform) = camera_query.single();

        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        {
            let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
            let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

            let grid_x = ((world_position.x + total_width / 2.0) / TILE_SIZE) as i32;
            let grid_y = ((world_position.y + total_height / 2.0) / TILE_SIZE) as i32;

            if grid_x >= 0
                && grid_x < ARENA_WIDTH as i32
                && grid_y >= 0
                && grid_y < ARENA_HEIGHT as i32
            {
                // Left Click = Blue Team, Right Click = Red Team!
                let team = if left_click { Team::Blue } else { Team::Red };

                spawn_events.send(SpawnRequest {
                    card_key: "knight".to_string(),
                    team,
                    grid_x,
                    grid_y,
                });
            }
        }
    }
}

fn main() {
    let stats_file = fs::read_to_string("assets/stats.json").unwrap();
    let parsed_stats: GameStats = serde_json::from_str(&stats_file).unwrap();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust Royale - Combat Sandbox".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ArenaGrid::new())
        .insert_resource(GlobalStats(parsed_stats))
        // INFINITE ELIXIR FOR TESTING!
        .insert_resource(PlayerState { elixir: 1000.0 })
        .add_event::<SpawnRequest>()
        .add_systems(Startup, setup_camera)
        .add_systems(
            Update,
            (
                draw_debug_grid,
                mouse_interaction, // <-- Re-added the yellow highlight!
                window_controls,
                sandbox_mouse_clicks, // Use our special dual-clicker!
                spawn_entity_system,
                targeting_system,        // 1. Find a target
                combat_damage_system,    // 2. Swing the sword and kill them
                physics_movement_system, // 3. Walk forward (if target is dead)
                draw_entities,
            ),
        )
        .run();
}
