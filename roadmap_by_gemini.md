current roadmap
Phase 1: The Combat Loop (Making it Violent)
Right now, your Knights are pacifists who walk off the edge of the board. We need them to fight.

Targeting AI: A system where units scan for the closest enemy and stop walking when Distance <= Range.

Combat System: A timer-based system reading hit_speed_ms to swing swords and subtract Health.

Death & Cleanup: A system that despawns entities when their health reaches 0.

Phase 2: The Match Rules (Making it a Game)
Right now, the game never starts and never ends.

Towers: Spawning the King and Princess towers at their exact fixed-point coordinates on Startup.

Match Clock & Phases: A global timer that handles the 3-minute match, triggers 2x Elixir at the 2:00 mark, and initiates Overtime.

Win Condition: Halting the game and declaring a winner when a King Tower falls or the clock runs out.

Phase 3: The Card System (Making it a Deck)
Right now, you are hardcoding the "knight" into your mouse clicks.

The Queue Array: Implementing the strict 8-slot array to hold your deck.

The Hand: Drawing 4 random cards into an active hand that you can select from.

The Rotation: Shifting the array and pulling a new card when you spend Elixir to play a troop.

Phase 4: Advanced Engine Mechanics (Making it Professional)
The Spatial Hash Map: Upgrading our combat to use the grid "buckets" so the engine can handle 100 units without dropping frame rates.

Spells & AoE: Handling cards like the Fireball that don't walk, but instead spawn, deal radius damage, and instantly despawn.

Dynamic Deployment Zones: Repainting the red/blue grid validation when an enemy Princess tower is destroyed.

Phase 5: Multiplayer (Making it 1v1)
Rollback Netcode: Integrating ggrs to sync Player 1 and Player 2 inputs across the internet deterministically.