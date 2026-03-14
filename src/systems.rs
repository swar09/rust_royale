use crate::arena::{ArenaGrid, TileType};
use crate::components::{
    AttackStats, AttackTimer, DeployTimer, Health, MatchPhase, MatchState, PhysicalBody, Position,
    SpawnRequest, Target, TargetingProfile, Team, TowerFootprint, TowerType, Velocity,
    WaypointPath,
};
use crate::constants::{ARENA_HEIGHT, ARENA_WIDTH, TILE_SIZE};
use crate::pathfinding::calculate_a_star;
use crate::stats::{GlobalStats, SpeedTier};
use bevy::{app::AppExit, prelude::*};
use std::collections::HashMap;

/// Spawns the 2D camera so we can actually see the world
pub fn setup_camera(mut commands: Commands, mut window_query: Query<&mut Window>) {
    let mut camera = Camera2dBundle::default();

    // Automatically scale the camera so the entire grid (plus a small margin) is ALWAYS visible.
    // This fixes clipping issues for users on smaller laptop screens like MacBooks.
    let min_width = (ARENA_WIDTH as f32 * TILE_SIZE) + 100.0;
    let min_height = (ARENA_HEIGHT as f32 * TILE_SIZE) + 100.0;

    camera.projection.scaling_mode = bevy::render::camera::ScalingMode::AutoMin {
        min_width,
        min_height,
    };

    commands.spawn(camera);

    // Maximize the window on startup
    if let Ok(mut window) = window_query.get_single_mut() {
        window.set_maximized(true);
    }
}

/// Uses Bevy's Gizmos to draw the 18x32 wireframe matrix
pub fn draw_debug_grid(mut gizmos: Gizmos, grid: Res<ArenaGrid>) {
    let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
    let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    // Draw the Background Tiles
    for y in 0..ARENA_HEIGHT {
        for x in 0..ARENA_WIDTH {
            let color = match grid.tiles[y * ARENA_WIDTH + x] {
                TileType::Grass => Color::DARK_GREEN,
                TileType::River => Color::BLUE,
                TileType::Bridge => Color::GRAY,
                TileType::Tower => Color::GOLD,
                TileType::Wall => Color::DARK_GRAY,
            };

            let pos = Vec2::new(
                start_x + (x as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
                start_y + (y as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
            );

            // Draw a slightly smaller rect to see the grid lines
            gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 0.9), color);
        }
    }
}

pub fn mouse_interaction(
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    let window = window_query.single();
    let (camera, camera_transform) = camera_query.single();

    // 1. Get mouse position in world coordinates
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
    {
        // 2. Map continuous world position to grid indices
        // We add the offset to align with the grid drawing math
        let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
        let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

        let grid_x = ((world_position.x + total_width / 2.0) / TILE_SIZE) as i32;
        let grid_y = ((world_position.y + total_height / 2.0) / TILE_SIZE) as i32;

        // 3. Highlight the tile if inside the 18x32 bounds
        if grid_x >= 0 && grid_x < ARENA_WIDTH as i32 && grid_y >= 0 && grid_y < ARENA_HEIGHT as i32
        {
            let pos = Vec2::new(
                (-total_width / 2.0) + (grid_x as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
                (-total_height / 2.0) + (grid_y as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
            );

            // Draw it slightly larger than 0.9 so it surrounds the tile and doesn't Z-fight!
            gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 1.05), Color::YELLOW);
        }
    }
}

pub fn window_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
    mut window_query: Query<&mut Window>,
) {
    // Esc to Close
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }

    // Tab to Toggle Fullscreen (so you can minimize manually)
    if let Ok(mut window) = window_query.get_single_mut() {
        if keyboard_input.just_pressed(KeyCode::Tab) {
            window.mode = match window.mode {
                bevy::window::WindowMode::Windowed => bevy::window::WindowMode::Fullscreen,
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }
}

pub fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut spawn_events: EventWriter<SpawnRequest>, // This lets us fire the event!
) {
    // Only run this code on the exact frame the user clicks Left Click
    if buttons.just_pressed(MouseButton::Left) {
        let window = window_query.single();
        let (camera, camera_transform) = camera_query.single();

        // 1. Raycast the mouse pixel to the 2D world
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        {
            let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
            let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

            // 2. Convert to discrete grid coordinates
            let grid_x = ((world_position.x + total_width / 2.0) / TILE_SIZE) as i32;
            let grid_y = ((world_position.y + total_height / 2.0) / TILE_SIZE) as i32;

            // 3. If the click is inside the 18x32 arena, trigger the spawn!
            if grid_x >= 0
                && grid_x < ARENA_WIDTH as i32
                && grid_y >= 0
                && grid_y < ARENA_HEIGHT as i32
            {
                println!("Mouse clicked on grid [{}, {}]", grid_x, grid_y);

                // Fire the event! For testing, we hardcode the "knight".
                spawn_events.send(SpawnRequest {
                    card_key: "knight".to_string(),
                    team: Team::Blue,
                    grid_x,
                    grid_y,
                });
            }
        }
    }
}

pub fn match_manager_system(
    mut commands: Commands,
    time: Res<Time>,
    mut match_state: ResMut<MatchState>,
    mut grid: ResMut<ArenaGrid>,
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
                } else {
                    if crowns == 3 {
                        match_state.red_crowns = 3; // King Tower guarantees exactly 3 crowns
                    } else {
                        match_state.red_crowns = (match_state.red_crowns + crowns).min(3);
                    }
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

pub fn spawn_entity_system(
    mut commands: Commands,
    mut spawn_requests: EventReader<SpawnRequest>,
    global_stats: Res<GlobalStats>,
    mut match_state: ResMut<MatchState>,
    grid: Res<ArenaGrid>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return; // No spawning after the game ends!
    }

    for request in spawn_requests.read() {
        if let Some(troop_data) = global_stats.0.troops.get(&request.card_key) {
            // --- TERRAIN / BOUNDARY VALIDATION ---
            if request.grid_x < 0
                || request.grid_x >= crate::constants::ARENA_WIDTH as i32
                || request.grid_y < 0
                || request.grid_y >= crate::constants::ARENA_HEIGHT as i32
            {
                println!("ERROR: Cannot deploy outside the arena bounds!");
                continue;
            }

            let tile_index =
                (request.grid_y * crate::constants::ARENA_WIDTH as i32 + request.grid_x) as usize;
            let tile = &grid.tiles[tile_index];

            let can_deploy = match tile {
                TileType::Grass | TileType::Bridge => true,
                _ => false, // Cannot deploy on River, Tower, or Wall!
            };

            if !can_deploy {
                println!("ERROR: Cannot deploy on {:?} tile!", tile);
                continue;
            }

            let cost = troop_data.elixir_cost as f32;

            // --- DUAL ECONOMY VALIDATION ---
            let (current_elixir, team_name) = match request.team {
                Team::Blue => (match_state.blue_elixir, "Blue"),
                Team::Red => (match_state.red_elixir, "Red"),
            };

            if current_elixir < cost {
                println!(
                    "ERROR: {} Team needs {} Elixir, but only has {:.1}",
                    team_name, cost, current_elixir
                );
                continue;
            }

            // Deduct from the correct bank
            if request.team == Team::Blue {
                match_state.blue_elixir -= cost;
            } else {
                match_state.red_elixir -= cost;
            }
            println!(
                "Spent {} Elixir from {} Team. Remaining: {:.1}",
                cost,
                team_name,
                if request.team == Team::Blue {
                    match_state.blue_elixir
                } else {
                    match_state.red_elixir
                }
            );

            // Convert grid coordinates to fixed-point center-of-tile coordinates
            let fixed_x = (request.grid_x * 1000) + 500;
            let fixed_y = (request.grid_y * 1000) + 500;

            // --- THE ENUM TO MATH TRANSLATION ---
            // 1 unit of speed = 0.02 tiles/sec (CR logic) mapped to Fixed-Point (1000 = 1 tile)
            let math_speed = match troop_data.speed {
                SpeedTier::VerySlow => 600,  // 30  units = 0.6 tiles/sec
                SpeedTier::Slow => 900,      // 45  units = 0.9 tiles/sec
                SpeedTier::Medium => 1200,   // 60  units = 1.2 tiles/sec
                SpeedTier::Fast => 1800,     // 90  units = 1.8 tiles/sec
                SpeedTier::VeryFast => 2400, // 120 units = 2.4 tiles/sec
            };

            // Calculate the radius (footprint / 2) in fixed-point math
            let collision_radius = (troop_data.footprint_x as i32 * 1000) / 2;

            let entity_id = commands
                .spawn((
                    Position {
                        x: fixed_x,
                        y: fixed_y,
                    },
                    Velocity(math_speed),
                    Health(troop_data.health),
                    request.team,
                    Target(None),
                    // --- THE PHYSICAL BODY ---
                    PhysicalBody {
                        radius: collision_radius,
                        mass: troop_data.mass,
                    },
                    AttackStats {
                        damage: troop_data.damage,
                        range: troop_data.range,
                        hit_speed_ms: troop_data.hit_speed_ms,
                        first_attack_sec: troop_data.first_attack_sec,
                    },
                    // Create a repeating timer based on the JSON hit speed
                    AttackTimer(Timer::from_seconds(
                        troop_data.hit_speed_ms as f32 / 1000.0,
                        TimerMode::Repeating,
                    )),
                    // --- READ THE JSON DELAY HERE ---
                    DeployTimer(Timer::from_seconds(
                        troop_data.deploy_time_sec,
                        TimerMode::Once,
                    )),
                    TargetingProfile {
                        is_flying: troop_data.is_flying,
                        is_building: false, // Troops are never buildings!
                        targets_air: troop_data.targets_air,
                        targets_ground: troop_data.targets_ground,
                        preference: troop_data.target_preference.clone(),
                    },
                    WaypointPath(Vec::new()), // <-- AND THE NEW PATHFINDER
                ))
                .id();

            println!(
                "SPAWNED: {} (Entity {:?}) at Grid [{}, {}] with {} HP, Speed {}!",
                troop_data.name,
                entity_id,
                request.grid_x,
                request.grid_y,
                troop_data.health,
                math_speed
            );
        } else {
            println!(
                "ERROR: Card '{}' not found in stats.json!",
                request.card_key
            );
        }
    }
}

pub fn physics_movement_system(
    time: Res<Time>,
    match_state: Res<MatchState>,
    grid: Res<crate::arena::ArenaGrid>,
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

                            if w_dist < 250.0 {
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
                    if dist < 250.0 {
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
            && grid_x < crate::constants::ARENA_WIDTH as i32
            && grid_y >= 0
            && grid_y < crate::constants::ARENA_HEIGHT as i32
        {
            let dest_tile =
                &grid.tiles[(grid_y * crate::constants::ARENA_WIDTH as i32 + grid_x) as usize];
            full_step_valid = match dest_tile {
                crate::arena::TileType::River => profile.is_flying,
                crate::arena::TileType::Tower | crate::arena::TileType::Wall => false,
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
            if ngx >= 0 && ngx < crate::constants::ARENA_WIDTH as i32 {
                let tile = &grid.tiles[(gy * crate::constants::ARENA_WIDTH as i32 + ngx) as usize];
                if match tile {
                    crate::arena::TileType::River => profile.is_flying,
                    crate::arena::TileType::Tower | crate::arena::TileType::Wall => false,
                    _ => true,
                } {
                    final_move_x = move_x;
                }
            }
            // Slide Y
            let ny = pos.y + move_y;
            let ngy = ny / 1000;
            let gx = pos.x / 1000;
            if ngy >= 0 && ngy < crate::constants::ARENA_HEIGHT as i32 {
                let tile = &grid.tiles[(ngy * crate::constants::ARENA_WIDTH as i32 + gx) as usize];
                if match tile {
                    crate::arena::TileType::River => profile.is_flying,
                    crate::arena::TileType::Tower | crate::arena::TileType::Wall => false,
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
            && cur_gx < crate::constants::ARENA_WIDTH as i32
            && cur_gy >= 0
            && cur_gy < crate::constants::ARENA_HEIGHT as i32
        {
            let cur_tile =
                &grid.tiles[(cur_gy * crate::constants::ARENA_WIDTH as i32 + cur_gx) as usize];
            match cur_tile {
                crate::arena::TileType::River => !profile.is_flying,
                crate::arena::TileType::Tower | crate::arena::TileType::Wall => true,
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

pub fn draw_entities(
    mut gizmos: Gizmos,
    query: Query<(
        &Position,
        &Team,
        Option<&TargetingProfile>,
        Option<&PhysicalBody>,
    )>,
) {
    let total_width = crate::constants::ARENA_WIDTH as f32 * crate::constants::TILE_SIZE;
    let total_height = crate::constants::ARENA_HEIGHT as f32 * crate::constants::TILE_SIZE;

    // We start from the bottom left corner
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    for (pos, team, profile, body) in query.iter() {
        // 1. Convert fixed-point (e.g., 1500) back to float grid coords (1.5)
        let float_x = pos.x as f32 / 1000.0;
        let float_y = pos.y as f32 / 1000.0;

        // 2. Multiply by tile size to get screen pixels
        let screen_x = start_x + (float_x * crate::constants::TILE_SIZE);
        let screen_y = start_y + (float_y * crate::constants::TILE_SIZE);

        let color = match team {
            Team::Blue => Color::CYAN,
            Team::Red => Color::TOMATO,
        };

        if let Some(prof) = profile {
            if prof.is_building {
                // To get the true size in pixels, we look at the 'radius' (which is footprint / 2)
                let visual_width_tiles = if let Some(b) = body {
                    // physical body radius is stored as (footprint * 1000) / 2
                    // We want the total width in tiles: (radius * 2) / 1000
                    (b.radius * 2) as f32 / 1000.0
                } else {
                    3.0 // Fallback
                };

                gizmos.rect_2d(
                    Vec2::new(screen_x, screen_y),
                    0.0,
                    Vec2::splat(crate::constants::TILE_SIZE * visual_width_tiles),
                    color,
                );
                continue;
            }
        }

        // Draw the walking troops as a filled circle!
        gizmos.circle_2d(
            Vec2::new(screen_x, screen_y),
            crate::constants::TILE_SIZE * 0.4,
            color,
        );
    }
}

pub fn setup_ui(mut commands: Commands) {
    // We create a Text node and instantly give it our Marker component
    commands.spawn((
        TextBundle::from_section(
            "Elixir: 0.0", // Dummy starting text
            TextStyle {
                font_size: 40.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        }),
        crate::components::ElixirUIText, // <-- THE MAKER TAG!
    ));
}

pub fn update_elixir_ui(
    match_state: Res<MatchState>, // Read the match state
    // Find exactly ONE mutable text component that also has our marker tag
    mut query: Query<&mut Text, With<crate::components::ElixirUIText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        let minutes = (match_state.clock_seconds / 60.0) as u32;
        let seconds = (match_state.clock_seconds % 60.0) as u32;
        text.sections[0].value = format!(
            "⏱ {}:{:02} | 💧 Blue: {:.1} | 🔴 Red: {:.1} | 👑 {}-{}",
            minutes,
            seconds,
            match_state.blue_elixir,
            match_state.red_elixir,
            match_state.blue_crowns,
            match_state.red_crowns
        );
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
        let sight_range =
            if attacker_profile.preference == crate::stats::TargetPreference::Buildings {
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
                if attacker_profile.preference == crate::stats::TargetPreference::Buildings
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
    mut grid: ResMut<ArenaGrid>,
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
                        } else {
                            if matches!(tower, TowerType::King) {
                                match_state.red_crowns = 3; // King Tower instantly sets score to 3
                            } else {
                                match_state.red_crowns = (match_state.red_crowns + 1).min(3);
                            }
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

pub fn deployment_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeployTimer)>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            commands.entity(entity).remove::<DeployTimer>();
            println!("Entity {:?} finished deploying and woke up!", entity);
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
                && grid_ax < crate::constants::ARENA_WIDTH as i32
                && grid_ay >= 0
                && grid_ay < crate::constants::ARENA_HEIGHT as i32
            {
                let tile_a = &grid.tiles
                    [(grid_ay * crate::constants::ARENA_WIDTH as i32 + grid_ax) as usize];
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
                && grid_bx < crate::constants::ARENA_WIDTH as i32
                && grid_by >= 0
                && grid_by < crate::constants::ARENA_HEIGHT as i32
            {
                let tile_b = &grid.tiles
                    [(grid_by * crate::constants::ARENA_WIDTH as i32 + grid_bx) as usize];
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
                    && fgx < crate::constants::ARENA_WIDTH as i32
                    && fgy >= 0
                    && fgy < crate::constants::ARENA_HEIGHT as i32
                {
                    let tile_f =
                        &grid.tiles[(fgy * crate::constants::ARENA_WIDTH as i32 + fgx) as usize];
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
            if a_blocked && b_blocked && shares_target {
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

pub fn spawn_towers_system(mut commands: Commands, global_stats: Res<GlobalStats>) {
    let princess_data = global_stats.0.buildings.get("princess_tower").unwrap();
    let king_data = global_stats.0.buildings.get("king_tower").unwrap();

    let towers = vec![
        // Player Side (Blue)
        (
            "princess_tower",
            Team::Blue,
            3,
            5,
            princess_data,
            TowerType::Princess,
        ),
        (
            "princess_tower",
            Team::Blue,
            14,
            5,
            princess_data,
            TowerType::Princess,
        ),
        ("king_tower", Team::Blue, 8, 1, king_data, TowerType::King),
        // Opponent Side (Red)
        (
            "princess_tower",
            Team::Red,
            3,
            24,
            princess_data,
            TowerType::Princess,
        ),
        (
            "princess_tower",
            Team::Red,
            14,
            24,
            princess_data,
            TowerType::Princess,
        ),
        ("king_tower", Team::Red, 8, 27, king_data, TowerType::King),
    ];

    for (name, team, start_x, start_y, data, tower_type) in towers {
        // Calculate center precisely based on footprint
        let size_x = data.footprint_x as f32;
        let size_y = data.footprint_y as f32;

        let center_float_x = start_x as f32 + (size_x / 2.0);
        let center_float_y = start_y as f32 + (size_y / 2.0);

        let fixed_x = (center_float_x * 1000.0) as i32;
        let fixed_y = (center_float_y * 1000.0) as i32;

        let collision_radius = (data.footprint_x as i32 * 1000) / 2;
        let footprint_size = data.footprint_x as usize; // Towers are square (3x3 or 4x4)

        commands.spawn((
            Position {
                x: fixed_x,
                y: fixed_y,
            },
            Health(data.health),
            team,
            Target(None),
            PhysicalBody {
                radius: collision_radius,
                mass: 99_999, // Immovable!
            },
            AttackStats {
                damage: data.damage,
                range: data.range_max,
                hit_speed_ms: data.hit_speed_ms,
                first_attack_sec: data.first_attack_sec,
            },
            AttackTimer(Timer::from_seconds(
                data.hit_speed_ms as f32 / 1000.0,
                TimerMode::Repeating,
            )),
            TargetingProfile {
                is_flying: false,
                is_building: true,
                targets_air: true,
                targets_ground: true,
                preference: crate::stats::TargetPreference::Any,
            },
            tower_type,
            TowerFootprint {
                start_x: start_x as usize,
                start_y: start_y as usize,
                size: footprint_size,
            },
        ));

        println!(
            "SPAWNED: {} (Team: {:?}) at Center Grids [{}, {}]!",
            name, team, center_float_x, center_float_y
        );
    }
}
