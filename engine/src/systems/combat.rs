#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use rust_royale_core::components::{
    AoEPayload, AttackStats, AttackTimer, DeathSpawn, DeathSpawnEvent, DeployTimer, Health,
    MatchPhase, MatchState, PhysicalBody, Position, Projectile, SpellStrike, Target,
    TargetingProfile, Team, TowerFootprint, TowerStatus, TowerType, WaypointPath,
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
            Option<&mut WaypointPath>,
            Option<&TowerStatus>,
            Option<&PhysicalBody>,
        ),
        Without<DeployTimer>,
    >,
    defenders: Query<
        (
            Entity,
            &Position,
            &Team,
            &TargetingProfile,
            Option<&PhysicalBody>,
        ),
        With<Health>,
    >,
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
        path,
        tower_status,
        attacker_body,
    ) in attackers.iter_mut()
    {
        // --- 3. THE SLUMBER CHECK ---
        if let Some(status) = tower_status {
            if *status == TowerStatus::Sleeping {
                continue; // King is asleep! Skip scanning for targets.
            }
        }
        // --- 3. SIGHT RANGE CALCULATION ---
        let mut sight_range: f32 = if attacker_profile.preference
            == rust_royale_core::stats::TargetPreference::Buildings
        {
            999.0 // Giants always see their targets
        } else {
            5.5 // Standard troops get distracted within 5.5 tiles
        };

        // Towers should be able to see as far as they can shoot!
        if attacker_profile.is_building {
            sight_range = sight_range.max(attack_stats.range);
        }

        // --- THE DISTRACTION FIX ---
        if let Some(current_target_ent) = target.0 {
            if let Ok((_, defender_pos, _, _, defender_body)) = defenders.get(current_target_ent) {
                let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
                let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
                let center_dist = (dx * dx + dy * dy).sqrt();
                let attacker_radius = attacker_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
                let target_radius = defender_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
                let dist = center_dist - attacker_radius - target_radius;

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

        for (defender_ent, defender_pos, defender_team, defender_profile, defender_body) in
            defenders.iter()
        {
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
                let center_dist = (dx * dx + dy * dy).sqrt();
                let attacker_radius = attacker_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
                let target_radius = defender_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
                let mut dist = center_dist - attacker_radius - target_radius;

                // --- LANE BIAS FIX ---
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

        let (final_target, final_dist) = if closest_dist <= sight_range {
            (closest_enemy, closest_dist)
        } else if !attacker_profile.is_building
            || attacker_profile.preference == rust_royale_core::stats::TargetPreference::Buildings
        {
            (closest_building, closest_building_dist)
        } else {
            (None, f32::MAX)
        };

        if let Some(enemy_ent) = final_target {
            // ONLY OVERWRITE THE TARGET IF IT CHANGED!
            if target.0 != Some(enemy_ent) {
                target.0 = Some(enemy_ent);

                if let Some(mut p) = path {
                    p.0.clear(); // Recalculate A*
                }

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
    match_state: Res<MatchState>,
    mut attackers: Query<(
        Entity,
        &Position,
        &mut AttackTimer,
        &AttackStats,
        &mut Target,
        Option<&mut WaypointPath>,
        Option<&PhysicalBody>,
    )>,
    defenders: Query<(Entity, &Position, Option<&PhysicalBody>)>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return; // No combat after the game ends!
    }
    for (_attacker_ent, attacker_pos, mut timer, stats, mut target, path, attacker_body) in
        attackers.iter_mut()
    {
        let target_entity = match target.0 {
            Some(ent) => ent,
            None => continue,
        };

        // --- INSTANT GHOST TARGET CHECK ---
        if defenders.get(target_entity).is_err() {
            target.0 = None;
            if let Some(mut p) = path {
                p.0.clear(); // Target is dead, clear the path so we can recalculate!
            }
            continue;
        }

        // --- RANGE CHECK ---
        if let Ok((_, defender_pos, defender_body)) = defenders.get(target_entity) {
            let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
            let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
            let center_dist = (dx * dx + dy * dy).sqrt();
            let attacker_radius = attacker_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
            let target_radius = defender_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
            let dist = center_dist - attacker_radius - target_radius;

            if dist > stats.range {
                continue;
            }
        }

        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            timer.0.set_duration(std::time::Duration::from_secs_f32(
                stats.hit_speed_ms as f32 / 1000.0,
            ));

            if let Some(target_ent) = target.0 {
                let projectile_speed = 6000; // 6 tiles per second

                commands.spawn((
                    Position {
                        x: attacker_pos.x,
                        y: attacker_pos.y,
                    },
                    Projectile {
                        damage: stats.damage,
                        speed: projectile_speed,
                    },
                    Target(Some(target_ent)),
                ));

                println!("Fired a projectile for {} damage!", stats.damage);
            }
        }
    }
}

pub fn projectile_flight_system(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Position, &Projectile, &Target)>,
    mut targets: Query<
        (
            Entity,
            &mut Health,
            Option<&TowerType>,
            Option<&TowerFootprint>,
            &Team,
            Option<&mut TowerStatus>,
            Option<&DeathSpawn>,
            &Position,
        ),
        (With<Health>, Without<Projectile>),
    >,
    mut match_state: ResMut<MatchState>,
    mut grid: ResMut<rust_royale_core::arena::ArenaGrid>,
    mut other_troops: Query<&mut Target, Without<Projectile>>, // For clearing targets if a unit died
    mut death_events: EventWriter<DeathSpawnEvent>,
) {
    let delta = time.delta_seconds();

    for (proj_entity, mut proj_pos, proj_stats, target) in projectiles.iter_mut() {
        if let Some(target_ent) = target.0 {
            if let Ok((
                target_ent_inner,
                mut health,
                tower_type,
                tower_footprint,
                target_team,
                mut tower_status,
                death_spawn,
                target_pos,
            )) = targets.get_mut(target_ent)
            {
                let dx = target_pos.x - proj_pos.x;
                let dy = target_pos.y - proj_pos.y;
                let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();

                if dist < 400.0 {
                    // It hit! Apply the damage.
                    let mut king_destroyed_team = None;
                    let mut wake_king_team = None;

                    health.0 -= proj_stats.damage;

                    // --- ALARM CLOCK 1: DIRECT DAMAGE ---
                    if let Some(ref mut status) = tower_status {
                        if **status == TowerStatus::Sleeping {
                            **status = TowerStatus::Active;
                            println!(
                                "👑 The {:?} King Tower has AWAKENED from direct damage!",
                                target_team
                            );
                        }
                    }

                    println!(
                        "💥 Projectile hit! Dealt {} damage. Target HP: {}",
                        proj_stats.damage, health.0
                    );

                    if health.0 <= 0 {
                        // --- DEATH SPAWN ---
                        if let Some(ds) = death_spawn {
                            death_events.send(DeathSpawnEvent {
                                card_key: ds.card_key.clone(),
                                count: ds.count,
                                team: *target_team,
                                fixed_x: target_pos.x,
                                fixed_y: target_pos.y,
                            });
                        }

                        println!("Entity {:?} was SLAIN by Projectile!", target_ent_inner);
                        commands.entity(target_ent_inner).despawn();

                        // Clear paths/targets for anyone else who was aiming here
                        for mut other_target in other_troops.iter_mut() {
                            if other_target.0 == Some(target_ent_inner) {
                                other_target.0 = None;
                            }
                        }

                        if let Some(footprint) = tower_footprint {
                            grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);
                        }

                        if let Some(tower) = tower_type {
                            if matches!(tower, TowerType::King) {
                                king_destroyed_team = Some(*target_team);
                            } else if matches!(tower, TowerType::Princess) {
                                wake_king_team = Some(*target_team);
                            }

                            if *target_team == Team::Red {
                                if matches!(tower, TowerType::King) {
                                    match_state.blue_crowns = 3;
                                } else {
                                    match_state.blue_crowns = (match_state.blue_crowns + 1).min(3);
                                }
                            } else if matches!(tower, TowerType::King) {
                                match_state.red_crowns = 3;
                            } else {
                                match_state.red_crowns = (match_state.red_crowns + 1).min(3);
                            }

                            println!(
                                "👑 TOWER DOWN! Score: {}-{}",
                                match_state.blue_crowns, match_state.red_crowns
                            );

                            if matches!(tower, TowerType::King)
                                || match_state.phase == MatchPhase::Overtime
                            {
                                match_state.phase = MatchPhase::GameOver;
                                let winner = if *target_team == Team::Red {
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

                    // AUTO-DESTROY PRINCESS TOWERS IF KING FELL
                    if let Some(losing_team) = king_destroyed_team {
                        let mut princess_towers_to_destroy = Vec::new();
                        for (ent, _, tower_type, footprint, team, _, _, _) in targets.iter() {
                            if *team == losing_team
                                && matches!(tower_type, Some(TowerType::Princess))
                            {
                                princess_towers_to_destroy.push((
                                    ent,
                                    TowerFootprint {
                                        start_x: footprint.unwrap().start_x,
                                        start_y: footprint.unwrap().start_y,
                                        size: footprint.unwrap().size,
                                    },
                                ));
                            }
                        }

                        for (ent, footprint) in princess_towers_to_destroy {
                            commands.entity(ent).despawn();
                            grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);
                            println!("💥 King fell! Princess tower automatically destroyed.");
                        }
                    }

                    // --- EXECUTE ALARM CLOCK 2 ---
                    if let Some(team_to_wake) = wake_king_team {
                        for (_, _, t_type, _, t_team, mut opt_status, _, _) in targets.iter_mut() {
                            if *t_team == team_to_wake && matches!(t_type, Some(TowerType::King)) {
                                if let Some(ref mut status) = opt_status {
                                    if **status == TowerStatus::Sleeping {
                                        **status = TowerStatus::Active;
                                        println!("👑 The {:?} King Tower has AWAKENED because a Princess fell!", t_team);
                                    }
                                }
                            }
                        }
                    }

                    commands.entity(proj_entity).despawn();
                } else {
                    let move_dist = proj_stats.speed as f32 * delta;
                    if move_dist >= dist {
                        proj_pos.x = target_pos.x;
                        proj_pos.y = target_pos.y;
                    } else {
                        let dir_x = dx as f32 / dist;
                        let dir_y = dy as f32 / dist;
                        proj_pos.x += (dir_x * move_dist) as i32;
                        proj_pos.y += (dir_y * move_dist) as i32;
                    }
                }
            } else {
                println!("💨 Projectile missed! Target died mid-flight.");
                commands.entity(proj_entity).despawn();
            }
        } else {
            commands.entity(proj_entity).despawn();
        }
    }
}

pub fn spell_impact_system(
    mut commands: Commands,
    time: Res<Time>,
    mut spells: Query<
        (Entity, &Position, &mut AoEPayload, &Team, &mut DeployTimer),
        With<SpellStrike>,
    >,
    mut targets: Query<(
        Entity,
        &Position,
        &mut Health,
        Option<&TowerType>,
        Option<&TowerFootprint>,
        &Team,
        Option<&mut TowerStatus>,
        Option<&DeathSpawn>,
    )>,
    mut match_state: ResMut<MatchState>,
    mut grid: ResMut<rust_royale_core::arena::ArenaGrid>,
    mut other_troops: Query<&mut Target, Without<SpellStrike>>,
    mut death_events: EventWriter<DeathSpawnEvent>,
) {
    for (spell_ent, spell_pos, mut payload, spell_team, mut timer) in spells.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            println!(
                "💥 SPELL DETONATED (Wave {}/{}) at {}, {}!",
                payload.waves_total - payload.waves_remaining + 1,
                payload.waves_total,
                spell_pos.x,
                spell_pos.y
            );

            let mut king_destroyed_team = None;
            let mut wake_king_team = None;

            // Loop through literally everything on the board
            for (
                target_ent,
                target_pos,
                mut health,
                tower_type,
                tower_footprint,
                target_team,
                mut opt_status,
                death_spawn,
            ) in targets.iter_mut()
            {
                // Spells don't hurt your own troops!
                if spell_team == target_team {
                    continue;
                }

                // Check distance using the Payload's radius
                let dx = target_pos.x - spell_pos.x;
                let dy = target_pos.y - spell_pos.y;
                let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();

                if dist <= payload.radius as f32 {
                    // It's in the blast zone!
                    let damage_dealt = if tower_type.is_some() {
                        payload.tower_damage
                    } else {
                        payload.damage
                    };
                    health.0 -= damage_dealt;

                    println!(
                        "🔥 AoE hit {:?} for {} damage! HP remaining: {}",
                        target_team, damage_dealt, health.0
                    );

                    // --- ALARM CLOCK 1 (Direct Damage) ---
                    if let Some(ref mut status) = opt_status {
                        if **status == TowerStatus::Sleeping {
                            **status = TowerStatus::Active;
                            println!(
                                "👑 The {:?} King Tower was AWAKENED by a Spell!",
                                target_team
                            );
                        }
                    }

                    // --- DEATH RESOLUTION ---
                    if health.0 <= 0 {
                        if let Some(ds) = death_spawn {
                            death_events.send(DeathSpawnEvent {
                                card_key: ds.card_key.clone(),
                                count: ds.count,
                                team: *target_team,
                                fixed_x: target_pos.x,
                                fixed_y: target_pos.y,
                            });
                        }

                        commands.entity(target_ent).despawn();
                        for mut other_target in other_troops.iter_mut() {
                            if other_target.0 == Some(target_ent) {
                                other_target.0 = None;
                            }
                        }

                        if let Some(footprint) = tower_footprint {
                            grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);
                        }

                        if let Some(tower) = tower_type {
                            if matches!(tower, TowerType::King) {
                                king_destroyed_team = Some(*target_team);
                            } else if matches!(tower, TowerType::Princess) {
                                // --- ALARM CLOCK 2: PRINCESS FELL ---
                                wake_king_team = Some(*target_team);
                            }

                            if *target_team == Team::Red {
                                if matches!(tower, TowerType::King) {
                                    match_state.blue_crowns = 3;
                                } else {
                                    match_state.blue_crowns = (match_state.blue_crowns + 1).min(3);
                                }
                            } else if matches!(tower, TowerType::King) {
                                match_state.red_crowns = 3;
                            } else {
                                match_state.red_crowns = (match_state.red_crowns + 1).min(3);
                            }

                            println!(
                                "👑 TOWER DOWN! Score: {}-{}",
                                match_state.blue_crowns, match_state.red_crowns
                            );

                            if matches!(tower, TowerType::King)
                                || match_state.phase == MatchPhase::Overtime
                            {
                                match_state.phase = MatchPhase::GameOver;
                                let winner = if *target_team == Team::Red {
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

            // AUTO-DESTROY PRINCESS TOWERS IF KING FELL
            if let Some(losing_team) = king_destroyed_team {
                let mut princess_towers_to_destroy = Vec::new();
                for (ent, _, _, t_type, footprint, team, _, _) in targets.iter() {
                    if *team == losing_team && matches!(t_type, Some(TowerType::Princess)) {
                        princess_towers_to_destroy.push((
                            ent,
                            TowerFootprint {
                                start_x: footprint.unwrap().start_x,
                                start_y: footprint.unwrap().start_y,
                                size: footprint.unwrap().size,
                            },
                        ));
                    }
                }

                for (ent, footprint) in princess_towers_to_destroy {
                    commands.entity(ent).despawn();
                    grid.clear_tower(footprint.start_x, footprint.start_y, footprint.size);
                    println!("💥 King fell! Princess tower automatically destroyed.");
                }
            }

            // --- EXECUTE ALARM CLOCK 2 ---
            if let Some(team_to_wake) = wake_king_team {
                for (_, _, _, t_type, _, t_team, mut opt_status, _) in targets.iter_mut() {
                    if *t_team == team_to_wake && matches!(t_type, Some(TowerType::King)) {
                        if let Some(ref mut status) = opt_status {
                            if **status == TowerStatus::Sleeping {
                                **status = TowerStatus::Active;
                                println!(
                                    "👑 The {:?} King Tower has AWAKENED because a Princess fell!",
                                    t_team
                                );
                            }
                        }
                    }
                }
            }

            // --- MULTI-WAVE LOGIC ---
            payload.waves_remaining -= 1;
            if payload.waves_remaining > 0 {
                // Reset timer for the next wave (quick burst)
                timer
                    .0
                    .set_duration(std::time::Duration::from_secs_f32(0.1));
                timer.0.reset();
            } else {
                commands.entity(spell_ent).despawn();
            }
        }
    }
}
