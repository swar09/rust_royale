use bevy::prelude::*;
use rust_royale_core::arena::TileType;
use rust_royale_core::components::{
    AttackStats, AttackTimer, DeployTimer, Health, MatchPhase, MatchState, PhysicalBody, Position,
    SpawnRequest, Target, TargetingProfile, Team, TowerFootprint, TowerType, Velocity,
    WaypointPath,
};
use rust_royale_core::stats::{GlobalStats, SpeedTier};

pub fn spawn_entity_system(
    mut commands: Commands,
    mut spawn_requests: EventReader<SpawnRequest>,
    global_stats: Res<GlobalStats>,
    mut match_state: ResMut<MatchState>,
    grid: Res<rust_royale_core::arena::ArenaGrid>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return; // No spawning after the game ends!
    }

    for request in spawn_requests.read() {
        if let Some(troop_data) = global_stats.0.troops.get(&request.card_key) {
            // --- TERRAIN / BOUNDARY VALIDATION ---
            if request.grid_x < 0
                || request.grid_x >= rust_royale_core::constants::ARENA_WIDTH as i32
                || request.grid_y < 0
                || request.grid_y >= rust_royale_core::constants::ARENA_HEIGHT as i32
            {
                println!("ERROR: Cannot deploy outside the arena bounds!");
                continue;
            }

            let tile_index = (request.grid_y * rust_royale_core::constants::ARENA_WIDTH as i32
                + request.grid_x) as usize;
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
        let footprint_size = data.footprint_x; // Towers are square (3x3 or 4x4)

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
                preference: rust_royale_core::stats::TargetPreference::Any,
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
