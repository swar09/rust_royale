use crate::arena::{ArenaGrid, TileType};
use crate::components::{
    AttackStats, AttackTimer, Health, PlayerState, Position, SpawnRequest, Target, Team, Velocity,
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
    mut player_state: ResMut<PlayerState>, // <-- 1. Ask Bevy for the Player's bank account!
) {
    for request in spawn_requests.read() {
        if let Some(troop_data) = global_stats.0.troops.get(&request.card_key) {
            // --- 2. THE VALIDATION GATE ---
            let cost = troop_data.elixir_cost as f32;

            if player_state.elixir < cost {
                // The player is broke! Reject the click and move on.
                println!(
                    "ERROR: Not enough Elixir! Need {}, but only have {:.1}",
                    cost, player_state.elixir
                );
                continue;
            }

            // --- 3. THE TRANSACTION ---
            player_state.elixir -= cost;
            println!(
                "Spent {} Elixir. Remaining: {:.1}",
                cost, player_state.elixir
            );

            // Convert grid coordinates to fixed-point center-of-tile coordinates
            let fixed_x = (request.grid_x * 1000) + 500;
            let fixed_y = (request.grid_y * 1000) + 500;

            // --- THE ENUM TO MATH TRANSLATION ---
            let math_speed = match troop_data.speed {
                SpeedTier::Slow => 1000,     // 1.0 tiles per second
                SpeedTier::Medium => 1500,   // 1.5 tiles per second
                SpeedTier::Fast => 2000,     // 2.0 tiles per second
                SpeedTier::VeryFast => 2500, // 2.5 tiles per second
            };

            let entity_id = commands
                .spawn((
                    Position {
                        x: fixed_x,
                        y: fixed_y,
                    },
                    Velocity(math_speed), // Give the entity physical speed!
                    Health(troop_data.health),
                    request.team,
                    // --- NEW COMBAT COMPONENTS ---
                    Target(None), // Starts with no target
                    AttackStats {
                        damage: troop_data.damage,
                        range: troop_data.range,
                        hit_speed_ms: troop_data.hit_speed_ms,
                    },
                    // Create a repeating timer based on the JSON hit speed
                    AttackTimer(Timer::from_seconds(
                        troop_data.hit_speed_ms as f32 / 1000.0,
                        TimerMode::Repeating,
                    )),
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
    // We add &Target to the query so we know if they are fighting!
    mut query: Query<(&mut Position, &Velocity, &Team, &Target)>,
) {
    // time.delta_seconds() ensures movement is tied to actual time, not frame rate!
    let delta_time = time.delta_seconds();

    for (mut pos, velocity, team, target) in query.iter_mut() {
        // --- NEW LINE: If we have a target, STAND STILL! ---
        if target.0.is_some() {
            continue;
        }

        // Calculate how much distance to move this frame
        // Multiply by 1000 to keep it in our Fixed-Point format
        let frame_movement = (velocity.0 as f32 * delta_time) as i32;

        // Player 1 (Blue) walks UP the Y-axis. Player 2 (Red) walks DOWN.
        match team {
            Team::Blue => pos.y += frame_movement,
            Team::Red => pos.y -= frame_movement,
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
    mut attackers: Query<(Entity, &Position, &Team, &AttackStats, &mut Target)>,
    // Query 2: The Defenders (Everyone on the board who has Health)
    defenders: Query<(Entity, &Position, &Team), With<Health>>,
) {
    for (attacker_ent, attacker_pos, attacker_team, attack_stats, mut target) in
        attackers.iter_mut()
    {
        // If they already have a target, skip the scanning math to save CPU
        if target.0.is_some() {
            continue;
        }

        let mut closest_enemy = None;
        let mut closest_dist = f32::MAX;

        // Scan every other unit on the board
        for (defender_ent, defender_pos, defender_team) in defenders.iter() {
            // Only look at the enemy team!
            if attacker_team != defender_team {
                // Calculate distance using the Pythagorean theorem, converted to Float Tiles
                let dx = (attacker_pos.x - defender_pos.x) as f32 / 1000.0;
                let dy = (attacker_pos.y - defender_pos.y) as f32 / 1000.0;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < closest_dist {
                    closest_dist = dist;
                    closest_enemy = Some(defender_ent);
                }
            }
        }

        // If we found an enemy, and they are inside our attack range... LOCK ON!
        if let Some(enemy_ent) = closest_enemy {
            if closest_dist <= attack_stats.range {
                target.0 = Some(enemy_ent);
                println!(
                    "Entity {:?} Locked onto Enemy {:?} at distance {:.2}",
                    attacker_ent, enemy_ent, closest_dist
                );
            }
        }
    }
}

pub fn combat_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut attackers: Query<(Entity, &mut AttackTimer, &AttackStats, &mut Target)>,
    // Notice we removed mut from Health here for the quick check
    mut defenders: Query<&mut Health>,
) {
    for (attacker_ent, mut timer, stats, mut target) in attackers.iter_mut() {
        let target_entity = match target.0 {
            Some(ent) => ent,
            None => continue,
        };

        // --- THE FIX: INSTANT GHOST TARGET CHECK ---
        // If the engine can no longer find the defender's health, it means they
        // were killed by someone else! Clear the target instantly and skip this frame.
        if defenders.get(target_entity).is_err() {
            target.0 = None;
            continue;
        }

        // 2. Tick the attack animation clock
        timer.0.tick(time.delta());

        // 3. If the clock just finished
        if timer.0.just_finished() {
            // We already proved the defender exists, so it's safe to unwrap
            if let Ok(mut defender_health) = defenders.get_mut(target_entity) {
                defender_health.0 -= stats.damage;
                println!(
                    "Entity {:?} hit {:?} for {} damage! (Target HP: {})",
                    attacker_ent, target_entity, stats.damage, defender_health.0
                );

                if defender_health.0 <= 0 {
                    println!("Entity {:?} was SLAIN!", target_entity);
                    commands.entity(target_entity).despawn();

                    // Clear our own target when we get the kill
                    target.0 = None;
                }
            }
        }
    }
}
