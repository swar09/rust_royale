use crate::arena::{ArenaGrid, TileType};
use crate::components::{
    AttackStats, AttackTimer, DeployTimer, Health, PhysicalBody, PlayerState, Position,
    SpawnRequest, Target, TargetingProfile, Team, Velocity,
};
use crate::constants::{ARENA_HEIGHT, ARENA_WIDTH, TILE_SIZE};
use crate::stats::{GlobalStats, SpeedTier};
use bevy::{app::AppExit, prelude::*};

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

pub fn elixir_generation_system(time: Res<Time>, mut player_state: ResMut<PlayerState>) {
    // 1 Elixir every 2.8 seconds = ~0.357 Elixir per second
    let generation_rate = 1.0 / 2.8;

    // time.delta_seconds() ensures it is perfectly tied to the clock, not frame rate
    player_state.elixir += generation_rate * time.delta_seconds();

    // The strict 10.0 cap
    if player_state.elixir > 10.0 {
        player_state.elixir = 10.0;
    }
}

pub fn spawn_entity_system(
    mut commands: Commands,
    mut spawn_requests: EventReader<SpawnRequest>,
    global_stats: Res<GlobalStats>,
    mut player_state: ResMut<PlayerState>,
) {
    for request in spawn_requests.read() {
        if let Some(troop_data) = global_stats.0.troops.get(&request.card_key) {
            // --- THE VALIDATION GATE ---
            let cost = troop_data.elixir_cost as f32;

            if player_state.elixir < cost {
                println!(
                    "ERROR: Not enough Elixir! Need {}, but only have {:.1}",
                    cost, player_state.elixir
                );
                continue;
            }

            // --- THE TRANSACTION ---
            player_state.elixir -= cost;
            println!(
                "Spent {} Elixir. Remaining: {:.1}",
                cost, player_state.elixir
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
                    // --- THE TACTICAL BRAIN ---
                    TargetingProfile {
                        is_flying: troop_data.is_flying,
                        is_building: false, // Troops are never buildings!
                        targets_air: troop_data.targets_air,
                        targets_ground: troop_data.targets_ground,
                        preference: troop_data.target_preference.clone(),
                    },
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
    mut movers: Query<
        (
            Entity,
            &mut Position,
            &Velocity,
            &Team,
            &Target,
            &AttackStats,
        ),
        Without<DeployTimer>,
    >,
) {
    let delta_time = time.delta_seconds();

    // Pass 1: Snapshot all current positions into a HashMap so we can look up targets
    // without needing a second (conflicting) query.
    let position_snapshot: std::collections::HashMap<Entity, (i32, i32)> = movers
        .iter()
        .map(|(ent, pos, _, _, _, _)| (ent, (pos.x, pos.y)))
        .collect();

    // Pass 2: Now iterate mutably and apply movement
    for (_ent, mut pos, velocity, team, target, attack_stats) in movers.iter_mut() {
        let frame_movement = (velocity.0 as f32 * delta_time) as i32;

        match target.0 {
            Some(target_ent) => {
                // Look up the target's position from our snapshot
                if let Some(&(tx, ty)) = position_snapshot.get(&target_ent) {
                    let dx = (tx - pos.x) as f32 / 1000.0;
                    let dy = (ty - pos.y) as f32 / 1000.0;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist <= attack_stats.range {
                        // In range — STAND STILL and fight!
                        continue;
                    }

                    // Out of range — CHASE the target!
                    if dist > 0.01 {
                        let dir_x = dx / dist;
                        let dir_y = dy / dist;
                        pos.x += (dir_x * frame_movement as f32) as i32;
                        pos.y += (dir_y * frame_movement as f32) as i32;
                    }
                }
                // If target not found in snapshot, ghost target cleanup will handle it
            }
            None => {
                // No target — march toward enemy base
                match team {
                    Team::Blue => pos.y += frame_movement,
                    Team::Red => pos.y -= frame_movement,
                }
            }
        }
    }
}

pub fn draw_entities(mut gizmos: Gizmos, query: Query<(&Position, &Team)>) {
    let total_width = crate::constants::ARENA_WIDTH as f32 * crate::constants::TILE_SIZE;
    let total_height = crate::constants::ARENA_HEIGHT as f32 * crate::constants::TILE_SIZE;

    // We start from the bottom left corner
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    for (pos, team) in query.iter() {
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

        // Draw the unit as a filled circle!
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
    player_state: Res<PlayerState>, // Read bank account
    // Find exactly ONE mutable text component that also has our marker tag
    mut query: Query<&mut Text, With<crate::components::ElixirUIText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        // Update the string on screen! {:.1} rounds it to 1 decimal place (e.g. 5.4)
        text.sections[0].value = format!("Elixir: {:.1}", player_state.elixir);
    }
}

pub fn targeting_system(
    // Query 1: The Attackers (Looking for a target)
    mut attackers: Query<
        (
            Entity,
            &Position,
            &Team,
            &AttackStats,
            &TargetingProfile,
            &mut Target,
            &mut AttackTimer,
        ),
        Without<DeployTimer>,
    >,
    // Query 2: The Defenders (Everyone on the board who has Health)
    defenders: Query<(Entity, &Position, &Team, &TargetingProfile), With<Health>>,
) {
    for (
        attacker_ent,
        attacker_pos,
        attacker_team,
        attack_stats,
        attacker_profile,
        mut target,
        mut attack_timer,
    ) in attackers.iter_mut()
    {
        // If they already have a target, skip the scanning math to save CPU
        if target.0.is_some() {
            continue;
        }

        let mut closest_enemy = None;
        let mut closest_dist = f32::MAX;

        // Scan every other unit on the board
        for (defender_ent, defender_pos, defender_team, defender_profile) in defenders.iter() {
            // Only look at the enemy team!
            if attacker_team != defender_team {
                // --- RULE 1: AIR TARGETING ---
                if defender_profile.is_flying && !attacker_profile.targets_air {
                    continue; // Defender is in the air, but I can't look up!
                }

                // --- RULE 2: GROUND TARGETING ---
                if !defender_profile.is_flying && !attacker_profile.targets_ground {
                    continue; // Defender is on the ground, but I only shoot air!
                }

                // --- RULE 3: TARGET PREFERENCE ---
                // If I am a Giant (Buildings Only) and you are NOT a building, skip!
                if attacker_profile.preference == crate::stats::TargetPreference::Buildings
                    && !defender_profile.is_building
                {
                    continue;
                }

                // --- THE MATH ---
                let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
                let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < closest_dist {
                    closest_dist = dist;
                    closest_enemy = Some(defender_ent);
                }
            }
        }

        // If we found an enemy, LOCK ON regardless of distance!
        // The movement system will handle chasing if they're out of range.
        if let Some(enemy_ent) = closest_enemy {
            target.0 = Some(enemy_ent);

            // Only pre-charge the fast first attack if we're already in striking distance
            if closest_dist <= attack_stats.range {
                attack_timer
                    .0
                    .set_duration(std::time::Duration::from_secs_f32(
                        attack_stats.first_attack_sec,
                    ));
                attack_timer.0.reset();
            }

            println!(
                "Entity {:?} Locked onto Enemy {:?} at distance {:.2}",
                attacker_ent, enemy_ent, closest_dist
            );
        }
    }
}

pub fn combat_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut attackers: Query<(
        Entity,
        &Position,
        &mut AttackTimer,
        &AttackStats,
        &mut Target,
    )>,
    mut defenders: Query<(&Position, &mut Health)>,
) {
    for (attacker_ent, attacker_pos, mut timer, stats, mut target) in attackers.iter_mut() {
        let target_entity = match target.0 {
            Some(ent) => ent,
            None => continue,
        };

        // --- INSTANT GHOST TARGET CHECK ---
        if defenders.get(target_entity).is_err() {
            target.0 = None;
            continue;
        }

        // --- RANGE CHECK: Only tick the attack clock if we're in striking distance ---
        // If out of range, DON'T drop the target — the movement system will chase.
        // We just skip damage this frame so the timer doesn't tick while we're running.
        if let Ok((defender_pos, _)) = defenders.get(target_entity) {
            let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
            let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
            let dist = (dx * dx + dy * dy).sqrt();

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

            if let Ok((_, mut defender_health)) = defenders.get_mut(target_entity) {
                defender_health.0 -= stats.damage;
                println!(
                    "Entity {:?} hit {:?} for {} damage! (Target HP: {})",
                    attacker_ent, target_entity, stats.damage, defender_health.0
                );

                if defender_health.0 <= 0 {
                    println!("Entity {:?} was SLAIN!", target_entity);
                    commands.entity(target_entity).despawn();
                    target.0 = None;
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
    // We only want to push things that have both a Position and a PhysicalBody
    mut query: Query<(&mut Position, &PhysicalBody, &TargetingProfile)>,
) {
    // iter_combinations_mut lets us compare every pair of troops exactly once per frame
    let mut combinations = query.iter_combinations_mut();

    while let Some([(mut pos_a, body_a, profile_a), (mut pos_b, body_b, profile_b)]) =
        combinations.fetch_next()
    {
        // --- LAYER CHECK: Flying units don't collide with ground units! ---
        if profile_a.is_flying != profile_b.is_flying {
            continue; // One is in the air, one is on the ground — they phase through each other
        }

        let dx = (pos_a.x - pos_b.x) as f32;
        let dy = (pos_a.y - pos_b.y) as f32;
        let dist_sq = dx * dx + dy * dy;

        let min_dist = (body_a.radius + body_b.radius) as f32;

        // If they are overlapping (and not literally on the exact same 0,0 pixel to avoid divide-by-zero)
        if dist_sq > 0.1 && dist_sq < min_dist * min_dist {
            let dist = dist_sq.sqrt();
            let overlap = min_dist - dist;

            // --- THE MASS CALCULATION ---
            // The heavier you are, the less you get pushed.
            let total_mass = (body_a.mass + body_b.mass) as f32;
            let push_ratio_a = body_b.mass as f32 / total_mass; // A takes B's mass % of the push
            let push_ratio_b = body_a.mass as f32 / total_mass; // B takes A's mass % of the push

            // Normalize the direction vector
            let dir_x = dx / dist;
            let dir_y = dy / dist;

            // Apply the separation force!
            // We multiply by 0.5 to smooth out the pushing so it doesn't instantly teleport them
            let push_force = 0.5;

            // Push A away from B
            pos_a.x += (dir_x * overlap * push_ratio_a * push_force) as i32;
            pos_a.y += (dir_y * overlap * push_ratio_a * push_force) as i32;

            // Push B away from A (Notice the minus signs to push the opposite way!)
            pos_b.x -= (dir_x * overlap * push_ratio_b * push_force) as i32;
            pos_b.y -= (dir_y * overlap * push_ratio_b * push_force) as i32;
        }
    }
}
