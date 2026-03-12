use bevy::prelude::*;
use std::fs;


use rust_royale::arena::ArenaGrid;
use rust_royale::components::PlayerState;
use rust_royale::stats::{GameStats, GlobalStats};
use rust_royale::systems::{
    draw_debug_grid, draw_entities, elixir_generation_system, handle_mouse_clicks,
    mouse_interaction, physics_movement_system, setup_camera, setup_ui, spawn_entity_system,
    update_elixir_ui, window_controls,
};

fn main() {
    let stats_file = fs::read_to_string("assets/stats.json")
        .expect("Failed to find assets/stats.json! Make sure the folder exists.");
    let parsed_stats: GameStats = serde_json::from_str(&stats_file).expect("Failed to parse JSON!");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust Royale - Official Match".into(),
                mode: bevy::window::WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ArenaGrid::new())
        .insert_resource(GlobalStats(parsed_stats))
        .insert_resource(PlayerState { elixir: 5.0 })
        .add_event::<rust_royale::components::SpawnRequest>()
        .add_systems(Startup, (setup_camera, setup_ui))
        .add_systems(
            Update,
            (
                draw_debug_grid,
                mouse_interaction,
                window_controls,
                handle_mouse_clicks,
                elixir_generation_system,
                update_elixir_ui,
                spawn_entity_system,
                physics_movement_system,
                draw_entities,
            ),
        )
        .run();
}
