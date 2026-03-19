#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use rust_royale_core::components::{
    AoEPayload, AttackStats, AttackTimer, DeathSpawn, DeathSpawnEvent, DeployTimer, Health,
    MatchPhase, MatchState, PhysicalBody, Position, Projectile, SpawnLane, SpellStrike, Target,
    TargetingProfile, Team, TowerFootprint, TowerStatus, TowerType, WaypointPath,
};
use rust_royale_core::constants::TILE_SIZE;

/// Returns true if a troop in the given lane is allowed to target a building
/// at this fixed-point x position.
///
/// Centre band (tiles 7-13, fixed 7000-13000) is valid for both lanes —
/// it covers the king tower and bridge approach tiles.
/// Outside the centre band, only same-side buildings are valid.
/// Non-buildings always pass (troops are distractions, not lane-locked).
fn lane_allows_target(
    spawn_lane: Option<&SpawnLane>,
    target_fixed_x: i32,
    target_is_building: bool,
) -> bool {
    if !target_is_building {
        return true;
    }
    let lane = match spawn_lane {
        Some(l) => l,
        None => return true, // towers have no SpawnLane → always allow
    };
    if target_fixed_x >= 7_000 && target_fixed_x <= 13_000 {
        return true; // king tower centre band — both lanes allowed
    }
    match lane {
        SpawnLane::Left => target_fixed_x < 10_000,
        SpawnLane::Right => target_fixed_x >= 10_000,
    }
}

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
            Option<&SpawnLane>,
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
        return;
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
        tower_status,
        attacker_body,
        spawn_lane,
    ) in attackers.iter_mut()
    {
        if let Some(status) = tower_status {
            if *status == TowerStatus::Sleeping {
                continue;
            }
        }

        let sight_range: f32 = if attacker_profile.is_building {
            attack_stats.range
        } else if attacker_profile.preference
            == rust_royale_core::stats::TargetPreference::Buildings
        {
            999.0
        } else {
            attack_stats.range.max(5.5)
        };

        // ----------------------------------------------------------------
        // Stickiness — validate BOTH range AND lane before keeping target
        // ----------------------------------------------------------------
        let target_before_stickiness = target.0; // remember for re-acquire detection below

        if let Some(cur_ent) = target.0 {
            if let Ok((_, def_pos, _, def_profile, def_body)) = defenders.get(cur_ent) {
                let dx = (attacker_pos.x - def_pos.x) as f32 / 1000.0;
                let dy = (attacker_pos.y - def_pos.y) as f32 / 1000.0;
                let center_dist = (dx * dx + dy * dy).sqrt();
                let ar = attacker_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
                let tr = def_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
                let dist = center_dist - ar - tr;

                let lane_ok = lane_allows_target(spawn_lane, def_pos.x, def_profile.is_building);

                if dist <= sight_range && lane_ok {
                    continue; // target is still valid — keep it
                }

                // Out of range or wrong lane — clear target but NOT the path.
                target.0 = None;
            } else {
                // Target entity was despawned — clear both
                target.0 = None;
                if let Some(ref mut p) = path {
                    p.0.clear();
                }
            }
        }

        // ----------------------------------------------------------------
        // Scan for a new target
        // ----------------------------------------------------------------
        let mut best_troop: Option<Entity> = None;
        let mut best_troop_dist = f32::MAX;
        let mut best_building: Option<Entity> = None;
        let mut best_building_dist = f32::MAX;

        for (def_ent, def_pos, def_team, def_profile, def_body) in defenders.iter() {
            if attacker_team == def_team {
                continue;
            }
            if def_profile.is_flying && !attacker_profile.targets_air {
                continue;
            }
            if !def_profile.is_flying && !attacker_profile.targets_ground {
                continue;
            }
            if attacker_profile.preference == rust_royale_core::stats::TargetPreference::Buildings
                && !def_profile.is_building
            {
                continue;
            }
            // Lane filter — the core fix
            if !lane_allows_target(spawn_lane, def_pos.x, def_profile.is_building) {
                continue;
            }

            let dx = (attacker_pos.x - def_pos.x) as f32 / 1000.0;
            let dy = (attacker_pos.y - def_pos.y) as f32 / 1000.0;
            let center_dist = (dx * dx + dy * dy).sqrt();
            let ar = attacker_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
            let tr = def_body.map_or(0.0, |b| b.radius as f32 / 1000.0);
            let dist = center_dist - ar - tr;

            if def_profile.is_building {
                if dist < best_building_dist {
                    best_building_dist = dist;
                    best_building = Some(def_ent);
                }
            } else if dist < best_troop_dist {
                best_troop_dist = dist;
                best_troop = Some(def_ent);
            }
        }

        // Nearby troop wins over distant building; otherwise march to building
        let (final_target, final_dist) = if best_troop_dist <= sight_range {
            (best_troop, best_troop_dist)
        } else {
            (best_building, best_building_dist)
        };

        if let Some(enemy_ent) = final_target {
            if target.0 != Some(enemy_ent) {
                // Only clear path when switching to a genuinely different entity.
                // If we dropped the target above (range check) and the scan re-acquires
                // the exact same entity, target_before_stickiness == Some(enemy_ent),
                // so we keep the existing path and avoid the thrash freeze.
                let is_new_entity = target_before_stickiness != Some(enemy_ent);
                target.0 = Some(enemy_ent);
                if is_new_entity {
                    if let Some(mut p) = path {
                        p.0.clear();
                    }
                }
                if final_dist <= attack_stats.range {
                    attack_timer
                        .0
                        .set_duration(std::time::Duration::from_secs_f32(
                            attack_stats.first_attack_sec,
                        ));
                    attack_timer.0.reset();
                }
                if spawn_lane.is_some() && is_new_entity {
                    println!(
                        "Entity {:?} (lane {:?}) -> {:?}",
                        attacker_ent, spawn_lane, enemy_ent
                    );
                }
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
        return;
    }
    for (_attacker_ent, attacker_pos, mut timer, stats, mut target, path, attacker_body) in
        attackers.iter_mut()
    {
        let target_entity = match target.0 {
            Some(ent) => ent,
            None => continue,
        };

        if defenders.get(target_entity).is_err() {
            target.0 = None;
            if let Some(mut p) = path {
                p.0.clear();
            }
            continue;
        }

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
                commands.spawn((
                    Position {
                        x: attacker_pos.x,
                        y: attacker_pos.y,
                    },
                    Projectile {
                        damage: stats.damage,
                        speed: 6000,
                    },
                    Target(Some(target_ent)),
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::YELLOW,
                            custom_size: Some(Vec2::splat(TILE_SIZE * 0.2)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 2.0),
                        ..default()
                    },
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
    mut other_troops: Query<&mut Target, Without<Projectile>>,
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
                    let mut king_destroyed_team = None;
                    let mut wake_king_team = None;

                    health.0 -= proj_stats.damage;

                    if let Some(ref mut status) = tower_status {
                        if **status == TowerStatus::Sleeping {
                            **status = TowerStatus::Active;
                            println!("👑 {:?} King Tower AWAKENED (direct damage)!", target_team);
                        }
                    }
                    println!("💥 Hit! -{} dmg. HP={}", proj_stats.damage, health.0);

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
                        commands.entity(target_ent_inner).despawn();
                        for mut ot in other_troops.iter_mut() {
                            if ot.0 == Some(target_ent_inner) {
                                ot.0 = None;
                            }
                        }
                        if let Some(fp) = tower_footprint {
                            grid.clear_tower(fp.start_x, fp.start_y, fp.size);
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
                                "👑 TOWER DOWN! {}-{}",
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
                                    "🛑 {} WINS! {}-{}",
                                    winner, match_state.blue_crowns, match_state.red_crowns
                                );
                            }
                        }
                    }

                    if let Some(losing_team) = king_destroyed_team {
                        let mut to_destroy = Vec::new();
                        for (ent, _, tt, fp, team, _, _, _) in targets.iter() {
                            if *team == losing_team && matches!(tt, Some(TowerType::Princess)) {
                                to_destroy.push((
                                    ent,
                                    TowerFootprint {
                                        start_x: fp.unwrap().start_x,
                                        start_y: fp.unwrap().start_y,
                                        size: fp.unwrap().size,
                                    },
                                ));
                            }
                        }
                        for (ent, fp) in to_destroy {
                            commands.entity(ent).despawn();
                            grid.clear_tower(fp.start_x, fp.start_y, fp.size);
                        }
                    }

                    if let Some(team_to_wake) = wake_king_team {
                        for (_, _, tt, _, t_team, mut opt_s, _, _) in targets.iter_mut() {
                            if *t_team == team_to_wake && matches!(tt, Some(TowerType::King)) {
                                if let Some(ref mut s) = opt_s {
                                    if **s == TowerStatus::Sleeping {
                                        **s = TowerStatus::Active;
                                        println!("👑 {:?} King AWAKENED — princess fell!", t_team);
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
                        proj_pos.x += (dx as f32 / dist * move_dist) as i32;
                        proj_pos.y += (dy as f32 / dist * move_dist) as i32;
                    }
                }
            } else {
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
    mut targets: Query<
        (
            Entity,
            &mut Position,
            &mut Health,
            Option<&TowerType>,
            Option<&TowerFootprint>,
            &Team,
            Option<&mut TowerStatus>,
            Option<&DeathSpawn>,
            Option<&PhysicalBody>,
        ),
        Without<SpellStrike>,
    >,
    mut match_state: ResMut<MatchState>,
    mut grid: ResMut<rust_royale_core::arena::ArenaGrid>,
    mut other_troops: Query<&mut Target, Without<SpellStrike>>,
    mut death_events: EventWriter<DeathSpawnEvent>,
) {
    for (spell_ent, spell_pos, mut payload, spell_team, mut timer) in spells.iter_mut() {
        timer.0.tick(time.delta());
        if !timer.0.just_finished() {
            continue;
        }

        println!(
            "💥 SPELL wave {}/{} at ({},{})",
            payload.waves_total - payload.waves_remaining + 1,
            payload.waves_total,
            spell_pos.x,
            spell_pos.y
        );

        let mut king_destroyed_team = None;
        let mut wake_king_team = None;

        for (
            target_ent,
            mut target_pos,
            mut health,
            tower_type,
            tower_footprint,
            target_team,
            mut opt_status,
            death_spawn,
            physical_body,
        ) in targets.iter_mut()
        {
            if spell_team == target_team {
                continue;
            }

            let dx = target_pos.x - spell_pos.x;
            let dy = target_pos.y - spell_pos.y;
            let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
            if dist > payload.radius as f32 {
                continue;
            }

            let dmg = if tower_type.is_some() {
                payload.tower_damage
            } else {
                payload.damage
            };
            health.0 -= dmg;

            if payload.knockback > 0 && tower_type.is_none() && dist > 0.0 {
                let mass = physical_body.map_or(1, |b| b.mass).max(1);
                let force = payload.knockback as f32 / mass as f32;
                target_pos.x += (dx as f32 / dist * force) as i32;
                target_pos.y += (dy as f32 / dist * force) as i32;
            }

            if let Some(ref mut s) = opt_status {
                if **s == TowerStatus::Sleeping {
                    **s = TowerStatus::Active;
                }
            }

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
                for mut ot in other_troops.iter_mut() {
                    if ot.0 == Some(target_ent) {
                        ot.0 = None;
                    }
                }
                if let Some(fp) = tower_footprint {
                    grid.clear_tower(fp.start_x, fp.start_y, fp.size);
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
                    if matches!(tower, TowerType::King) || match_state.phase == MatchPhase::Overtime
                    {
                        match_state.phase = MatchPhase::GameOver;
                    }
                }
            }
        }

        if let Some(losing_team) = king_destroyed_team {
            let mut to_destroy = Vec::new();
            for (ent, _, _, tt, fp, team, _, _, _) in targets.iter() {
                if *team == losing_team && matches!(tt, Some(TowerType::Princess)) {
                    to_destroy.push((
                        ent,
                        TowerFootprint {
                            start_x: fp.unwrap().start_x,
                            start_y: fp.unwrap().start_y,
                            size: fp.unwrap().size,
                        },
                    ));
                }
            }
            for (ent, fp) in to_destroy {
                commands.entity(ent).despawn();
                grid.clear_tower(fp.start_x, fp.start_y, fp.size);
            }
        }

        if let Some(team_to_wake) = wake_king_team {
            for (_, _, _, tt, _, t_team, mut opt_s, _, _) in targets.iter_mut() {
                if *t_team == team_to_wake && matches!(tt, Some(TowerType::King)) {
                    if let Some(ref mut s) = opt_s {
                        if **s == TowerStatus::Sleeping {
                            **s = TowerStatus::Active;
                        }
                    }
                }
            }
        }

        payload.waves_remaining -= 1;
        if payload.waves_remaining > 0 {
            timer
                .0
                .set_duration(std::time::Duration::from_secs_f32(0.1));
            timer.0.reset();
        } else {
            commands.entity(spell_ent).despawn();
        }
    }
}
