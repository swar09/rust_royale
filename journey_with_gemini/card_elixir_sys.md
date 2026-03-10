1. The Match Clock & Time States (The Global Director)
The match timer is not just a UI element counting down from 3:00. It is a Global State Machine that constantly broadcasts its current phase to the rest of the engine.

In your Rust engine, the match will shift through strict states, each with its own specific rules:

State 1: PreMatch (0 to 3 seconds): The arena is locked. Elixir generates, but cards cannot be dropped. This accounts for network loading and the "Match Starting" animation.

State 2: RegularTime (First 2 minutes): Standard rules apply.

State 3: DoubleElixir (Final 1 minute): The engine intercepts the global Elixir generation rate and applies a 2.0 multiplier.

State 4: Overtime / SuddenDeath: The win condition fundamentally changes. If any tower falls, the game instantly triggers the EndMatch state.

State 5: Tiebreaker: If time runs out in Overtime, the engine pauses all unit movement and runs a rapid logic loop that drains tower health simultaneously until the tower with the lowest initial health hits zero.

2. The Elixir Economy (Continuous Math vs. Discrete UI)
Elixir looks like distinct drops (1, 2, 3...) on the screen, but inside the engine, it is a continuous fixed-point number.

The Cap: It is hard-capped at exactly 10.0.

The Generation Tick: Every single frame (60 times a second), the engine adds a tiny fraction of Elixir to both players' pools based on the Match Clock's current state.

Match State,Time to Generate 1 Elixir,Internal Engine Logic
Regular (1x),2.8 seconds,Adds ~0.0059 elixir per frame (at 60 FPS)
Double (2x),1.4 seconds,Adds ~0.0119 elixir per frame
Triple (3x),0.9 seconds,Adds ~0.0185 elixir per frame


The "Leakage" Edge Case: When a player's Elixir hits 10.0, generation stops. However, there is a hidden mechanic: if you deploy a card exactly when you hit 10, there is a tiny "server delay" buffer (about 1 second) where you technically don't waste the generation. We will need to map that buffer in our logic so high-level players aren't punished for perfect timing.


Gemini said
Stepping back from the code to map out the core game rules is a highly disciplined architectural move. If the "Matrix" is the physical body of the arena, the Clock, the Elixir, and the Deck are the nervous system.

They are all deeply interconnected. The Clock controls the Elixir rate, the Elixir restricts the Cards, and the Cards spawn the Entities.

Here is the precise engineering breakdown of how these three systems actually operate under the hood in a professional RTS engine.

1. The Match Clock & Time States (The Global Director)
The match timer is not just a UI element counting down from 3:00. It is a Global State Machine that constantly broadcasts its current phase to the rest of the engine.

In your Rust engine, the match will shift through strict states, each with its own specific rules:

State 1: PreMatch (0 to 3 seconds): The arena is locked. Elixir generates, but cards cannot be dropped. This accounts for network loading and the "Match Starting" animation.

State 2: RegularTime (First 2 minutes): Standard rules apply.

State 3: DoubleElixir (Final 1 minute): The engine intercepts the global Elixir generation rate and applies a 2.0 multiplier.

State 4: Overtime / SuddenDeath: The win condition fundamentally changes. If any tower falls, the game instantly triggers the EndMatch state.

State 5: Tiebreaker: If time runs out in Overtime, the engine pauses all unit movement and runs a rapid logic loop that drains tower health simultaneously until the tower with the lowest initial health hits zero.

2. The Elixir Economy (Continuous Math vs. Discrete UI)
Elixir looks like distinct drops (1, 2, 3...) on the screen, but inside the engine, it is a continuous fixed-point number.

The Cap: It is hard-capped at exactly 10.0.

The Generation Tick: Every single frame (60 times a second), the engine adds a tiny fraction of Elixir to both players' pools based on the Match Clock's current state.

The True Elixir Generation Rates
If you want the game to feel identical to the real thing, you use these exact generation speeds:

Match State	Time to Generate 1 Elixir	Internal Engine Logic
Regular (1x)	2.8 seconds	Adds ~0.0059 elixir per frame (at 60 FPS)
Double (2x)	1.4 seconds	Adds ~0.0119 elixir per frame
Triple (3x)	0.9 seconds	Adds ~0.0185 elixir per frame
The "Leakage" Edge Case: When a player's Elixir hits 10.0, generation stops. However, there is a hidden mechanic: if you deploy a card exactly when you hit 10, there is a tiny "server delay" buffer (about 1 second) where you technically don't waste the generation. We will need to map that buffer in our logic so high-level players aren't punished for perfect timing.

3. The Card System (The Deterministic Queue)
The deck isn't actually "random" during the match. It uses a Fixed Queue array of 8 integers (representing Card IDs), shuffled once at the very beginning of the game using a shared network seed.

Here is how the array shifts when you play the game:

The Array Layout: You have an array of 8 slots.

[Slot 0, 1, 2, 3] = Your active hand.

[Slot 4] = The "Next" card preview.

[Slot 5, 6, 7] = The bottom of the deck.

The Deployment Action: You drag the card from Slot 2 (let's say it's a Knight) onto the arena.

The Validation Gate: The engine checks: if Player.elixir >= Card.cost AND Matrix.is_deployable(x, y) == true.

The Shift: * The Elixir is subtracted.

The Knight Entity is queued to spawn.

The Knight ID is pushed to the very back of the array (Slot 7).

The "Next" card (Slot 4) moves into your active hand.

Everything else shifts forward by one.
