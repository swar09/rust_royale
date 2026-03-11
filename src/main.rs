mod arena;
mod components;
mod constants;
mod stats;
mod systems;

use bevy::prelude::*;
use std::fs;

use crate::arena::ArenaGrid;
use crate::components::{PlayerState, SpawnRequest};
use crate::stats::{GameStats, GlobalStats};
use crate::systems::{
    draw_debug_grid, draw_entities, elixir_generation_system, handle_mouse_clicks,
    mouse_interaction, physics_movement_system, setup_camera, setup_ui, spawn_entity_system,
    update_elixir_ui, window_controls,
};

fn main() {
    let stats_file = fs::read_to_string("assets/stats.json")
        .expect("Failed to find assets/stats.json! Make sure the folder exists.");

    // Parse the JSON text into our Rust structs
    let parsed_stats: GameStats =
        serde_json::from_str(&stats_file).expect("Failed to parse JSON! Check for typos.");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust Royale".into(),
                mode: bevy::window::WindowMode::Windowed,
                ..default()
            }),
            ..default()
        })) // Loads the window, renderer, and core engine
        .insert_resource(ArenaGrid::new())
        .insert_resource(GlobalStats(parsed_stats))
        .insert_resource(PlayerState { elixir: 5.0 }) // <-- The starting bank account!
        .add_event::<SpawnRequest>() // <-- Register the Event
        .add_systems(Startup, (setup_camera, setup_ui))
        .add_systems(
            Update,
            (
                draw_debug_grid,
                mouse_interaction,
                window_controls,
                handle_mouse_clicks,      // <-- Add click listener
                elixir_generation_system, // <-- Ticks the economy up
                update_elixir_ui,         // <-- Update the text string
                spawn_entity_system,      // <-- Add spawner logic
                physics_movement_system,  // <-- Calculate movement
                draw_entities,            // <-- Draw the new position!
            ),
        )
        .run();
}
