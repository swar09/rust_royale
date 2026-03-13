use crate::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use bevy::prelude::*;

#[derive(Default, PartialEq, Clone, Copy, Debug)]
pub enum TileType {
    #[default]
    Grass,
    River,
    Bridge,
    Tower,
}

#[derive(Resource)]
pub struct ArenaGrid {
    pub tiles: Vec<TileType>,
}

impl ArenaGrid {
    pub fn new() -> Self {
        let mut tiles = vec![TileType::Grass; ARENA_WIDTH * ARENA_HEIGHT];

        // Define the River (Rows 15 and 16)
        for y in 15..=16 {
            for x in 0..ARENA_WIDTH {
                tiles[y * ARENA_WIDTH + x] = TileType::River;
            }
        }

        // Define Bridges (X-coordinates 3-5 and 12-14)
        for y in 15..=16 {
            for x in 2..5 {
                tiles[y * ARENA_WIDTH + x] = TileType::Bridge;
            }
            for x in 13..16 {
                tiles[y * ARENA_WIDTH + x] = TileType::Bridge;
            }
        }
        // --- PLAYER SIDE ---
        Self::place_tower(&mut tiles, 2, 5, 3); // Left Princess
        Self::place_tower(&mut tiles, 13, 5, 3); // Right Princess
        Self::place_tower(&mut tiles, 7, 1, 4); // King Tower

        // --- OPPONENT SIDE ---
        Self::place_tower(&mut tiles, 2, 24, 3); // Left Princess
        Self::place_tower(&mut tiles, 13, 24, 3); // Right Princess
        Self::place_tower(&mut tiles, 7, 27, 4); // King Tower

        Self { tiles }
    }

    fn place_tower(tiles: &mut Vec<TileType>, start_x: usize, start_y: usize, size: usize) {
        for y in start_y..start_y + size {
            for x in start_x..start_x + size {
                if x < ARENA_WIDTH && y < ARENA_HEIGHT {
                    tiles[y * ARENA_WIDTH + x] = TileType::Tower;
                }
            }
        }
    }

    /// Converts tower tiles back to Grass when a tower is destroyed
    pub fn clear_tower(&mut self, start_x: usize, start_y: usize, size: usize) {
        for y in start_y..start_y + size {
            for x in start_x..start_x + size {
                if x < ARENA_WIDTH && y < ARENA_HEIGHT {
                    self.tiles[y * ARENA_WIDTH + x] = TileType::Grass;
                }
            }
        }
    }
}
