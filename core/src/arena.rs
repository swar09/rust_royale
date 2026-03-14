use crate::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use bevy::prelude::*;

#[derive(Default, PartialEq, Clone, Copy, Debug)]
pub enum TileType {
    #[default]
    Grass,
    River,
    Bridge,
    Tower,
    Wall,
}

#[derive(Resource)]
pub struct ArenaGrid {
    pub tiles: Vec<TileType>,
}

impl Default for ArenaGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl ArenaGrid {
    pub fn new() -> Self {
        let mut tiles = vec![TileType::Grass; ARENA_WIDTH * ARENA_HEIGHT];

        // --- THE WALLS ---
        // 1. Solid Outer Boundary (Left and Right edges)
        for y in 0..ARENA_HEIGHT {
            tiles[y * ARENA_WIDTH] = TileType::Wall;
            tiles[y * ARENA_WIDTH + (ARENA_WIDTH - 1)] = TileType::Wall;
        }

        // 2. Solid Back Corners (y=0 and y=31, columns 1-6 and 13-18 since 0/19 are already Wall)
        // This leaves the space behind the King Tower (x=7..=12) open for deployment!
        for x in 1..=6 {
            tiles[x] = TileType::Wall;
            tiles[(ARENA_HEIGHT - 1) * ARENA_WIDTH + x] = TileType::Wall;
        }
        for x in 13..=18 {
            tiles[x] = TileType::Wall;
            tiles[(ARENA_HEIGHT - 1) * ARENA_WIDTH + x] = TileType::Wall;
        }

        // Define the River (Rows 15 and 16)
        for y in 15..=16 {
            for x in 1..=18 {
                tiles[y * ARENA_WIDTH + x] = TileType::River;
            }
        }

        // Define Bridges (X-coordinates 3-5 and 14-16)
        for y in 15..=16 {
            for x in 3..=5 {
                tiles[y * ARENA_WIDTH + x] = TileType::Bridge;
            }
            for x in 14..=16 {
                tiles[y * ARENA_WIDTH + x] = TileType::Bridge;
            }
        }
        // --- PLAYER SIDE ---
        Self::place_tower(&mut tiles, 3, 5, 3); // Left Princess
        Self::place_tower(&mut tiles, 14, 5, 3); // Right Princess
        Self::place_tower(&mut tiles, 8, 1, 4); // King Tower

        // --- OPPONENT SIDE ---
        Self::place_tower(&mut tiles, 3, 24, 3); // Left Princess
        Self::place_tower(&mut tiles, 14, 24, 3); // Right Princess
        Self::place_tower(&mut tiles, 8, 27, 4); // King Tower

        Self { tiles }
    }

    fn place_tower(tiles: &mut [TileType], start_x: usize, start_y: usize, size: usize) {
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
