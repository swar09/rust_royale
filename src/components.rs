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

// The global state for the player's economy
#[derive(Resource, Debug)]
pub struct PlayerState {
    pub elixir: f32, // Continuous float, capped at 10.0
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
