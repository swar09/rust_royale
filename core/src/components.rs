#![allow(dead_code)]
use bevy::prelude::*;

// The exact fixed-point coordinate of the unit
#[derive(Component, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

// The health pool
#[derive(Component, Debug)]
pub struct Health(pub i32);
#[derive(Component, Debug)]
pub struct MaxHealth(pub i32);
#[derive(Component)]
pub struct HealthValueText;

// Identifies which team owns the unit
#[derive(Component, Debug, PartialEq, Clone, Copy, Eq, Hash, Default)]
pub enum Team {
    #[default]
    Blue, // Player 1
    Red,  // Player 2
}

#[derive(Component, Debug)]
pub struct DeathSpawn {
    pub card_key: String,
    pub count: u32,
}

#[derive(Event, Debug)]
pub struct DeathSpawnEvent {
    pub card_key: String,
    pub count: u32,
    pub team: Team,
    pub fixed_x: i32,
    pub fixed_y: i32,
}

#[derive(Event, Debug)]
pub struct TowerDeathEvent {
    pub tower_entity: Entity,
    pub tower_type: TowerType,
    pub team: Team,
}

// The Event triggered when the UI asks to drop a card
#[derive(Event)]
pub struct SpawnRequest {
    pub card_key: String,
    pub team: Team,
    pub grid_x: i32,
    pub grid_y: i32,
}

// The continuous fixed-point speed of the unit
#[derive(Component, Debug)]
pub struct Velocity(pub i32);

#[derive(Component, Debug, PartialEq)]
pub enum TowerStatus {
    Sleeping,
    Active,
}

// The different phases of a 3-minute match
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MatchPhase {
    #[default]
    Regular, // First 2 minutes (1x Elixir)
    DoubleElixir, // Last 1 minute (2x Elixir)
    Overtime,     // Sudden Death (2x Elixir)
    GameOver,     // Match has ended
}

// The global state for the entire match
#[derive(Resource, Debug)]
pub struct MatchState {
    pub phase: MatchPhase,
    pub clock_seconds: f32, // Starts at 180.0 (3 minutes)
    pub blue_elixir: f32,   // Capped at 10.0
    pub red_elixir: f32,    // Capped at 10.0
    pub blue_crowns: u8,
    pub red_crowns: u8,
}

impl Default for MatchState {
    fn default() -> Self {
        Self {
            phase: MatchPhase::Regular,
            clock_seconds: 180.0,
            blue_elixir: 5.0, // Standard starting elixir
            red_elixir: 5.0,
            blue_crowns: 0,
            red_crowns: 0,
        }
    }
}

// Tags towers so the combat system knows when to award crowns
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum TowerType {
    Princess,
    King,
}

// Stores the grid origin and size so we can clear ArenaGrid tiles on destruction
#[derive(Component, Debug)]
pub struct TowerFootprint {
    pub start_x: usize,
    pub start_y: usize,
    pub size: usize,
}

#[derive(Component)]
pub struct ElixirUIText;

// Stores the specific Entity ID of the enemy we are currently attacking
#[derive(Component, Debug)]
pub struct Target(pub Option<Entity>);

// Holds the raw combat stats we read from JSON
#[derive(Component, Debug)]
pub struct AttackStats {
    pub damage: i32,
    pub range: f32, // Stored as tiles (e.g., 1.2)
    pub hit_speed_ms: u32,
    pub first_attack_sec: f32,
    pub projectile_speed: i32, // Fixed-point speed (e.g., 6000 units/sec)
}

/// Defines the splash damage profile for AoE melee/ranged attacks (Valkyrie, Baby Dragon)
#[derive(Component, Debug)]
pub struct SplashProfile {
    pub splash_radius: f32,   // Radius in tiles
    pub splash_type: crate::stats::SplashType,
}

// A Bevy stopwatch to ensure they only swing the sword every X seconds
#[derive(Component, Debug)]
pub struct AttackTimer(pub Timer);

// A countdown timer for when a troop is first dropped
#[derive(Component, Debug)]
pub struct DeployTimer(pub Timer);

// Stores the physical footprint and weight for collision pushing
#[derive(Component, Debug)]
pub struct PhysicalBody {
    pub radius: i32, // Stored in fixed-point math (1000 = 1 tile)
    pub mass: i32,
}

// Defines what this unit is, and what it is allowed to attack
#[derive(Component, Debug)]
pub struct TargetingProfile {
    pub is_flying: bool,
    pub is_building: bool,
    pub targets_air: bool,
    pub targets_ground: bool,
    pub preference: crate::stats::TargetPreference,
}

#[derive(Component, Debug)]
pub struct Projectile {
    pub damage: i32,
    pub speed: i32,          // Fixed-point speed (e.g., 5000 units per second)
    pub splash_radius: f32,  // 0.0 = single target, > 0.0 = AoE radius in tiles
    pub attacker_team: Team, // Which team fired this projectile
}

// A tag to identify an active spell waiting to explode
#[derive(Component, Debug)]
pub struct SpellStrike;

// The raw data for the Area of Effect blast
#[derive(Component, Debug)]
pub struct AoEPayload {
    pub damage: i32,
    pub tower_damage: i32, // Clash Royale spells do reduced damage to towers!
    pub radius: i32,       // Fixed-point math (e.g., 2.5 tiles = 2500 units)
    pub waves_total: u32,
    pub waves_remaining: u32,
    pub knockback: i32,
}

// Stores the turn-by-turn grid coordinates the unit needs to walk to
#[derive(Component, Debug, Default)]
pub struct WaypointPath(pub std::collections::VecDeque<(i32, i32)>);

/// The lane the unit was deployed in.
/// Locks a troop's default march goal to its deployment side so a P.E.K.K.A
/// dropped on the left doesn't suddenly veer toward the right tower.
/// Only overridden when the unit is actively targeting an enemy (distraction logic).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum SpawnLane {
    Left,  // grid_x < ARENA_WIDTH / 2  (x < 10)
    Right, // grid_x >= ARENA_WIDTH / 2 (x >= 10)
}

#[derive(Debug, Clone)]
pub struct Deck {
    pub hand: [Option<String>; 4],
    pub queue: Vec<String>,
}

impl Deck {
    pub fn new_shuffled(salt: u64) -> Self {
        let mut all_cards = vec![
            "knight".to_string(),
            "archer".to_string(),
            "minions".to_string(),
            "arrows".to_string(),
            "fireball".to_string(),
            "giant".to_string(),
            "musketeer".to_string(),
            "mini_pekka".to_string(),
        ];

        // Simple shuffle using system time + salt as seed (LCG)
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64;
        seed = seed.wrapping_add(salt);

        for i in (1..all_cards.len()).rev() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (seed as usize) % (i + 1);
            all_cards.swap(i, j);
        }

        let hand = [
            Some(all_cards.remove(0)),
            Some(all_cards.remove(0)),
            Some(all_cards.remove(0)),
            Some(all_cards.remove(0)),
        ];

        Self {
            hand,
            queue: all_cards,
        }
    }
}

#[derive(Resource, Debug)]
pub struct PlayerDeck {
    pub blue: Deck,
    pub red: Deck,
    pub blue_selected: Option<usize>,
    pub red_selected: Option<usize>,
}

// Markers for the UI Card buttons
#[derive(Component)]
pub struct CardUI {
    pub slot_index: usize,
    pub team: Team,
}

// The global state holding what we are currently dragging
#[derive(Resource, Default)]
pub struct DragState {
    pub is_dragging: bool,
    pub slot_index: usize,
    pub card_key: String,
    pub team: Team,
}

// A tag for the visual hologram floating under the cursor 
#[derive(Component)]
pub struct DragHologram;

impl Default for PlayerDeck {
    fn default() -> Self {
        Self {
            blue: Deck::new_shuffled(0),
            red: Deck::new_shuffled(99999),
            blue_selected: None,
            red_selected: None,
        }
    }
}