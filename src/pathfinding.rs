use crate::arena::{ArenaGrid, TileType};
use crate::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use std::collections::{BinaryHeap, HashMap};

// A helper struct for our Priority Queue to sort tiles by lowest cost
#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    cost: i32,
    pos: (i32, i32),
}

// We implement custom ordering so the BinaryHeap pops the LOWEST cost first
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.pos.cmp(&other.pos))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Calculates the Manhattan distance between two points (our heuristic)
fn heuristic(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

/// Returns a list of grid coordinates forming the shortest path
pub fn calculate_a_star(
    grid: &ArenaGrid,
    start: (i32, i32),
    goal: (i32, i32),
    is_flying: bool,
    attack_range_tiles: i32,
) -> Option<Vec<(i32, i32)>> {
    let mut frontier = BinaryHeap::new();
    frontier.push(Node {
        cost: 0,
        pos: start,
    });

    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut cost_so_far: HashMap<(i32, i32), i32> = HashMap::new();

    cost_so_far.insert(start, 0);

    let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];

    let mut best_node = goal;

    while let Some(current) = frontier.pop() {
        // Stop if we are within range of the goal
        let dist = heuristic(current.pos, goal);
        if dist <= attack_range_tiles {
            best_node = current.pos;
            break;
        }

        for (dx, dy) in directions.iter() {
            let next_x = current.pos.0 + dx;
            let next_y = current.pos.1 + dy;
            let next = (next_x, next_y);

            // 1. Boundary Check
            if next_x < 0
                || next_x >= ARENA_WIDTH as i32
                || next_y < 0
                || next_y >= ARENA_HEIGHT as i32
            {
                continue;
            }

            // 2. Obstacle Check (Using the exact same logic we built in the last step!)
            let tile_index = (next_y * ARENA_WIDTH as i32 + next_x) as usize;
            let tile = &grid.tiles[tile_index];

            let can_walk = match tile {
                TileType::River => is_flying,
                TileType::Tower => false,
                _ => true,
            };

            if !can_walk {
                continue;
            }

            // Diagonal moves cost slightly more (14 vs 10) to prevent weird zig-zagging
            let step_cost = if dx.abs() == 1 && dy.abs() == 1 {
                14
            } else {
                10
            };
            let new_cost = cost_so_far.get(&current.pos).unwrap() + step_cost;

            if !cost_so_far.contains_key(&next) || new_cost < *cost_so_far.get(&next).unwrap() {
                cost_so_far.insert(next, new_cost);
                let priority = new_cost + heuristic(next, goal) * 10;
                frontier.push(Node {
                    cost: priority,
                    pos: next,
                });
                came_from.insert(next, current.pos);
            }
        }
    }

    // Trace the path backward from the best node to the start
    let mut path = Vec::new();
    let mut current = best_node;

    if current != start && !came_from.contains_key(&current) {
        return None; // No path exists!
    }

    while current != start {
        path.push(current);
        current = *came_from.get(&current).unwrap();
    }

    // We want the path from start -> end, so we reverse it
    path.reverse();
    Some(path)
}
