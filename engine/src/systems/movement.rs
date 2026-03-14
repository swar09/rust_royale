#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use rust_royale_core::arena::{ArenaGrid, TileType};
use rust_royale_core::components::{
    AttackStats, DeployTimer, MatchPhase, MatchState, PhysicalBody, Position, Target,
    TargetingProfile, Team, Velocity, WaypointPath,
};
use rust_royale_core::pathfinding::calculate_a_star;
use std::collections::HashMap;

pub fn physics_movement_system(
    time: Res<Time>,
    match_state: Res<MatchState>,
    grid: Res<ArenaGrid>,
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
            ),
            Without<DeployTimer>,
        >,
    )>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return;
    }

    let delta_time = time.delta_seconds();

    // Snapshot ALL current positions (and radii for buildings) into a HashMap
    let mut position_snapshot: HashMap<Entity, (i32, i32, i32)> = HashMap::new();
    for (ent, pos, body) in queries.p0().iter() {
        let radius = body.map_or(0, |b| b.radius);
        position_snapshot.insert(ent, (pos.x, pos.y, radius));
    }

    // Simple movement — always walk straight toward the target.
    for (_ent, mut pos, velocity, team, target, attack_stats, profile, mut path) in
        queries.p1().iter_mut()
    {
        let frame_movement = (velocity.0 as f32 * delta_time) as i32;

        let mut move_x = 0;
        let mut move_y = 0;

        match target.0 {
            Some(target_ent) => {
                if let Some(&(tx, ty, target_radius)) = position_snapshot.get(&target_ent) {
                    let dx = (tx - pos.x) as f32 / 1000.0;
                    let dy = (ty - pos.y) as f32 / 1000.0;
                    let center_dist = (dx * dx + dy * dy).sqrt();
                    let dist = center_dist - (target_radius as f32 / 1000.0);

                    if dist <= attack_stats.range {
                        continue; // In range — STAND STILL and fight!
                    }

                    if dist > 0.01 {
                        let target_grid = (tx / 1000, ty / 1000);

                        // If we don't have a route yet, calculate one to the enemy!
                        if path.0.is_empty() {
                            let start_grid = (pos.x / 1000, pos.y / 1000);

                            let target_radius_tiles = (target_radius as f32 / 1000.0) as i32;
                            let total_range = (attack_stats.range as i32) + target_radius_tiles;

                            if let Some(new_route) = calculate_a_star(
                                &grid,
                                start_grid,
                                target_grid,
                                profile.is_flying,
                                total_range.max(1),
                            ) {
                                path.0 = new_route;
                            }
                        }

                        // Follow the GPS path!
                        if let Some(&(target_grid_x, target_grid_y)) = path.0.first() {
                            let target_world_x = (target_grid_x * 1000) + 500;
                            let target_world_y = (target_grid_y * 1000) + 500;

                            let wdx = (target_world_x - pos.x) as f32;
                            let wdy = (target_world_y - pos.y) as f32;
                            let w_dist = (wdx * wdx + wdy * wdy).sqrt();

                            if w_dist < 600.0 {
                                path.0.remove(0);
                            } else {
                                let dir_x = wdx / w_dist;
                                let dir_y = wdy / w_dist;
                                move_x = (dir_x * frame_movement as f32) as i32;
                                move_y = (dir_y * frame_movement as f32) as i32;
                            }
                        } else {
                            // Brute force walk straight at the target
                            let dir_x = dx / center_dist;
                            let dir_y = dy / center_dist;
                            move_x = (dir_x * frame_movement as f32) as i32;
                            move_y = (dir_y * frame_movement as f32) as i32;
                        }
                    }
                }
            }
            None => {
                // --- GPS NAVIGATION ---

                // 1. If we don't have a route yet, calculate one!
                if path.0.is_empty() {
                    let start_grid = (pos.x / 1000, pos.y / 1000);

                    // Set the destination to the grass tile right IN FRONT of the King Tower
                    let goal_grid = match team {
                        Team::Blue => (7, 26), // One tile below the Red King
                        Team::Red => (7, 2),   // One tile above the Blue King
                    };

                    if let Some(new_route) =
                        calculate_a_star(&grid, start_grid, goal_grid, profile.is_flying, 0)
                    {
                        path.0 = new_route;
                        println!(
                            "Entity {:?} calculated a path with {} steps!",
                            _ent,
                            path.0.len()
                        );
                    }
                }

                // 2. Follow the route!
                if let Some(&(target_grid_x, target_grid_y)) = path.0.first() {
                    // Convert grid target to exact fixed-point center of the tile
                    let target_world_x = (target_grid_x * 1000) + 500;
                    let target_world_y = (target_grid_y * 1000) + 500;

                    let dx = (target_world_x - pos.x) as f32;
                    let dy = (target_world_y - pos.y) as f32;
                    let dist = (dx * dx + dy * dy).sqrt();

                    // 3. Have we reached the center of the tile?
                    if dist < 600.0 {
                        // Cross it off the list! The next frame will target the next tile.
                        path.0.remove(0);
                    } else {
                        // Keep walking toward the current waypoint
                        let dir_x = dx / dist;
                        let dir_y = dy / dist;
                        move_x = (dir_x * frame_movement as f32) as i32;
                        move_y = (dir_y * frame_movement as f32) as i32;
                    }
                }
            }
        }

        // --- THE WALL CHECK ---
        // --- THE SLIDING WALL CHECK ---
        let mut final_move_x = 0;
        let mut final_move_y = 0;

        let grid_x = (pos.x + move_x) / 1000;
        let grid_y = (pos.y + move_y) / 1000;

        let is_using_gps = !path.0.is_empty();

        // 1. Check if the full step is valid
        let mut full_step_valid = false;
        if grid_x >= 0
            && grid_x < rust_royale_core::constants::ARENA_WIDTH as i32
            && grid_y >= 0
            && grid_y < rust_royale_core::constants::ARENA_HEIGHT as i32
        {
            let dest_tile = &grid.tiles
                [(grid_y * rust_royale_core::constants::ARENA_WIDTH as i32 + grid_x) as usize];
            full_step_valid = match dest_tile {
                rust_royale_core::arena::TileType::River => profile.is_flying,
                rust_royale_core::arena::TileType::Tower
                | rust_royale_core::arena::TileType::Wall => false,
                _ => true,
            };
        }

        // 2. Decide how to move
        if full_step_valid || is_using_gps {
            // Full move OK (or we trust GPS)
            final_move_x = move_x;
            final_move_y = move_y;
        } else {
            // Full move BLOCKED. Try SLIDING!
            // Slide X
            let nx = pos.x + move_x;
            let ngx = nx / 1000;
            let gy = pos.y / 1000;
            if ngx >= 0 && ngx < rust_royale_core::constants::ARENA_WIDTH as i32 {
                let tile = &grid.tiles
                    [(gy * rust_royale_core::constants::ARENA_WIDTH as i32 + ngx) as usize];
                if match tile {
                    rust_royale_core::arena::TileType::River => profile.is_flying,
                    rust_royale_core::arena::TileType::Tower
                    | rust_royale_core::arena::TileType::Wall => false,
                    _ => true,
                } {
                    final_move_x = move_x;
                }
            }
            // Slide Y
            let ny = pos.y + move_y;
            let ngy = ny / 1000;
            let gx = pos.x / 1000;
            if ngy >= 0 && ngy < rust_royale_core::constants::ARENA_HEIGHT as i32 {
                let tile = &grid.tiles
                    [(ngy * rust_royale_core::constants::ARENA_WIDTH as i32 + gx) as usize];
                if match tile {
                    rust_royale_core::arena::TileType::River => profile.is_flying,
                    rust_royale_core::arena::TileType::Tower
                    | rust_royale_core::arena::TileType::Wall => false,
                    _ => true,
                } {
                    final_move_y = move_y;
                }
            }
        }

        // 3. Emergency Escape: If currently stuck, ALWAYS allow ANY move to get out!
        let cur_gx = pos.x / 1000;
        let cur_gy = pos.y / 1000;
        let currently_stuck = if cur_gx >= 0
            && cur_gx < rust_royale_core::constants::ARENA_WIDTH as i32
            && cur_gy >= 0
            && cur_gy < rust_royale_core::constants::ARENA_HEIGHT as i32
        {
            let cur_tile = &grid.tiles
                [(cur_gy * rust_royale_core::constants::ARENA_WIDTH as i32 + cur_gx) as usize];
            match cur_tile {
                rust_royale_core::arena::TileType::River => !profile.is_flying,
                rust_royale_core::arena::TileType::Tower
                | rust_royale_core::arena::TileType::Wall => true,
                _ => false,
            }
        } else {
            true
        };

        if currently_stuck {
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
    // Snapshot all positions so we can look up target positions during collision resolution
    let pos_lookup: HashMap<Entity, (i32, i32)> =
        queries.p0().iter().map(|(e, p)| (e, (p.x, p.y))).collect();

    let mut p1 = queries.p1();
    let mut combinations = p1.iter_combinations_mut();

    while let Some(
        [(mut pos_a, body_a, profile_a, team_a, target_a, atk_a), (mut pos_b, body_b, profile_b, team_b, target_b, atk_b)],
    ) = combinations.fetch_next()
    {
        // --- LAYER CHECK: Flying units don't collide with ground units! ---
        if profile_a.is_flying != profile_b.is_flying {
            continue;
        }

        let dx = (pos_a.x - pos_b.x) as f32;
        let dy = (pos_a.y - pos_b.y) as f32;
        let dist_sq = dx * dx + dy * dy;

        let min_dist = (body_a.radius + body_b.radius) as f32;

        // If they are overlapping
        if dist_sq < min_dist * min_dist {
            // FIX: If they are on the EXACT same pixel (dist_sq == 0), give them a tiny deterministic nudge!
            let (dx, dy, dist) = if dist_sq <= 0.1 {
                let nudge_x = (pos_a.x % 3) as f32 - 1.0;
                let nudge_y = (pos_a.y % 3) as f32 - 1.0;
                let (nx, ny) = if nudge_x == 0.0 && nudge_y == 0.0 {
                    (1.0, 0.0)
                } else {
                    (nudge_x, nudge_y)
                };
                let pseudo_dist = (nx * nx + ny * ny).sqrt();
                (nx, ny, pseudo_dist)
            } else {
                (dx, dy, dist_sq.sqrt())
            };

            let overlap = min_dist - dist;

            // --- THE MASS CALCULATION ---
            let total_mass = (body_a.mass + body_b.mass) as f32;
            let push_ratio_a = body_b.mass as f32 / total_mass;
            let push_ratio_b = body_a.mass as f32 / total_mass;

            // Normalize the collision axis (A ←→ B direction)
            let col_dir_x = dx / dist;
            let col_dir_y = dy / dist;

            let is_same_team = team_a == team_b;
            let shares_target = is_same_team && target_a.0.is_some() && target_a.0 == target_b.0;

            // --- PUSH DIRECTION LOGIC ---
            // For ENEMIES: push along the collision axis (standard physics)
            // For SAME-TEAM + SAME-TARGET: push perpendicular to the TARGET direction!
            //   This is the key CR insight — the target (tower) doesn't move, so the
            //   perpendicular direction is ALWAYS THE SAME. No spinning, no oscillation.
            let (push_dir_x, push_dir_y, push_force) = if shares_target {
                // Get the shared target's position
                let target_ent = target_a.0.unwrap();
                if let Some(&(tx, ty)) = pos_lookup.get(&target_ent) {
                    // Direction from the midpoint of the two troops toward their target
                    let mid_x = (pos_a.x + pos_b.x) as f32 / 2.0;
                    let mid_y = (pos_a.y + pos_b.y) as f32 / 2.0;
                    let to_target_x = tx as f32 - mid_x;
                    let to_target_y = ty as f32 - mid_y;
                    let to_target_dist =
                        (to_target_x * to_target_x + to_target_y * to_target_y).sqrt();

                    if to_target_dist > 0.1 {
                        let ttx = to_target_x / to_target_dist;
                        let tty = to_target_y / to_target_dist;

                        // Perpendicular to the target direction (rotate 90°)
                        let perp_x = -tty;
                        let perp_y = ttx;

                        // Determine which side each troop should go:
                        let side_dot = dx * perp_x + dy * perp_y;
                        let sign = if side_dot >= 0.0 { 1.0 } else { -1.0 };

                        // Add a tiny 10% FORWARD bias to the fanning so they don't stop moving!
                        let fan_dir_x = perp_x * sign + ttx * 0.1;
                        let fan_dir_y = perp_y * sign + tty * 0.1;

                        (fan_dir_x, fan_dir_y, 0.7) // Stronger fanning, includes forward bias
                    } else {
                        // Fallback: too close to target, use normal collision axis
                        (col_dir_x, col_dir_y, 0.3)
                    }
                } else {
                    // Target not found, fall back to normal collision
                    (col_dir_x, col_dir_y, 0.3)
                }
            } else if is_same_team {
                (col_dir_x, col_dir_y, 0.3) // Soft collision for teammates
            } else {
                (col_dir_x, col_dir_y, 0.8) // Hard collision for enemies
            };

            // Calculate push deltas
            let mut push_ax = (push_dir_x * overlap * push_ratio_a * push_force) as i32;
            let mut push_ay = (push_dir_y * overlap * push_ratio_a * push_force) as i32;
            let mut push_bx = (push_dir_x * overlap * push_ratio_b * push_force) as i32;
            let mut push_by = (push_dir_y * overlap * push_ratio_b * push_force) as i32;

            // --- SLIDE-THROUGH FOR SHORT-RANGE BEHIND LONG-RANGE ---
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

            // Try to move A
            let new_ax = pos_a.x + push_ax;
            let new_ay = pos_a.y + push_ay;
            let grid_ax = new_ax / 1000;
            let grid_ay = new_ay / 1000;
            let mut a_blocked = true;

            if grid_ax >= 0
                && grid_ax < rust_royale_core::constants::ARENA_WIDTH as i32
                && grid_ay >= 0
                && grid_ay < rust_royale_core::constants::ARENA_HEIGHT as i32
            {
                let tile_a = &grid.tiles[(grid_ay * rust_royale_core::constants::ARENA_WIDTH as i32
                    + grid_ax) as usize];
                let can_walk_a = match tile_a {
                    TileType::River => profile_a.is_flying,
                    TileType::Tower | TileType::Wall => false,
                    _ => true,
                };
                if can_walk_a {
                    a_blocked = false;
                }
            }

            // Try to move B
            let mut new_bx = pos_b.x - push_bx;
            let mut new_by = pos_b.y - push_by;

            // If A was blocked, try to push B twice as much (the full overlap)
            if a_blocked {
                new_bx = pos_b.x - (push_ax + push_bx);
                new_by = pos_b.y - (push_ay + push_by);
            }

            let grid_bx = new_bx / 1000;
            let grid_by = new_by / 1000;
            let mut b_blocked = true;

            if grid_bx >= 0
                && grid_bx < rust_royale_core::constants::ARENA_WIDTH as i32
                && grid_by >= 0
                && grid_by < rust_royale_core::constants::ARENA_HEIGHT as i32
            {
                let tile_b = &grid.tiles[(grid_by * rust_royale_core::constants::ARENA_WIDTH as i32
                    + grid_bx) as usize];
                let can_walk_b = match tile_b {
                    TileType::River => profile_b.is_flying,
                    TileType::Tower | TileType::Wall => false,
                    _ => true,
                };
                if can_walk_b {
                    pos_b.x = new_bx;
                    pos_b.y = new_by;
                    b_blocked = false;
                }
            }

            // If B was blocked but A wasn't, go back and give A the full push!
            // If B was blocked but A wasn't, go back and give A the full push!
            if b_blocked && !a_blocked {
                let final_ax = pos_a.x + (push_ax + push_bx);
                let final_ay = pos_a.y + (push_ay + push_by);
                let fgx = final_ax / 1000;
                let fgy = final_ay / 1000;
                if fgx >= 0
                    && fgx < rust_royale_core::constants::ARENA_WIDTH as i32
                    && fgy >= 0
                    && fgy < rust_royale_core::constants::ARENA_HEIGHT as i32
                {
                    let tile_f = &grid.tiles
                        [(fgy * rust_royale_core::constants::ARENA_WIDTH as i32 + fgx) as usize];
                    if match tile_f {
                        TileType::River => profile_a.is_flying,
                        TileType::Tower | TileType::Wall => false,
                        _ => true,
                    } {
                        pos_a.x = final_ax;
                        pos_a.y = final_ay;
                    } else {
                        // Even final push blocked, just apply original A move
                        pos_a.x = new_ax;
                        pos_a.y = new_ay;
                    }
                }
            } else if !a_blocked {
                // Normal case: A just moves its portion
                pos_a.x = new_ax;
                pos_a.y = new_ay;
            }

            // --- BRIDGE DEADLOCK FALLBACK ---
            // If BOTH A and B were blocked (e.g. they are on a narrow bridge and trying to push sideways into the river),
            // then the sideways fanning failed. In this case, fall back to a standard RADIAL push
            // (along the axis between them) so one pushes the other forward/back to resolve the overlap.
            if a_blocked && b_blocked && is_same_team {
                // Standard radial collision axis
                let r_dir_x = dx / dist;
                let r_dir_y = dy / dist;
                let r_force = 0.3; // Soft teammate push

                let r_push_ax = (r_dir_x * overlap * push_ratio_a * r_force) as i32;
                let r_push_ay = (r_dir_y * overlap * push_ratio_a * r_force) as i32;
                let r_push_bx = (r_dir_x * overlap * push_ratio_b * r_force) as i32;
                let r_push_by = (r_dir_y * overlap * push_ratio_b * r_force) as i32;

                // Simple radial resolution (mostly ignoring terrain since we're desperate to resolve overlap)
                pos_a.x += r_push_ax;
                pos_a.y += r_push_ay;
                pos_b.x -= r_push_bx;
                pos_b.y -= r_push_by;
            }
        }
    }
}
