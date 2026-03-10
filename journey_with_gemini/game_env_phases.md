Phase 1: The "Architect's Blueprint" (Debug UI)
Before we load any 2D sprites or 3D models, we need to prove that our 18x32 math is working. We do this using Gizmos (Bevy's built-in tool for drawing simple geometric lines).

Draw the Grid: We write a System that loops through our ArenaGrid and draws an empty square for every tile.

Color-Code the Logic: * If is_walkable == false (The River), the system fills the square with Blue.

If a Princess Tower occupies the tile, it fills it with Red.

The Bridges are filled with Green.

The Result: When you launch the game, you won't see a grassy arena. You will see a stark, colorful wireframe matrix. This is your ultimate truth. If a unit walks on Blue, you immediately know your physics engine is broken.

Phase 2: The "Hand of God" (Mouse-to-Matrix UI)
Next, we need the game to understand what your mouse is doing. Your screen operates in standard pixels, but your game operates in our 18x32 Fixed-Point Matrix.

Raycasting System: We build a System that shoots an invisible laser from your mouse cursor into the 2D/3D world.

The Translation: It takes the screen pixel (e.g., X: 1920, Y: 1080) and translates it into a Fixed-Point Coordinate (e.g., X: 15000, Y: 10000).

The Hover Highlight: We divide that coordinate by 1000 to find the Matrix Tile (15, 10). We tell the Gizmo UI to draw a bright yellow box around tile (15, 10).
Now, as you move your mouse, a yellow square perfectly snaps from tile to tile across your grid.

Phase 3: The "Inspector" (Dev GUI)
Because we are tweaking complex edge cases (like unit mass, movement speed, and exact hit radiuses), we do not want to stop and recompile the Rust code every time we change a number.

The Tool: We plug in a crate called bevy_egui.

What it does: It creates a highly technical, floating window inside your game (it looks a bit like the TUI interfaces you like, but with sliders and checkboxes).

The Goal: You can click on the "Test Unit," and the Egui window pops up showing its internal Fixed-Point coordinates, its current State (Idle/Walking), and its Health. You can use a slider to change its speed from 1500 to 3000 while the game is running to see what happens.

Phase 4: The "Player Experience" (The Actual HUD)
Only once the invisible math is fully visible and interactive do we build the actual player UI.

Bevy uses a UI system heavily inspired by CSS Flexbox.

The Canvas: We create a UI Entity called the RootNode that covers the whole screen.

The Elixir Bar: We anchor a rectangular Node to the bottom center. We give it a FillPercentage component that is tied to your PlayerState.elixir value.

The Deck (Flexbox): We create a horizontal row (Flex Row) and spawn 4 "Card" UI Entities inside it.

The Interaction: When you click a Card UI Entity, it checks your PlayerState.elixir. If you have enough, it changes your mouse into the "Hover Highlight" mode we built in Phase 2.