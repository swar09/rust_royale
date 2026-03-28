use bevy::{app::AppExit, prelude::*};
use rust_royale_core::components::{CardUI, DragState, PlayerDeck, SpawnRequest, Team};
use rust_royale_core::constants::{ARENA_HEIGHT, ARENA_WIDTH, TILE_SIZE};
use rust_royale_core::stats::GlobalStats;

/// Spawns the 2D camera so we can actually see the world
pub fn setup_camera(mut commands: Commands, mut window_query: Query<&mut Window>) {
    let mut camera = Camera2dBundle::default();

    // Automatically scale the camera so the entire grid (plus a small margin) is ALWAYS visible.
    // This fixes clipping issues for users on smaller laptop screens like MacBooks.
    let min_width = (ARENA_WIDTH as f32 * TILE_SIZE) + 100.0;
    let min_height = (ARENA_HEIGHT as f32 * TILE_SIZE) + 100.0;

    camera.projection.scaling_mode = bevy::render::camera::ScalingMode::AutoMin {
        min_width,
        min_height,
    };

    commands.spawn(camera);

    // Maximize the window on startup
    if let Ok(mut window) = window_query.get_single_mut() {
        window.set_maximized(true);
    }
}

pub fn mouse_interaction(
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos,
    drag_state: Res<DragState>,
    global_stats: Res<GlobalStats>,
) {
    let window = window_query.single();
    let (camera, camera_transform) = camera_query.single();

    // 1. Get mouse position in world coordinates
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
    {
        let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
        let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

        let grid_x = ((world_position.x + total_width / 2.0) / TILE_SIZE) as i32;
        let grid_y = ((world_position.y + total_height / 2.0) / TILE_SIZE) as i32;

        if grid_x >= 0 && grid_x < ARENA_WIDTH as i32 && grid_y >= 0 && grid_y < ARENA_HEIGHT as i32
        {
            let pos = Vec2::new(
                (-total_width / 2.0) + (grid_x as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
                (-total_height / 2.0) + (grid_y as f32 * TILE_SIZE) + (TILE_SIZE / 2.0),
            );

            // Draw the basic hover square if dragging
            if drag_state.is_dragging {
                gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 1.05), Color::WHITE);

                // Add Hologram radius if it's a spell
                if let Some(spell_data) = global_stats.0.spells.get(&drag_state.card_key) {
                    if spell_data.radius > 0.0 {
                        // Spell radii are in tiles
                        let pixel_radius = spell_data.radius * TILE_SIZE;
                        gizmos.circle_2d(pos, pixel_radius, Color::rgba(1.0, 0.2, 0.2, 0.5));
                        
                        // Draw some internal crosshairs for aiming
                        gizmos.line_2d(pos - Vec2::X * 10.0, pos + Vec2::X * 10.0, Color::RED);
                        gizmos.line_2d(pos - Vec2::Y * 10.0, pos + Vec2::Y * 10.0, Color::RED);
                    }
                } else if let Some(_troop_data) = global_stats.0.troops.get(&drag_state.card_key) {
                    // It's a troop, show its footprint deployment area
                    // Just a simple green box to hint deployment
                    gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 1.05), Color::rgba(0.0, 1.0, 0.0, 0.5));
                }
            } else {
                // Not dragging, just a faint yellow hover
                gizmos.rect_2d(pos, 0.0, Vec2::splat(TILE_SIZE * 1.05), Color::rgba(1.0, 1.0, 0.0, 0.3));
            }
        }
    }
}

pub fn window_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
    mut window_query: Query<&mut Window>,
) {
    // Q to Close
    if keyboard_input.just_pressed(KeyCode::KeyQ) {
        exit.send(AppExit);
    }

    // Tab to Toggle Fullscreen (so you can minimize manually)
    if let Ok(mut window) = window_query.get_single_mut() {
        if keyboard_input.just_pressed(KeyCode::Tab) {
            window.mode = match window.mode {
                bevy::window::WindowMode::Windowed => bevy::window::WindowMode::Fullscreen,
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }
}

pub fn select_card_system(keyboard_input: Res<ButtonInput<KeyCode>>, mut deck: ResMut<PlayerDeck>) {
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        deck.blue_selected = Some(0);
        deck.red_selected = Some(0);
    }
    if keyboard_input.just_pressed(KeyCode::Digit2) {
        deck.blue_selected = Some(1);
        deck.red_selected = Some(1);
    }
    if keyboard_input.just_pressed(KeyCode::Digit3) {
        deck.blue_selected = Some(2);
        deck.red_selected = Some(2);
    }
    if keyboard_input.just_pressed(KeyCode::Digit4) {
        deck.blue_selected = Some(3);
        deck.red_selected = Some(3);
    }
    if keyboard_input.just_pressed(KeyCode::Escape) {
        deck.blue_selected = None;
        deck.red_selected = None;
    }
}

pub fn handle_drag_and_drop(
    buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut spawn_events: EventWriter<SpawnRequest>,
    mut deck: ResMut<PlayerDeck>,
    mut drag_state: ResMut<DragState>,
    interaction_query: Query<(&Interaction, &CardUI), Changed<Interaction>>,
) {
    // 1. Detect if player starts dragging a card from the UI bar
    for (interaction, card_ui) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && card_ui.team == Team::Blue {
            if let Some(ref card_key) = deck.blue.hand[card_ui.slot_index] {
                drag_state.is_dragging = true;
                drag_state.slot_index = card_ui.slot_index;
                drag_state.card_key = card_key.clone();
                drag_state.team = card_ui.team;
                
                // Also update legacy selection so the UI highlights
                deck.blue_selected = Some(card_ui.slot_index);
                deck.red_selected = Some(card_ui.slot_index); // For testing sandbox Red
            }
        }
    }

    // 2. Detect dropping the card or clicking the board
    let left_release = buttons.just_released(MouseButton::Left);
    let right_click = buttons.just_pressed(MouseButton::Right);

    // If they were dragging and let go, OR if they right-clicked (for Red team sandbox testing)
    if (left_release && drag_state.is_dragging) || right_click {
        let window = window_query.single();
        let (camera, camera_transform) = camera_query.single();

        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        {
            let total_width = ARENA_WIDTH as f32 * TILE_SIZE;
            let total_height = ARENA_HEIGHT as f32 * TILE_SIZE;

            // Are we dropping on the top UI or the bottom Card bar?
            // A simple check: if the mouse is below Y=120px, it's the UI bar, cancel drag.
            let mouse_y_screen = window.cursor_position().unwrap().y;
            let is_on_ui = mouse_y_screen >= window.height() - 120.0;

            let grid_x = ((world_position.x + total_width / 2.0) / TILE_SIZE) as i32;
            let grid_y = ((world_position.y + total_height / 2.0) / TILE_SIZE) as i32;

            if !is_on_ui
                && grid_x >= 0
                && grid_x < ARENA_WIDTH as i32
                && grid_y >= 0
                && grid_y < ARENA_HEIGHT as i32
            {
                // RED TEAM TESTING (Right Click Bypass)
                if right_click {
                    if let Some(sel_idx) = deck.red_selected {
                        if let Some(ref card_key) = deck.red.hand[sel_idx] {
                            spawn_events.send(SpawnRequest {
                                card_key: card_key.clone(),
                                team: Team::Red,
                                grid_x,
                                grid_y,
                            });
                        }
                    }
                } 
                // BLUE TEAM NORMAL DROP
                else if drag_state.is_dragging {
                    println!(
                        "Dropped '{}' at grid [{}, {}]",
                        drag_state.card_key, grid_x, grid_y
                    );
                    spawn_events.send(SpawnRequest {
                        card_key: drag_state.card_key.clone(),
                        team: drag_state.team,
                        grid_x,
                        grid_y,
                    });
                }
            }
        }

        // Reset drag state no matter where they let go
        drag_state.is_dragging = false;
    }
}
