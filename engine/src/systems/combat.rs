#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use rust_royale_core::components::{
    AttackStats, AttackTimer, DeployTimer, Health, MatchPhase, MatchState, PhysicalBody, Position,
    Target, TargetingProfile, Team, TowerFootprint, TowerType, WaypointPath,
};

pub fn targeting_system(
    match_state: Res<MatchState>,
    mut attackers: Query<
        (
            Entity,
            &Position,
            &Team,
            &AttackStats,
            &TargetingProfile,
            &mut Target,
            &mut AttackTimer,
            &mut WaypointPath,
        ),
        Without<DeployTimer>,
    >,
    defenders: Query<(Entity, &Position, &Team, &TargetingProfile), With<Health>>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return; // No target scanning after the game ends!
    }
    for (
        attacker_ent,
        attacker_pos,
        attacker_team,
        attack_stats,
        attacker_profile,
        mut target,
        mut attack_timer,
        mut path,
    ) in attackers.iter_mut()
    {
        let sight_range = if attacker_profile.preference
            == rust_royale_core::stats::TargetPreference::Buildings
        {
            999.0 // Giants always see their targets
        } else {
            5.5 // Standard troops get distracted within 5.5 tiles
        };

        // --- THE DISTRACTION FIX ---
        // If we already have a target, check if it's an active fight or just a distant map-march!
        if let Some(current_target_ent) = target.0 {
            if let Ok((_, defender_pos, _, _)) = defenders.get(current_target_ent) {
                let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
                let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
                let dist = (dx * dx + dy * dy).sqrt();

                // If the target is within our 5.5 aggro radius, we are actively fighting!
                // Skip the scan so we don't get distracted mid-swing.
                if dist <= sight_range {
                    continue;
                }
            } else {
                target.0 = None; // Target died, clear it!
            }
        }

        let mut closest_enemy = None;
        let mut closest_dist = f32::MAX;

        let mut closest_building = None;
        let mut closest_building_dist = f32::MAX;

        for (defender_ent, defender_pos, defender_team, defender_profile) in defenders.iter() {
            if attacker_team != defender_team {
                if defender_profile.is_flying && !attacker_profile.targets_air {
                    continue;
                }
                if !defender_profile.is_flying && !attacker_profile.targets_ground {
                    continue;
                }
                if attacker_profile.preference
                    == rust_royale_core::stats::TargetPreference::Buildings
                    && !defender_profile.is_building
                {
                    continue;
                }

                let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
                let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
                let mut dist = (dx * dx + dy * dy).sqrt();

                // --- LANE BIAS FIX ---
                // In Clash Royale, left lane troops strongly prefer left lane targets.
                // However, we only apply this when they are far away. Once they are deep in
                // opponent territory (dist < 10.0), they just go for the closest thing.
                let attacker_lane_left = (attacker_pos.x as f32 / 1000.0) < 9.0;
                let defender_lane_left = (defender_pos.x as f32 / 1000.0) < 9.0;

                if defender_profile.is_building
                    && dist > 10.0
                    && attacker_lane_left != defender_lane_left
                {
                    dist += 20.0; // Stable lane commitment
                }

                if dist < closest_dist {
                    closest_dist = dist;
                    closest_enemy = Some(defender_ent);
                }

                if defender_profile.is_building && dist < closest_building_dist {
                    closest_building_dist = dist;
                    closest_building = Some(defender_ent);
                }
            }
        }

        // --- CR LANE LOGIC ---
        let (final_target, final_dist) = if closest_dist <= sight_range {
            (closest_enemy, closest_dist)
        } else {
            (closest_building, closest_building_dist)
        };

        if let Some(enemy_ent) = final_target {
            // ONLY OVERWRITE THE TARGET IF IT CHANGED!
            if target.0 != Some(enemy_ent) {
                target.0 = Some(enemy_ent);
                path.0.clear(); // Recalculate A*

                if final_dist <= attack_stats.range {
                    attack_timer
                        .0
                        .set_duration(std::time::Duration::from_secs_f32(
                            attack_stats.first_attack_sec,
                        ));
                    attack_timer.0.reset();
                }

                println!(
                    "Entity {:?} was Distracted/Locked onto {:?}",
                    attacker_ent, enemy_ent
                );
            }
        }
    }
}

pub fn combat_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut match_state: ResMut<MatchState>,
    mut grid: ResMut<rust_royale_core::arena::ArenaGrid>,
    mut attackers: Query<(
        Entity,
        &Position,
        &mut AttackTimer,
        &AttackStats,
        &mut Target,
        &mut WaypointPath,
    )>,
    mut defenders: Query<(
        &Position,
        &mut Health,
        Option<&PhysicalBody>,
        Option<&TowerType>,
        Option<&TowerFootprint>,
        &Team,
    )>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return; // No combat after the game ends!
    }
    for (attacker_ent, attacker_pos, mut timer, stats, mut target, mut path) in attackers.iter_mut()
    {
        let target_entity = match target.0 {
            Some(ent) => ent,
            None => continue,
        };

        // --- INSTANT GHOST TARGET CHECK ---
        if defenders.get(target_entity).is_err() {
            target.0 = None;
            path.0.clear(); // Target is dead, clear the path so we can recalculate!
            continue;
        }

        // --- RANGE CHECK: Only tick the attack clock if we're in striking distance ---
        // If out of range, DON'T drop the target — the movement system will chase.
        // We just skip damage this frame so the timer doesn't tick while we're running.
        if let Ok((defender_pos, _, defender_body, _, _, _)) = defenders.get(target_entity) {
            let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
            let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
            let center_dist = (dx * dx + dy * dy).sqrt();
            // Subtract the target's radius to measure to the EDGE, not center!
            let target_radius = defender_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
            let dist = center_dist - target_radius;

            if dist > stats.range {
                continue; // Out of range — don't attack, but keep the lock!
            }
        }

        // Tick the attack animation clock
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            // --- THE COOLDOWN RESET ---
            timer.0.set_duration(std::time::Duration::from_secs_f32(
                stats.hit_speed_ms as f32 / 1000.0,
            ));

            if let Ok((_, mut defender_health, _, tower_type, tower_footprint, defender_team)) =
                defenders.get_mut(target_entity)
            {
                defender_health.0 -= stats.damage;
                println!(
                    "Entity {:?} hit {:?} for {} damage! (Target HP: {})",
                    attacker_ent, target_entity, stats.damage, defender_health.0
                );

                if defender_health.0 <= 0 {
                    println!("Entity {:?} was SLAIN!", target_entity);
                    commands.entity(target_entity).despawn();
                    target.0 = None;
                    path.0.clear(); // BUG FIX: Clear stale waypoints so the troop recalculates!

                    // --- TOWER TILE CLEANUP ---
                    if let Some(footprint) = tower_footprint {
                        grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);
                    }

                    // --- CROWN LOGIC ---
                    if let Some(tower) = tower_type {
                        if *defender_team == Team::Red {
                            if matches!(tower, TowerType::King) {
                                match_state.blue_crowns = 3; // King Tower instantly sets score to 3
                            } else {
                                match_state.blue_crowns = (match_state.blue_crowns + 1).min(3);
                            }
                        } else if matches!(tower, TowerType::King) {
                            match_state.red_crowns = 3; // King Tower instantly sets score to 3
                        } else {
                            match_state.red_crowns = (match_state.red_crowns + 1).min(3);
                        }

                        println!(
                            "👑 TOWER DOWN! Score: {}-{}",
                            match_state.blue_crowns, match_state.red_crowns
                        );

                        // Sudden Death Check or King Tower Kill
                        if matches!(tower, TowerType::King)
                            || match_state.phase == MatchPhase::Overtime
                        {
                            match_state.phase = MatchPhase::GameOver;
                            let winner = if *defender_team == Team::Red {
                                "BLUE"
                            } else {
                                "RED"
                            };
                            println!(
                                "🛑 MATCH OVER BY KNOCKOUT! {} TEAM WINS! {}-{}",
                                winner, match_state.blue_crowns, match_state.red_crowns
                            );
                        }
                    }
                }
            }
        }
    }
}
