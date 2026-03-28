use bevy::prelude::*;
use std::fs;

use rust_royale_core::arena::ArenaGrid;
use rust_royale_core::components::{MatchState, PlayerDeck};
use rust_royale_core::stats::{GameStats, GlobalStats};
use rust_royale_engine::systems::combat::{
    combat_damage_system, projectile_flight_system, spell_impact_system, targeting_system,
};
use rust_royale_engine::systems::input::{
    handle_drag_and_drop, mouse_interaction, select_card_system, setup_camera, window_controls,
};
use rust_royale_engine::systems::match_manager::match_manager_system;
use rust_royale_engine::systems::movement::{physics_movement_system, troop_collision_system};
use rust_royale_engine::systems::spawning::{
    deployment_system, handle_death_spawns_system, spawn_entity_system, spawn_towers_system,
};
use rust_royale_engine::systems::ui::{
    draw_debug_grid, setup_ui, sync_visuals_system, update_card_bar_system, update_elixir_ui,
    update_health_text_system,
};

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
        .insert_resource(rust_royale_core::components::DragState::default())
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .add_event::<rust_royale_core::components::SpawnRequest>()
        .add_event::<rust_royale_core::components::DeathSpawnEvent>()
        .add_event::<rust_royale_core::components::TowerDeathEvent>()
        .add_systems(Startup, (setup_camera, spawn_towers_system, setup_ui))
        // Input in Update
        .add_systems(
            Update,
            (
                mouse_interaction,
                window_controls,
                select_card_system,
                handle_drag_and_drop,
            ),
        )
        // Game logic in FixedUpdate with explicit ordering
        .add_systems(
            FixedUpdate,
            (
                spawn_entity_system,
                deployment_system,
                match_manager_system,
                targeting_system,
                combat_damage_system,
                projectile_flight_system,
                spell_impact_system,
                physics_movement_system,
                troop_collision_system,
                handle_death_spawns_system,
            )
                .chain(),
        )
        // Rendering in Update
        .add_systems(
            Update,
            (
                draw_debug_grid,
                sync_visuals_system,
                update_card_bar_system,
                update_elixir_ui,
                update_health_text_system,
            ),
        )
        .run();
}
