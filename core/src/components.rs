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

// Identifies which team owns the unit
#[derive(Component, Debug, PartialEq, Clone, Copy)]
pub enum Team {
    Blue, // Player 1
    Red,  // Player 2
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
#[derive(Component, Debug)]
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

// Stores the turn-by-turn grid coordinates the unit needs to walk to
#[derive(Component, Debug, Default)]
pub struct WaypointPath(pub Vec<(i32, i32)>);
