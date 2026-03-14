use bevy::prelude::*;
use std::fs;

use rust_royale_core::arena::ArenaGrid;
use rust_royale_core::components::{MatchState, PlayerDeck};
use rust_royale_core::stats::{GameStats, GlobalStats};
use rust_royale_engine::systems::combat::{combat_damage_system, targeting_system};
use rust_royale_engine::systems::input::{
    handle_mouse_clicks, mouse_interaction, select_card_system, setup_camera, window_controls,
};
use rust_royale_engine::systems::match_manager::match_manager_system;
use rust_royale_engine::systems::movement::{physics_movement_system, troop_collision_system};
use rust_royale_engine::systems::spawning::{
    deployment_system, spawn_entity_system, spawn_towers_system,
};
use rust_royale_engine::systems::ui::{draw_debug_grid, draw_entities, setup_ui, update_elixir_ui};

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
        .insert_resource(MatchState::default())
        .insert_resource(PlayerDeck::default())
        .add_event::<rust_royale_core::components::SpawnRequest>()
        .add_systems(Startup, (setup_camera, spawn_towers_system, setup_ui))
        .add_systems(
            Update,
            (
                draw_debug_grid,
                mouse_interaction,
                window_controls,
                select_card_system,
                handle_mouse_clicks,
                match_manager_system, // <-- THE NEW CLOCK AND ECONOMY
                spawn_entity_system,
                deployment_system,
                targeting_system,
                combat_damage_system,
                physics_movement_system,
                troop_collision_system,
                update_elixir_ui, // <-- SHOWS CLOCK, ELIXIR, AND CROWNS
                draw_entities,
            ),
        )
        .run();
}
