use bevy::prelude::*;

// --- CONSTANTS ---
const TILE_SIZE: f32 = 20.0; // The visual pixel size of each tile on your screen
const ARENA_WIDTH: usize = 18;
const ARENA_HEIGHT: usize = 32;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // Loads the window, renderer, and core engine
        .add_systems(Startup, setup_camera) // Runs exactly once when the app starts
        .add_systems(Update, draw_debug_grid) // Runs every single frame (60+ FPS)
        .run();
}

// --- SYSTEMS ---

/// Spawns the 2D camera so we can actually see the world
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// Uses Bevy's Gizmos to draw the 18x32 wireframe matrix
fn draw_debug_grid(mut gizmos: Gizmos) {
    // Calculate the total pixel width and height of the arena
    let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
    let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

    // Offset coordinates so the grid sits perfectly in the center of our window
    let start_x = -total_width / 2.0;
    let start_y = -total_height / 2.0;

    // Draw Vertical Lines (X-axis)
    for x in 0..=ARENA_WIDTH {
        let x_pos = start_x + (x as f32 * TILE_SIZE);
        gizmos.line_2d(
            Vec2::new(x_pos, start_y),
            Vec2::new(x_pos, start_y + total_height),
            Color::DARK_GRAY,
        );
    }

    // Draw Horizontal Lines (Y-axis)
    for y in 0..=ARENA_HEIGHT {
        let y_pos = start_y + (y as f32 * TILE_SIZE);
        gizmos.line_2d(
            Vec2::new(start_x, y_pos),
            Vec2::new(start_x + total_width, y_pos),
            Color::DARK_GRAY,
        );
    }
}