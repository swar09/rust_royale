use bevy::prelude::*;

// --- CONSTANTS ---
const TILE_SIZE: f32 = 20.0; // The visual pixel size of each tile on your screen
const ARENA_WIDTH: usize = 18;
const ARENA_HEIGHT: usize = 32;
#[derive(Default, PartialEq, Clone, Copy, Debug)]
enum TileType {
    #[default]
    Grass,
    River,
    Bridge,
}

#[derive(Resource)]
struct ArenaGrid {
    tiles: Vec<TileType>,
}

impl ArenaGrid {
    fn new() -> Self {
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

        Self { tiles }
    }
}
fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // Loads the window, renderer, and core engine
        .add_systems(Startup, setup_camera) // Runs exactly once when the app starts
        .add_systems(Update, draw_debug_grid) // Runs every single frame (60+ FPS)
        .insert_resource(ArenaGrid::new())
        .run();
}

// --- SYSTEMS ---

/// Spawns the 2D camera so we can actually see the world
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// Uses Bevy's Gizmos to draw the 18x32 wireframe matrix
fn draw_debug_grid(mut gizmos: Gizmos, grid: Res<ArenaGrid>) {
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
