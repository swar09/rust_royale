#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use rust_royale_core::arena::{ArenaGrid, TileType};
use rust_royale_core::components::{
    AttackStats, DeployTimer, MatchPhase, MatchState, PhysicalBody, Position, SpawnLane, Target,
    TargetingProfile, Team, TowerFootprint, TowerType, Velocity, WaypointPath,
};
use rust_royale_core::pathfinding::calculate_a_star;
use std::collections::HashMap;

/// Attempt A* and assign to path ONLY when the result is non-empty.
///
/// A* returns Some(vec![]) when the start tile is already within
/// attack_range_tiles of the goal (Manhattan). If we blindly assign that,
/// path stays empty every frame and the troop freezes because it never
/// enters the "walk straight" fallback.
///
/// By ignoring empty results we let the straight-line walk below handle
/// the final approach, which uses the correct Euclidean range check.
fn try_calc_path(
    path: &mut WaypointPath,
    grid: &ArenaGrid,
    start: (i32, i32),
    goal: (i32, i32),
    is_flying: bool,
    range_tiles: i32,
) {
    if let Some(new_route) = calculate_a_star(grid, start, goal, is_flying, range_tiles) {
        if !new_route.is_empty() {
            path.0 = new_route;
        }
        // If new_route is empty, A* says "you're already close enough in Manhattan
        // distance" — leave path.0 alone so the straight-line fallback fires.
    }
}

pub fn physics_movement_system(
    time: Res<Time>,
    match_state: Res<MatchState>,
    grid: Res<ArenaGrid>,
    towers: Query<(&Team, &TowerType, &TowerFootprint)>,
    mut queries: ParamSet<(
        Query<(Entity, &Position, Option<&PhysicalBody>)>,
        Query<
            (
                Entity,
                &mut Position,
                &Velocity,
                &Team,
                &Target,
                &AttackStats,
                &TargetingProfile,
                &mut WaypointPath,
                Option<&SpawnLane>,
            ),
            Without<DeployTimer>,
        >,
    )>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return;
    }

    let delta_time = time.delta_seconds();

    let mut position_snapshot: HashMap<Entity, (i32, i32, i32)> = HashMap::new();
    for (ent, pos, body) in queries.p0().iter() {
        let radius = body.map_or(0, |b| b.radius);
        position_snapshot.insert(ent, (pos.x, pos.y, radius));
    }

    // Alive tower lookup for lane goal selection
    let divider_tiles = rust_royale_core::constants::ARENA_WIDTH / 2;
    let mut red_left_alive = false;
    let mut red_right_alive = false;
    let mut blue_left_alive = false;
    let mut blue_right_alive = false;

    for (t_team, t_type, footprint) in towers.iter() {
        if matches!(t_type, TowerType::Princess) {
            let is_left = footprint.start_x < divider_tiles;
            match t_team {
                Team::Red => {
                    if is_left {
                        red_left_alive = true;
                    } else {
                        red_right_alive = true;
                    }
                }
                Team::Blue => {
                    if is_left {
                        blue_left_alive = true;
                    } else {
                        blue_right_alive = true;
                    }
                }
            }
        }
    }

    for (_ent, mut pos, velocity, team, target, attack_stats, profile, mut path, spawn_lane) in
        queries.p1().iter_mut()
    {
        let frame_movement = (velocity.0 as f32 * delta_time) as i32;
        let mut move_x = 0i32;
        let mut move_y = 0i32;
        let mut using_straight_line = false; // set true when walking directly at target

        match target.0 {
            // ---------------------------------------------------------------
            // HAS A TARGET — walk toward it (or stand still if in range)
            // ---------------------------------------------------------------
            Some(target_ent) => {
                let snapshot = match position_snapshot.get(&target_ent) {
                    Some(s) => *s,
                    None => continue,
                };
                let (tx, ty, target_radius) = snapshot;

                let dx = (tx - pos.x) as f32 / 1000.0;
                let dy = (ty - pos.y) as f32 / 1000.0;
                let center_dist = (dx * dx + dy * dy).sqrt();
                let dist = center_dist - (target_radius as f32 / 1000.0);

                if dist <= attack_stats.range {
                    continue; // in range — stand still and fight
                }

                // Need to walk closer. Recalculate path if empty.
                if path.0.is_empty() {
                    let start_grid = (pos.x / 1000, pos.y / 1000);
                    let target_grid_raw = (tx / 1000, ty / 1000);
                    let target_radius_tiles = (target_radius as f32 / 1000.0) as i32;
                    let total_range = (attack_stats.range as i32) + target_radius_tiles;

                    // If the target is a centre-band building (king tower, fixed_x ~10000),
                    // A* toward its centre pulls left-lane troops rightward and vice versa.
                    // Override with the lane-appropriate approach tile so the path stays
                    // on the correct side of the arena.
                    let on_left = match spawn_lane {
                        Some(SpawnLane::Left) => true,
                        Some(SpawnLane::Right) => false,
                        None => pos.x < 10_000,
                    };
                    let target_in_centre = tx >= 7_000 && tx <= 13_000;
                    let target_grid = if target_in_centre {
                        // Approach the king from the correct lane side.
                        // Red king y=27..=30 → approach y=26
                        // Blue king y=1..=4  → approach y=5
                        let approach_x = if on_left { 8 } else { 11 };
                        let approach_y = if target_grid_raw.1 > 15 { 26 } else { 5 };
                        (approach_x, approach_y)
                    } else {
                        target_grid_raw
                    };

                    let a_star_range = if target_in_centre {
                        0
                    } else {
                        total_range.max(1)
                    };

                    try_calc_path(
                        &mut path,
                        &grid,
                        start_grid,
                        target_grid,
                        profile.is_flying,
                        a_star_range,
                    );
                }

                if let Some(&(wgx, wgy)) = path.0.first() {
                    // Follow the next GPS waypoint
                    let twx = (wgx * 1000) + 500;
                    let twy = (wgy * 1000) + 500;
                    let wdx = (twx - pos.x) as f32;
                    let wdy = (twy - pos.y) as f32;
                    let w_dist = (wdx * wdx + wdy * wdy).sqrt();

                    if w_dist < 600.0 {
                        path.0.remove(0); // reached this waypoint, move=0 this frame
                    } else {
                        move_x = (wdx / w_dist * frame_movement as f32) as i32;
                        move_y = (wdy / w_dist * frame_movement as f32) as i32;
                    }
                } else {
                    // path is empty — A* said "already close enough" in Manhattan terms,
                    // but Euclidean dist > range.
                    // Only use straight-line if not blocked by river; otherwise force
                    // an A* recalculation so we find a path via the bridge.
                    let troop_gy = pos.y / 1000;
                    let target_gy = ty / 1000;
                    let river_between = !profile.is_flying
                        && ((troop_gy <= 14 && target_gy >= 17)
                            || (troop_gy >= 17 && target_gy <= 14));

                    if river_between {
                        // Force A* recalc next frame by making sure path is empty
                        // (it already is) — but use the correct approach tile as goal
                        // so it routes through the bridge.
                        let start_grid = (pos.x / 1000, pos.y / 1000);
                        let on_left = match spawn_lane {
                            Some(SpawnLane::Left) => true,
                            Some(SpawnLane::Right) => false,
                            None => pos.x < 10_000,
                        };
                        let target_in_centre = tx >= 7_000 && tx <= 13_000;
                        let target_grid = if target_in_centre {
                            let approach_x = if on_left { 8 } else { 11 };
                            let approach_y = if target_gy > 15 { 26 } else { 5 };
                            (approach_x, approach_y)
                        } else {
                            (tx / 1000, ty / 1000)
                        };
                        let target_radius_tiles = (target_radius as f32 / 1000.0) as i32;
                        let total_range = (attack_stats.range as i32) + target_radius_tiles;
                        let a_star_range = if target_in_centre {
                            0
                        } else {
                            total_range.max(1)
                        };
                        try_calc_path(
                            &mut path,
                            &grid,
                            start_grid,
                            target_grid,
                            profile.is_flying,
                            a_star_range,
                        );
                        // move stays 0 this frame; will follow GPS path next frame
                    } else if center_dist > 0.01 {
                        let dir_x = dx / center_dist;
                        let dir_y = dy / center_dist;
                        move_x = (dir_x * frame_movement as f32) as i32;
                        move_y = (dir_y * frame_movement as f32) as i32;
                        using_straight_line = true;
                    }
                }
            }

            // ---------------------------------------------------------------
            // NO TARGET — march toward the lane goal
            // ---------------------------------------------------------------
            None => {
                if path.0.is_empty() {
                    let start_grid = (pos.x / 1000, pos.y / 1000);

                    // Derive lane from component; fall back to current x if missing
                    let on_left = match spawn_lane {
                        Some(SpawnLane::Left) => true,
                        Some(SpawnLane::Right) => false,
                        None => pos.x < 10_000,
                    };

                    // Goal tiles must be:
                    // 1. Adjacent to the target tower (not inside it)
                    // 2. On the SAME side as the spawn lane so A* routes
                    //    through the correct bridge, not across to the other side.
                    //
                    // King tower occupies x=8..=11, y=27..=30 (Red) / y=1..=4 (Blue)
                    // Left-lane approach: x=8, one row outside the tower
                    // Right-lane approach: x=11, one row outside the tower
                    let goal_grid: (i32, i32) = match team {
                        Team::Blue => {
                            if on_left {
                                if red_left_alive {
                                    (4, 27)
                                } else {
                                    (8, 26)
                                }
                            } else {
                                if red_right_alive {
                                    (15, 27)
                                } else {
                                    (11, 26)
                                }
                            }
                        }
                        Team::Red => {
                            if on_left {
                                if blue_left_alive {
                                    (4, 4)
                                } else {
                                    (8, 5)
                                }
                            } else {
                                if blue_right_alive {
                                    (15, 4)
                                } else {
                                    (11, 5)
                                }
                            }
                        }
                    };

                    try_calc_path(
                        &mut path,
                        &grid,
                        start_grid,
                        goal_grid,
                        profile.is_flying,
                        0,
                    );
                }

                if let Some(&(wgx, wgy)) = path.0.first() {
                    let twx = (wgx * 1000) + 500;
                    let twy = (wgy * 1000) + 500;
                    let dx = (twx - pos.x) as f32;
                    let dy = (twy - pos.y) as f32;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist < 600.0 {
                        path.0.remove(0);
                    } else {
                        move_x = (dx / dist * frame_movement as f32) as i32;
                        move_y = (dy / dist * frame_movement as f32) as i32;
                    }
                }
            }
        }

        // ---------------------------------------------------------------
        // Wall / slide check — only apply move if destination is walkable
        // ---------------------------------------------------------------
        let mut final_move_x = 0i32;
        let mut final_move_y = 0i32;

        // If we still have GPS waypoints, trust the path (it was validated by A*)
        // Also bypass for straight-line approach — the destination tile may be a
        // Tower but we need to walk adjacent to it; the range check will stop us.
        let is_using_gps = !path.0.is_empty();

        if is_using_gps || using_straight_line {
            pos.x += move_x;
            pos.y += move_y;
            continue; // skip wall check — Tower tiles blocked but we approach adjacent
        }

        let new_gx = (pos.x + move_x) / 1000;
        let new_gy = (pos.y + move_y) / 1000;

        let full_step_ok = new_gx >= 0
            && new_gx < rust_royale_core::constants::ARENA_WIDTH as i32
            && new_gy >= 0
            && new_gy < rust_royale_core::constants::ARENA_HEIGHT as i32
            && {
                let t = &grid.tiles
                    [(new_gy * rust_royale_core::constants::ARENA_WIDTH as i32 + new_gx) as usize];
                match t {
                    TileType::River => profile.is_flying,
                    TileType::Tower | TileType::Wall => false,
                    _ => true,
                }
            };

        if full_step_ok {
            final_move_x = move_x;
            final_move_y = move_y;
        } else {
            // Try sliding along X axis
            let nx = pos.x + move_x;
            let ngx = nx / 1000;
            let gy = pos.y / 1000;
            if ngx >= 0 && ngx < rust_royale_core::constants::ARENA_WIDTH as i32 {
                let t = &grid.tiles
                    [(gy * rust_royale_core::constants::ARENA_WIDTH as i32 + ngx) as usize];
                if match t {
                    TileType::River => profile.is_flying,
                    TileType::Tower | TileType::Wall => false,
                    _ => true,
                } {
                    final_move_x = move_x;
                }
            }
            // Try sliding along Y axis
            let ny = pos.y + move_y;
            let ngy = ny / 1000;
            let gx = pos.x / 1000;
            if ngy >= 0 && ngy < rust_royale_core::constants::ARENA_HEIGHT as i32 {
                let t = &grid.tiles
                    [(ngy * rust_royale_core::constants::ARENA_WIDTH as i32 + gx) as usize];
                if match t {
                    TileType::River => profile.is_flying,
                    TileType::Tower | TileType::Wall => false,
                    _ => true,
                } {
                    final_move_y = move_y;
                }
            }
        }

        // Emergency escape — if currently inside an impassable tile, allow any move
        let cur_gx = pos.x / 1000;
        let cur_gy = pos.y / 1000;
        let stuck = if cur_gx >= 0
            && cur_gx < rust_royale_core::constants::ARENA_WIDTH as i32
            && cur_gy >= 0
            && cur_gy < rust_royale_core::constants::ARENA_HEIGHT as i32
        {
            let cur_tile = &grid.tiles
                [(cur_gy * rust_royale_core::constants::ARENA_WIDTH as i32 + cur_gx) as usize];
            match cur_tile {
                TileType::River => !profile.is_flying,
                TileType::Tower | TileType::Wall => true,
                _ => false,
            }
        } else {
            true
        };

        if stuck {
            pos.x += move_x;
            pos.y += move_y;
        } else {
            pos.x += final_move_x;
            pos.y += final_move_y;
        }
    }
}

pub fn troop_collision_system(
    grid: Res<ArenaGrid>,
    mut queries: ParamSet<(
        Query<(Entity, &Position)>,
        Query<(
            &mut Position,
            &PhysicalBody,
            &TargetingProfile,
            &Team,
            &Target,
            &AttackStats,
        )>,
    )>,
) {
    let pos_lookup: HashMap<Entity, (i32, i32)> =
        queries.p0().iter().map(|(e, p)| (e, (p.x, p.y))).collect();

    let mut p1 = queries.p1();
    let mut combinations = p1.iter_combinations_mut();

    while let Some(
        [(mut pos_a, body_a, profile_a, team_a, target_a, atk_a), (mut pos_b, body_b, profile_b, team_b, target_b, atk_b)],
    ) = combinations.fetch_next()
    {
        if profile_a.is_flying != profile_b.is_flying {
            continue;
        }

        let dx = (pos_a.x - pos_b.x) as f32;
        let dy = (pos_a.y - pos_b.y) as f32;
        let dist_sq = dx * dx + dy * dy;
        let min_dist = (body_a.radius + body_b.radius) as f32;

        if dist_sq >= min_dist * min_dist {
            continue;
        }

        let (dx, dy, dist) = if dist_sq <= 0.1 {
            let nx = (pos_a.x % 3) as f32 - 1.0;
            let ny = (pos_a.y % 3) as f32 - 1.0;
            let (nx, ny) = if nx == 0.0 && ny == 0.0 {
                (1.0, 0.0)
            } else {
                (nx, ny)
            };
            let d = (nx * nx + ny * ny).sqrt();
            (nx, ny, d)
        } else {
            (dx, dy, dist_sq.sqrt())
        };

        let overlap = min_dist - dist;
        let total_mass = (body_a.mass + body_b.mass) as f32;
        let push_ratio_a = body_b.mass as f32 / total_mass;
        let push_ratio_b = body_a.mass as f32 / total_mass;
        let col_dir_x = dx / dist;
        let col_dir_y = dy / dist;

        let is_same_team = team_a == team_b;
        let shares_target = is_same_team && target_a.0.is_some() && target_a.0 == target_b.0;

        let (push_dir_x, push_dir_y, push_force) = if shares_target {
            let target_ent = target_a.0.unwrap();
            if let Some(&(tx, ty)) = pos_lookup.get(&target_ent) {
                let mid_x = (pos_a.x + pos_b.x) as f32 / 2.0;
                let mid_y = (pos_a.y + pos_b.y) as f32 / 2.0;
                let to_tx = tx as f32 - mid_x;
                let to_ty = ty as f32 - mid_y;
                let to_td = (to_tx * to_tx + to_ty * to_ty).sqrt();
                if to_td > 0.1 {
                    let ttx = to_tx / to_td;
                    let tty = to_ty / to_td;
                    let perp_x = -tty;
                    let perp_y = ttx;
                    let sign = if dx * perp_x + dy * perp_y >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    (perp_x * sign + ttx * 0.1, perp_y * sign + tty * 0.1, 0.7)
                } else {
                    (col_dir_x, col_dir_y, 0.3)
                }
            } else {
                (col_dir_x, col_dir_y, 0.3)
            }
        } else if is_same_team {
            (col_dir_x, col_dir_y, 0.3)
        } else {
            (col_dir_x, col_dir_y, 0.8)
        };

        let mut push_ax = (push_dir_x * overlap * push_ratio_a * push_force) as i32;
        let mut push_ay = (push_dir_y * overlap * push_ratio_a * push_force) as i32;
        let mut push_bx = (push_dir_x * overlap * push_ratio_b * push_force) as i32;
        let mut push_by = (push_dir_y * overlap * push_ratio_b * push_force) as i32;

        if is_same_team && shares_target {
            let range_diff = atk_a.range - atk_b.range;
            if range_diff.abs() > 0.3 {
                if range_diff < 0.0 {
                    push_ax = (push_ax as f32 * 0.15) as i32;
                    push_ay = (push_ay as f32 * 0.15) as i32;
                } else {
                    push_bx = (push_bx as f32 * 0.15) as i32;
                    push_by = (push_by as f32 * 0.15) as i32;
                }
            }
        }

        let new_ax = pos_a.x + push_ax;
        let new_ay = pos_a.y + push_ay;
        let gax = new_ax / 1000;
        let gay = new_ay / 1000;
        let mut a_blocked = true;
        if gax >= 0
            && gax < rust_royale_core::constants::ARENA_WIDTH as i32
            && gay >= 0
            && gay < rust_royale_core::constants::ARENA_HEIGHT as i32
        {
            let tile =
                &grid.tiles[(gay * rust_royale_core::constants::ARENA_WIDTH as i32 + gax) as usize];
            if match tile {
                TileType::River => profile_a.is_flying,
                TileType::Tower | TileType::Wall => false,
                _ => true,
            } {
                a_blocked = false;
            }
        }

        let (new_bx, new_by) = if a_blocked {
            (pos_b.x - (push_ax + push_bx), pos_b.y - (push_ay + push_by))
        } else {
            (pos_b.x - push_bx, pos_b.y - push_by)
        };

        let gbx = new_bx / 1000;
        let gby = new_by / 1000;
        let mut b_blocked = true;
        if gbx >= 0
            && gbx < rust_royale_core::constants::ARENA_WIDTH as i32
            && gby >= 0
            && gby < rust_royale_core::constants::ARENA_HEIGHT as i32
        {
            let tile =
                &grid.tiles[(gby * rust_royale_core::constants::ARENA_WIDTH as i32 + gbx) as usize];
            if match tile {
                TileType::River => profile_b.is_flying,
                TileType::Tower | TileType::Wall => false,
                _ => true,
            } {
                pos_b.x = new_bx;
                pos_b.y = new_by;
                b_blocked = false;
            }
        }

        if b_blocked && !a_blocked {
            let fax = pos_a.x + (push_ax + push_bx);
            let fay = pos_a.y + (push_ay + push_by);
            let fgx = fax / 1000;
            let fgy = fay / 1000;
            let ok = fgx >= 0
                && fgx < rust_royale_core::constants::ARENA_WIDTH as i32
                && fgy >= 0
                && fgy < rust_royale_core::constants::ARENA_HEIGHT as i32
                && match grid.tiles
                    [(fgy * rust_royale_core::constants::ARENA_WIDTH as i32 + fgx) as usize]
                {
                    TileType::River => profile_a.is_flying,
                    TileType::Tower | TileType::Wall => false,
                    _ => true,
                };
            pos_a.x = if ok { fax } else { new_ax };
            pos_a.y = if ok { fay } else { new_ay };
        } else if !a_blocked {
            pos_a.x = new_ax;
            pos_a.y = new_ay;
        }

        // Bridge deadlock fallback
        if a_blocked && b_blocked && is_same_team {
            pos_a.x += (col_dir_x * overlap * push_ratio_a * 0.3) as i32;
            pos_a.y += (col_dir_y * overlap * push_ratio_a * 0.3) as i32;
            pos_b.x -= (col_dir_x * overlap * push_ratio_b * 0.3) as i32;
            pos_b.y -= (col_dir_y * overlap * push_ratio_b * 0.3) as i32;
        }
    }
}
