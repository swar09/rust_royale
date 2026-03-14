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
            // --- TIEBREAKER: Instant comparison for lowest HP ---
            match_state.clock_seconds = 0.0;

            let mut min_hp = i32::MAX;
            let mut max_hp = i32::MIN;

            for (_, health, _, _, _) in towers.iter() {
                if health.0 < min_hp {
                    min_hp = health.0;
                }
                if health.0 > max_hp {
                    max_hp = health.0;
                }
            }

            if min_hp == max_hp || min_hp == i32::MAX {
                // If min_hp == max_hp, ALL towers have exactly the same health.
                println!(
                    "⚖️ TIEBREAKER: All towers have completely equal health ({} HP) — it's a DRAW!",
                    min_hp
                );
            } else {
                // Find and instantly destroy the tower(s) with the minimum HP
                let mut king_destroyed_team = None;

                for (entity, health, team, tower_type, footprint) in towers.iter() {
                    if health.0 == min_hp {
                        commands.entity(entity).despawn();
                        grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);

                        let crowns = match tower_type {
                            TowerType::Princess => 1,
                            TowerType::King => 3,
                        };

                        if *team == Team::Red {
                            if crowns == 3 {
                                match_state.blue_crowns = 3;
                                king_destroyed_team = Some(*team);
                            } else {
                                match_state.blue_crowns = (match_state.blue_crowns + crowns).min(3);
                            }
                        } else {
                            if crowns == 3 {
                                match_state.red_crowns = 3;
                                king_destroyed_team = Some(*team);
                            } else {
                                match_state.red_crowns = (match_state.red_crowns + crowns).min(3);
                            }
                        }

                        println!(
                            "💥 TIEBREAKER! {:?} tower with lowest HP ({}) destroyed! Score: {}-{}",
                            team, min_hp, match_state.blue_crowns, match_state.red_crowns
                        );
                    }
                }

                // Also clean up any remaining princess towers if their king was the lowest
                if let Some(losing_team) = king_destroyed_team {
                    for (entity, _, team, tower_type, footprint) in towers.iter() {
                        if *team == losing_team && matches!(tower_type, TowerType::Princess) {
                            commands.entity(entity).despawn();
                            grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);
                        }
                    }
                }
            }

            match_state.phase = MatchPhase::GameOver;
            println!(
                "🛑 MATCH OVER! Final Score: {}-{}",
                match_state.blue_crowns, match_state.red_crowns
            );
        }
    }

    // Elixir Generation
    let multiplier = match match_state.phase {
        MatchPhase::Regular => 1.0,
        MatchPhase::GameOver => 0.0,
        _ => 2.0, // DoubleElixir and Overtime are both 2x
    };

    let elixir_gain = (1.0 / 2.8) * multiplier * delta;

    match_state.blue_elixir = (match_state.blue_elixir + elixir_gain).min(10.0);
    match_state.red_elixir = (match_state.red_elixir + elixir_gain).min(10.0);
}
