use bevy::prelude::*;
use std::fs;

use rust_royale::arena::ArenaGrid;
use rust_royale::components::MatchState;
use rust_royale::stats::{GameStats, GlobalStats};
use rust_royale::systems::{
    combat_damage_system, deployment_system, draw_debug_grid, draw_entities, handle_mouse_clicks,
    match_manager_system, mouse_interaction, physics_movement_system, setup_camera, setup_ui,
    spawn_entity_system, spawn_towers_system, targeting_system, troop_collision_system,
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
        .insert_resource(MatchState::default())
        .add_event::<rust_royale::components::SpawnRequest>()
        .add_systems(Startup, (setup_camera, spawn_towers_system, setup_ui))
        .add_systems(
            Update,
            (
                draw_debug_grid,
                mouse_interaction,
                window_controls,
                handle_mouse_clicks,
                match_manager_system,
                spawn_entity_system,
                deployment_system,
                targeting_system,
                combat_damage_system,
                physics_movement_system,
                troop_collision_system,
                update_elixir_ui,
                draw_entities,
            ),
        )
        .run();
}
