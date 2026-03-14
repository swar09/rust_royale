use bevy::prelude::*;
use rust_royale_core::components::{
    Health, MatchPhase, MatchState, Team, TowerFootprint, TowerType,
};

pub fn match_manager_system(
    mut commands: Commands,
    time: Res<Time>,
    mut match_state: ResMut<MatchState>,
    mut grid: ResMut<rust_royale_core::arena::ArenaGrid>,
    towers: Query<(Entity, &Health, &Team, &TowerType, &TowerFootprint)>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return;
    }

    let delta = time.delta_seconds();
    match_state.clock_seconds -= delta;

    // Phase Transitions
    if match_state.phase == MatchPhase::Regular && match_state.clock_seconds <= 60.0 {
        match_state.phase = MatchPhase::DoubleElixir;
        println!("🕒 60 SECONDS LEFT: DOUBLE ELIXIR!");
    } else if match_state.clock_seconds <= 0.0 {
        if match_state.phase == MatchPhase::DoubleElixir {
            if match_state.blue_crowns == match_state.red_crowns {
                match_state.phase = MatchPhase::Overtime;
                match_state.clock_seconds = 60.0; // 1 Minute of Overtime
                println!("⚔️ OVERTIME! SUDDEN DEATH!");
            } else {
                match_state.phase = MatchPhase::GameOver;
                match_state.clock_seconds = 0.0;
                println!(
                    "🛑 MATCH OVER! Final Score: {}-{}",
                    match_state.blue_crowns, match_state.red_crowns
                );
            }
        } else if match_state.phase == MatchPhase::Overtime {
            // --- TIEBREAKER: Destroy the tower with the lowest HP ---
            match_state.clock_seconds = 0.0;

            let mut weakest: Option<(Entity, i32, Team, u8, TowerFootprint)> = None;

            for (entity, health, team, tower_type, footprint) in towers.iter() {
                let crowns_worth = match tower_type {
                    TowerType::Princess => 1_u8,
                    TowerType::King => 3_u8,
                };

                let is_weaker = match &weakest {
                    None => true,
                    Some((_, lowest_hp, _, _, _)) => health.0 < *lowest_hp,
                };

                if is_weaker {
                    weakest = Some((
                        entity,
                        health.0,
                        *team,
                        crowns_worth,
                        TowerFootprint {
                            start_x: footprint.start_x,
                            start_y: footprint.start_y,
                            size: footprint.size,
                        },
                    ));
                }
            }

            if let Some((entity, hp, team, crowns, footprint)) = weakest {
                commands.entity(entity).despawn();
                grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);

                if team == Team::Red {
                    if crowns == 3 {
                        match_state.blue_crowns = 3; // King Tower guarantees exactly 3 crowns
                    } else {
                        match_state.blue_crowns = (match_state.blue_crowns + crowns).min(3);
                    }
                } else if crowns == 3 {
                    match_state.red_crowns = 3; // King Tower guarantees exactly 3 crowns
                } else {
                    match_state.red_crowns = (match_state.red_crowns + crowns).min(3);
                }

                println!(
                    "⚖️ TIEBREAKER! Destroyed {:?} tower with {} HP! Score: {}-{}",
                    team, hp, match_state.blue_crowns, match_state.red_crowns
                );
            } else {
                println!("⚖️ TIEBREAKER: No towers remain — it's a DRAW!");
            }

            match_state.phase = MatchPhase::GameOver;
            println!("🛑 MATCH OVER!");
        }
    }

    // Elixir Generation
    let multiplier = match match_state.phase {
        MatchPhase::Regular => 1.0,
        _ => 2.0, // DoubleElixir and Overtime are both 2x
    };

    let elixir_gain = (1.0 / 2.8) * multiplier * delta;

    match_state.blue_elixir = (match_state.blue_elixir + elixir_gain).min(10.0);
    match_state.red_elixir = (match_state.red_elixir + elixir_gain).min(10.0);
}
