World Grid & Coordinate System
1. The Dual-System Architecture
The engine operates on two parallel systems that communicate every frame:

The Discrete Matrix (World Rules): An 18 x 32 grid of static tiles. This dictates where troops can be placed, where the river flows, and how pathfinding (A*) navigates around obstacles.

The Continuous Physics (Movement & Collisions): A fixed-point coordinate system where troops exist as points with circular collision radiuses. Troops glide continuously over the matrix, resolving physical bumps and pushes independent of the rigid grid.

2. The Arena Matrix (Discrete Grid)
The arena is exactly 18 tiles wide by 32 tiles long (576 total tiles).
The coordinate origin (0, 0) is the bottom-left corner of Player 1's side.

Key Landmarks & Dimensions:

Player Territory: Y = 0 to Y = 14.

The River: 2 tiles thick. Occupies Y = 15 and Y = 16. (is_walkable: false).

The Bridges: 3 tiles wide by 2 tiles long. They override the river at specific X-coordinates to make it walkable.

King Tower (4x4): Placed at Y = 1 through Y = 4 in the center.

Note: Row Y = 0 (behind the King Tower) remains is_walkable: true and is_deployable: true to allow banking heavy troops.

Princess Towers (3x3): Placed dynamically near the bridges on both sides.

3. Fixed-Point Arithmetic (Multiplayer Sync)
To guarantee deterministic logic across different PCs, the engine strictly forbids standard floating-point numbers (f32 / f64) for gameplay logic.

All continuous positions, distances, and speeds are stored as integers using a multiplier of 1000.

1.0 Tile = 1000 internal units.

Speed of 1.5 tiles/sec = 1500 internal units.

A troop at exactly X: 5.5, Y: 10.2 is stored in memory as X: 5500, Y: 10200.