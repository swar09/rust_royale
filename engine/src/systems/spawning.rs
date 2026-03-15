use bevy::prelude::*;
use rust_royale_core::arena::TileType;
use rust_royale_core::components::{
    AoEPayload, AttackStats, AttackTimer, DeathSpawn, DeathSpawnEvent, DeployTimer, Health,
    MatchPhase, MatchState, PhysicalBody, PlayerDeck, Position, SpawnRequest, SpellStrike, Target,
    TargetingProfile, Team, TowerFootprint, TowerStatus, TowerType, Velocity, WaypointPath,
};
use rust_royale_core::stats::{GlobalStats, SpeedTier};

pub fn spawn_entity_system(
    mut commands: Commands,
    mut spawn_requests: EventReader<SpawnRequest>,
    global_stats: Res<GlobalStats>,
    mut match_state: ResMut<MatchState>,
    grid: Res<rust_royale_core::arena::ArenaGrid>,
    mut deck: ResMut<PlayerDeck>,
    towers: Query<(&Team, &TowerType, &TowerFootprint)>,
) {
    if match_state.phase == MatchPhase::GameOver {
        return; // No spawning after the game ends
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

            // --- DYNAMIC POCKET / DEPLOYMENT ZONE VALIDATION ---
            let divider = rust_royale_core::constants::ARENA_WIDTH as i32 / 2;
            let is_left_lane = request.grid_x < divider;

            let mut red_left_alive = false;
            let mut red_right_alive = false;
            let mut blue_left_alive = false;
            let mut blue_right_alive = false;

            for (t_team, t_type, footprint) in towers.iter() {
                if matches!(t_type, TowerType::Princess) {
                    if *t_team == Team::Red {
                        if footprint.start_x < divider as usize {
                            red_left_alive = true;
                        } else {
                            red_right_alive = true;
                        }
                    } else if *t_team == Team::Blue {
                        if footprint.start_x < divider as usize {
                            blue_left_alive = true;
                        } else {
                            blue_right_alive = true;
                        }
                    }
                }
            }

            let (min_y, max_y) = match request.team {
                Team::Blue => {
                    let max_y = if is_left_lane {
                        if red_left_alive {
                            14
                        } else {
                            20
                        }
                    } else {
                        if red_right_alive {
                            14
                        } else {
                            20
                        }
                    };
                    (0, max_y)
                }
                Team::Red => {
                    let min_y = if is_left_lane {
                        if blue_left_alive {
                            17
                        } else {
                            11
                        }
                    } else {
                        if blue_right_alive {
                            17
                        } else {
                            11
                        }
                    };
                    (min_y, 31)
                }
            };

            if request.grid_y < min_y || request.grid_y > max_y {
                println!(
                    "ERROR: Cannot deploy at Y={}! Zone restricted for Team {:?}. Limits: [{}, {}]",
                    request.grid_y, request.team, min_y, max_y
                );
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

            // --- DECK ROTATION LOGIC ---
            if let Some(selected_idx) = deck.selected_index {
                let team_deck = match request.team {
                    Team::Blue => &mut deck.blue,
                    Team::Red => &mut deck.red,
                };
                if let Some(played_card) = team_deck.hand[selected_idx].take() {
                    team_deck.queue.push(played_card);
                    team_deck.hand[selected_idx] = Some(team_deck.queue.remove(0));
                }
            }
            deck.selected_index = None;

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

            let count = troop_data.spawn_count.unwrap_or(1);

            let mut entity_ids = Vec::new();

            // THE SWARM LOOP
            for i in 0..count {
                let base_x = (request.grid_x * 1000) + 500;
                let base_y = (request.grid_y * 1000) + 500;

                let offset_x = if count > 1 {
                    ((i as i32 % 2) * 400) - 200
                } else {
                    0
                };

                let offset_y = if count > 1 { (i as i32 / 2) * -400 } else { 0 };

                let fixed_x = base_x + offset_x;
                let fixed_y = base_y + offset_y;

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

                let mut entity_cmds = commands.spawn((
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
                ));

                if let Some(ds_card) = &troop_data.death_spawn {
                    entity_cmds.insert(DeathSpawn {
                        card_key: ds_card.clone(),
                        count: troop_data.death_spawn_count.unwrap_or(1),
                    });
                }

                entity_ids.push(entity_cmds.id());
            }

            println!(
                "SPAWNED: {} {}s (Entities {:?}) at Grid [{}, {}]!",
                count, troop_data.name, entity_ids, request.grid_x, request.grid_y
            );
        } else if let Some(spell_data) = global_stats.0.spells.get(&request.card_key) {
            // --- SPELL DEPLOYMENT BRANCH ---

            // 1. Basic Arena Bounds (Spells ignore TileType like Rivers and Towers!)
            if request.grid_x < 0
                || request.grid_x >= rust_royale_core::constants::ARENA_WIDTH as i32
                || request.grid_y < 0
                || request.grid_y >= rust_royale_core::constants::ARENA_HEIGHT as i32
            {
                continue;
            }

            let cost = spell_data.elixir_cost as f32;

            // 2. Economy Validation
            let current_elixir = if request.team == Team::Blue {
                match_state.blue_elixir
            } else {
                match_state.red_elixir
            };
            if current_elixir < cost {
                continue;
            }

            // 3. Deduct Elixir and Rotate Deck
            if request.team == Team::Blue {
                match_state.blue_elixir -= cost;
                if let Some(selected_idx) = deck.selected_index {
                    if let Some(played_card) = deck.blue.hand[selected_idx].take() {
                        deck.blue.queue.push(played_card);
                        deck.blue.hand[selected_idx] = Some(deck.blue.queue.remove(0));
                    }
                }
            } else {
                match_state.red_elixir -= cost;
                if let Some(selected_idx) = deck.selected_index {
                    if let Some(played_card) = deck.red.hand[selected_idx].take() {
                        deck.red.queue.push(played_card);
                        deck.red.hand[selected_idx] = Some(deck.red.queue.remove(0));
                    }
                }
            }
            deck.selected_index = None;

            // 4. Calculate Math & Spawn the Invisible Marker
            let fixed_x = (request.grid_x * 1000) + 500;
            let fixed_y = (request.grid_y * 1000) + 500;
            let fixed_radius = (spell_data.radius * 1000.0) as i32;

            let dmg = spell_data.damage.unwrap_or(0);
            let tower_dmg = spell_data.crown_tower_damage.unwrap_or(dmg / 3); // CR spells do ~30% to towers
            let waves = spell_data.waves.unwrap_or(1);

            commands.spawn((
                Position {
                    x: fixed_x,
                    y: fixed_y,
                },
                request.team,
                SpellStrike,
                DeployTimer(Timer::from_seconds(1.0, TimerMode::Once)), // 1-second travel/fall time
                AoEPayload {
                    damage: dmg / waves as i32,
                    tower_damage: tower_dmg / waves as i32,
                    radius: fixed_radius,
                    waves_total: waves,
                    waves_remaining: waves,
                },
            ));

            println!(
                "☄️ SPAWNED: {} Spell at Grid [{}, {}]!",
                spell_data.name, request.grid_x, request.grid_y
            );
        } else {
            println!(
                "ERROR: Card '{}' not found in troops or spells JSON!",
                request.card_key
            );
        }
    }
}

pub fn deployment_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeployTimer), Without<SpellStrike>>,
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

        let initial_status = match tower_type {
            TowerType::Princess => TowerStatus::Active,
            TowerType::King => TowerStatus::Sleeping,
        };

        let mut tower_cmds = commands.spawn((
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
            initial_status,
            TowerFootprint {
                start_x: start_x as usize,
                start_y: start_y as usize,
                size: footprint_size,
            },
        ));

        if let Some(ds_card) = &data.death_spawn {
            tower_cmds.insert(DeathSpawn {
                card_key: ds_card.clone(),
                count: data.death_spawn_count.unwrap_or(1),
            });
        }

        println!(
            "SPAWNED: {} (Team: {:?}) at Center Grids [{}, {}]!",
            name, team, center_float_x, center_float_y
        );
    }
}

pub fn handle_death_spawns_system(
    mut commands: Commands,
    mut events: EventReader<DeathSpawnEvent>,
    global_stats: Res<GlobalStats>,
) {
    for event in events.read() {
        if let Some(troop_data) = global_stats.0.troops.get(&event.card_key) {
            for i in 0..event.count {
                // Minor offset so they explode outward slightly
                let offset_x = if event.count > 1 {
                    (i as i32 % 2) * 400 - 200
                } else {
                    0
                };
                let offset_y = if event.count > 1 {
                    (i as i32 / 2) * -400
                } else {
                    0
                };

                let math_speed = match troop_data.speed {
                    SpeedTier::VerySlow => 600,
                    SpeedTier::Slow => 900,
                    SpeedTier::Medium => 1200,
                    SpeedTier::Fast => 1800,
                    SpeedTier::VeryFast => 2400,
                };
                let collision_radius = (troop_data.footprint_x as i32 * 1000) / 2;

                commands.spawn((
                    Position {
                        x: event.fixed_x + offset_x,
                        y: event.fixed_y + offset_y,
                    },
                    Velocity(math_speed),
                    Health(troop_data.health),
                    event.team,
                    Target(None),
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
                    AttackTimer(Timer::from_seconds(
                        troop_data.hit_speed_ms as f32 / 1000.0,
                        TimerMode::Repeating,
                    )),
                    // Small delay so they don't attack instantly
                    DeployTimer(Timer::from_seconds(0.1, TimerMode::Once)),
                    TargetingProfile {
                        is_flying: troop_data.is_flying,
                        is_building: false,
                        targets_air: troop_data.targets_air,
                        targets_ground: troop_data.targets_ground,
                        preference: troop_data.target_preference.clone(),
                    },
                    WaypointPath(Vec::new()),
                ));
            }
            println!(
                "💀 DEATH SPAWN: {} {}s popped out!",
                event.count, troop_data.name
            );
        }
    }
}
