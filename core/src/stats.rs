#![allow(dead_code)]
use bevy::prelude::Resource;
use serde::Deserialize;
use std::collections::HashMap;

// --- 1. The Strict Enums ---

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum SpeedTier {
    #[serde(rename = "Very Slow")]
    VerySlow,
    Slow,
    Medium,
    Fast,
    #[serde(rename = "Very Fast")]
    VeryFast,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum TargetPreference {
    Any,
    Buildings,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum SplashType {
    #[serde(rename = "target_centered")]
    TargetCentered,
    #[serde(rename = "self_centered")]
    SelfCentered,
    #[serde(rename = "linear")]
    Linear,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum SpellType {
    #[serde(rename = "damage")]
    Damage,
    #[serde(rename = "spawner")]
    Spawner,
}

// --- 2. The Data Structs ---

#[derive(Deserialize, Debug, Clone)]
pub struct BuildingStats {
    pub id: u32,
    pub name: String,
    pub health: i32,
    pub damage: i32,
    pub hit_speed_ms: u32,
    pub deploy_time_sec: f32,
    pub first_attack_sec: f32,
    pub range_max: f32, // Changed from 'range' to support the Mortar logic
    pub footprint_x: usize,
    pub footprint_y: usize,

    // Optional edge cases we discussed
    pub range_min: Option<f32>,
    pub death_payload_id: Option<u32>,
    pub hidden_when_inactive: Option<bool>,
    pub spawns_troop_id: Option<u32>,
    pub ignores_deployment_zones: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TroopStats {
    pub id: u32,
    pub name: String,
    pub elixir_cost: u32,
    pub health: i32,
    pub damage: i32,
    pub hit_speed_ms: u32,
    pub deploy_time_sec: f32,
    pub first_attack_sec: f32,

    // Using our strict Enums!
    pub speed: SpeedTier,
    pub target_preference: TargetPreference,

    pub range: f32,
    pub footprint_x: usize,
    pub footprint_y: usize,

    // --- The New Physics & Targeting Rules ---
    pub is_flying: bool,
    pub targets_air: bool,
    pub targets_ground: bool,
    pub mass: i32,

    // Splash Mechanics (Optional)
    pub splash_radius: Option<f32>,
    pub splash_type: Option<SplashType>, // Enum!
    pub pierce_length: Option<f32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpellStats {
    pub id: u32,
    pub name: String,
    pub elixir_cost: u32,

    pub spell_type: SpellType, // Enum!
    pub radius: f32,           // The continuous float for Area of Effect

    pub damage: Option<i32>,
    pub crown_tower_damage: Option<i32>,
    pub knockback_force: Option<i32>,
    pub spawns_troop_id: Option<u32>,
    pub spawn_count: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameStats {
    pub buildings: HashMap<String, BuildingStats>,
    pub troops: HashMap<String, TroopStats>,
    pub spells: HashMap<String, SpellStats>,
}

// 3. Make it a Bevy Resource so any System can read it
#[derive(Resource)]
pub struct GlobalStats(pub GameStats);
