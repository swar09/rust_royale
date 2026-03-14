use bevy::prelude::*;
use std::fs;

use rust_royale_core::arena::ArenaGrid;
use rust_royale_core::components::MatchState;
use rust_royale_core::stats::{GameStats, GlobalStats};
use rust_royale_engine::systems::combat::{combat_damage_system, targeting_system};
use rust_royale_engine::systems::input::{
    handle_mouse_clicks, mouse_interaction, setup_camera, window_controls,
};
use rust_royale_engine::systems::match_manager::match_manager_system;
use rust_royale_engine::systems::movement::{physics_movement_system, troop_collision_system};
use rust_royale_engine::systems::spawning::{
    deployment_system, spawn_entity_system, spawn_towers_system,
};
use rust_royale_engine::systems::ui::{draw_debug_grid, draw_entities, setup_ui, update_elixir_ui};

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
        .add_event::<rust_royale_core::components::SpawnRequest>()
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
