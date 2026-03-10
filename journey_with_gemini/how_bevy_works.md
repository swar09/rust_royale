Bevy uses an architecture called ECS (Entity Component System). Here is exactly how it works and how we will build the UI on top of it.

Part 1: How Bevy Works (The ECS Architecture)
Imagine Bevy as a massive, ultra-fast database running at 60 frames per second.

Instead of objects, everything in your game is divided into three distinct buckets:

1. Entities (The "ID Cards")
An Entity is literally just a number. It has no data, no logic, and no graphics. It is just an empty ID card (e.g., Entity #104). That’s it.

A Knight is an Entity.

A Fireball is an Entity.

The "Start Game" button is an Entity.

2. Components (The "Post-it Notes")
Components are pure data. They have no logic. You take these data "notes" and stick them onto your Entities.

If you take Entity #104 and slap a Position(x, 15, y: 10) note and a Health(1000) note on it, it physically exists in the world.

If you slap a Sprite("knight.png") note on it, it becomes visible.

If you slap a PlayerControl note on it, it becomes yours.

3. Systems (The "Workers")
Systems are pure logic (functions). They have no data of their own. Every frame, Systems run through your database looking for specific combinations of Post-it notes.

The Movement System: Says, "Give me every Entity that has a Position AND a Velocity. I don't care if it's a Knight or a Fireball. I will update their Position."

The Damage System: Says, "Give me every Entity that has Health AND was hit by an AttackEvent. I will subtract the math."

Why this is genius for our game: If you want a Freeze Spell to work, you don't have to write complex logic to pause the Knight, the Golem, and the Tower. You just write a System that temporarily removes their Velocity component. Without a Velocity note, the Movement System simply ignores them. They freeze instantly.

Part 2: How We Will Make the UI
In Bevy, the UI is not some separate magical layer. UI elements are just Entities with specific UI Components (like Button, Text, or Color) that the engine knows to draw on the screen instead of in the game world.

To build the Clash Royale interface, we will split the UI into two completely different layers:

Layer A: "Screen-Space" UI (The HUD)
This is the UI that sticks to your monitor and never moves, even if the camera moves.

What it includes: Your Elixir Bar, the 4 cards in your hand, the match timer, and the "Next Card" preview.

How it works: When you click a card in your hand, a System detects the click on that UI Entity. It changes your state to "Deploying_Mode" and attaches a ghost image of the card to your mouse cursor.

Layer B: "World-Space" UI (The Arena Overlay)
This is the UI that exists inside the matrix.

What it includes: The red and blue health bars floating above the troops, the white grid that appears when you drag a building, and the red "invalid placement" tint.

How it works: When you are in "Deploying_Mode", a System looks at your mouse's continuous float coordinates, divides by 1000 (our fixed-point math!), and figures out which Matrix tile you are hovering over. It then highlights that specific tile on the ground.

Part 3: The Full Flow (From UI to ECS)
Here is how the UI and the Bevy ECS talk to each other when you play the game:

The Click (UI): You drag the "Prince" card from your Screen-Space HUD.

The Hover (World UI): You hover over Tile (10, 5). The engine checks the Matrix. is_deployable is True. It highlights the tiles blue.

The Drop (System Event): You let go of the mouse. The UI sends a message: "Spawn a Prince at Fixed-Point (10000, 5000)."

The Spawn (Entity Creation): The Spawner System creates Entity #402. It slaps on Health, Position, Velocity, Sprite("prince.png"), and ChargeAbility components.

The Engine Takes Over: On the very next tick, the Movement System sees Entity #402 has a Position and Velocity, and begins moving it toward the enemy tower.