# 🏰 Rust Royale — Full Project Postmortem & Roadmap to Production

> **Audit Date:** March 25, 2025  
> **Codebase Size:** ~3,000 lines of Rust across 16 source files  
> **Engine:** Bevy 0.13.0 (ECS Architecture)  
> **Architecture:** Workspace monorepo (`core` / `engine` / `app` / `sandbox`)

---

## 📊 Executive Summary

Rust Royale is a **Clash Royale clone** built in Rust/Bevy. The project has made impressive progress on the **core simulation layer** — combat, pathfinding, collision physics, match flow, and deck management all exist and function. However, it is currently at the stage of a **working prototype / tech demo**, not a shippable game.

### What's Working Well ✅
| System | Status | Quality |
|--------|--------|---------|
| ECS Architecture | ✅ Complete | ⭐⭐⭐⭐⭐ Clean workspace separation |
| Data-Driven Stats (JSON) | ✅ Complete | ⭐⭐⭐⭐ 11 troops, 3 spells, 2 buildings |
| Arena Grid (20×32) | ✅ Complete | ⭐⭐⭐⭐ Walls, river, bridges, towers |
| A* Pathfinding | ✅ Complete | ⭐⭐⭐⭐ Diagonal movement, bridge routing |
| Targeting System | ✅ Complete | ⭐⭐⭐⭐ Air/ground, lane-locking, stickiness |
| Combat (Melee + Ranged) | ✅ Complete | ⭐⭐⭐ Projectiles exist but single-target only |
| Collision Physics | ✅ Complete | ⭐⭐⭐ Mass-based pushing, bridge deadlock fix |
| Match Manager | ✅ Complete | ⭐⭐⭐⭐ 3-min clock, double elixir, overtime, tiebreaker |
| Spell System (AoE) | ✅ Complete | ⭐⭐⭐ Fireball/Arrows with knockback + waves |
| Deck & Card Rotation | ✅ Complete | ⭐⭐⭐ 8-card deck, 4-card hand, rotation |
| Death Spawns | ✅ Complete | ⭐⭐⭐ Golem → Golemites works |
| Deployment Zones | ✅ Complete | ⭐⭐⭐ Dynamic expansion on princess tower death |
| Tower Waking (King) | ✅ Complete | ⭐⭐⭐⭐ Princess death or direct hit wakes king |

### What's Missing ❌
| System | Impact | Difficulty |
|--------|--------|------------|
| Splash Damage (Troops) | 🔴 Critical | Medium |
| AI Opponent | 🔴 Critical | Hard |
| Visual Assets / Sprites | 🔴 Critical | Medium |
| Sound Effects / Music | 🔴 Critical | Medium |
| UI/UX (Card Bar, Elixir Bar) | 🔴 Critical | Hard |
| Fixed Timestep | 🟡 High | Easy |
| Spawner Spells (Goblin Barrel) | 🟡 High | Medium |
| Building Cards (Tesla, Inferno) | 🟡 High | Hard |
| Level/Card Upgrade System | 🟡 High | Medium |
| Menus / Game States | 🟡 High | Medium |
| Multiplayer / Netcode | 🟡 High | Very Hard |
| Replay System | 🟢 Medium | Medium |
| Troop Animations | 🟢 Medium | Medium |
| Particle Effects | 🟢 Medium | Medium |
| Min-Range Attacks (Mortar) | 🟢 Low | Easy |

---

## 🔍 Deep Dive: Game Logic Audit

### 1. Combat System ([combat.rs](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs)) — 634 lines

**Strengths:**
- Euclidean edge-to-edge distance (center dist minus both radii) — this is the correct Clash Royale formula
- Lane-based target filtering prevents cross-lane sniping
- Target stickiness with re-acquire detection avoids path thrashing
- Projectile entities with travel time (not instant damage)
- King tower waking on princess death or direct damage hit

**Bugs & Issues Found:**

> [!WARNING]
> **BUG 1: Splash damage is NOT implemented for troops**
> Valkyrie and Baby Dragon have `splash_radius` and `splash_type` in JSON but [combat_damage_system](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#217-295) (line 217-293) only fires a single projectile at the current target. There is **zero code** to deal AoE damage to nearby enemies when the projectile hits. This makes Valkyrie functionally identical to a Knight with different stats.

> [!WARNING]
> **BUG 2: Projectile speed is hardcoded**
> Line 277: `speed: 6000` — every projectile moves at the same speed. Tower arrows, musketeer bullets, and melee "swing" projectiles all travel identically. Real Clash Royale has different projectile speeds per unit.

> [!WARNING]
> **BUG 3: Massive code duplication in tower death handling**
> The tower destruction + crown counting + king waking logic is copy-pasted in THREE places:
> - [projectile_flight_system](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#296-460) (lines 337-458)
> - [spell_impact_system](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#461-634) (lines 461-632)
> - [match_manager_system](file:///Users/parthagrawal99/rust_royale/engine/src/systems/match_manager.rs#6-127) (lines 60-104)
> 
> This is a maintenance nightmare — any fix must be applied 3 times.

> [!IMPORTANT]
> **BUG 4: `despawn()` vs `despawn_recursive()`**
> Line 361: `commands.entity(target_ent_inner).despawn()` — this despawns only the parent entity, **orphaning** the child [HealthValueText](file:///Users/parthagrawal99/rust_royale/core/src/components.rs#17-18) entity. Should be `despawn_recursive()` everywhere. This causes ghost text labels floating on screen after units die.

---

### 2. Movement System ([movement.rs](file:///Users/parthagrawal99/rust_royale/engine/src/systems/movement.rs)) — 613 lines

**Strengths:**
- A* pathfinding with GPS waypoint following
- Straight-line fallback for close-range approach
- River detection to force bridge routing
- Smart lane-aware king tower approach (left vs right side)
- Wall sliding when blocked diagonally
- Emergency escape from impassable tiles

**Issues Found:**

> [!WARNING]
> **PERF: `path.0.remove(0)` is O(n)**
> Lines 182, 315: Removing the first element of a `Vec` shifts all remaining elements. With 30+ waypoints this is measurable. Should use `VecDeque` or reverse the path and `pop()`.

> [!IMPORTANT]
> **BUG 5: No system ordering guarantees**
> All systems run in `Update` with no explicit ordering. Movement can run before targeting, causing one frame where troops move on stale target data. Bevy processes unordered systems in parallel/arbitrary order.

> [!NOTE]
> **Missing: Retargeting after path completion**
> When a troop reaches the end of its path but the target moved (e.g. was pushed by collision), the troop stands still for one frame before recalculating. In fast-paced matches this creates visible "stutter steps."

---

### 3. Spawning System ([spawning.rs](file:///Users/parthagrawal99/rust_royale/engine/src/systems/spawning.rs)) — 566 lines

**Strengths:**
- Full deployment zone validation (terrain + ownership + dynamic pocket expansion)
- Elixir cost validation and deduction
- Deck rotation on card play
- Multi-unit spawning with offset positioning (Archers × 2, Skarmy × 15)
- Death spawn inheritance of parent lane

**Issues Found:**

> [!WARNING]
> **BUG 6: Spawner spells (Goblin Barrel) don't work**
> The `goblin_barrel` spell in stats.json has `spell_type: "spawner"` with `spawns_troop_id: 201` and `spawn_count: 3`. But [spawn_entity_system](file:///Users/parthagrawal99/rust_royale/engine/src/systems/spawning.rs#12-342) only handles the `"damage"` spell path — it creates an [AoEPayload](file:///Users/parthagrawal99/rust_royale/core/src/components.rs#162-170) with damage=0 for spawner spells. There's **no code** to actually spawn the goblins.

> [!IMPORTANT]
> **BUG 7: Deck rotation is broken for Red team**
> In the spell spawning path (lines 278-295), deck rotation uses `deck.selected_index` but this index is shared between both teams. If Red plays a spell, it rotates Red's deck correctly, but `deck.selected_index` was set by the Left/Right click handler which ALWAYS sets Blue's selection. The Red team's deck can get out of sync.

> [!NOTE]
> **Missing: Tower footprint validation for multi-tile spawns**
> Golem has `footprint_x: 2`, meaning it occupies a 2×2 area. But spawning only validates the single deployment tile, not the footprint. A Golem deployed at the edge of a bridge could overlap into the river.

---

### 4. Pathfinding ([pathfinding.rs](file:///Users/parthagrawal99/rust_royale/core/src/pathfinding.rs)) — 141 lines

**Strengths:**
- Clean A* with Manhattan heuristic
- Proper diagonal cost weighting (14 vs 10)
- Flying bypass for river tiles
- Early termination at attack range

**Issues Found:**

> [!NOTE]
> **Missing: Tower tiles are totally impassable**
> Line 94: Towers block pathfinding identically to walls. In real Clash Royale, troops should pathfind TO a tower's edge, not around it. This works because the targeting system does straight-line approach once close, but it's fragile.

> [!IMPORTANT]
> **PERF: No path caching**
> A* recalculates from scratch every time `path.0.is_empty()`. For 15 skeletons targeting the same tower, that's 15 identical A* runs per path-clear. A simple cache keyed on [(start_grid, goal_grid)](file:///Users/parthagrawal99/rust_royale/core/src/pathfinding.rs#14-20) would save significant CPU.

---

### 5. Match Manager ([match_manager.rs](file:///Users/parthagrawal99/rust_royale/engine/src/systems/match_manager.rs)) — 127 lines

**Strengths:**
- Correct phase transitions (Regular → Double Elixir → Overtime → GameOver)
- Overtime tiebreaker with lowest-HP tower destruction
- Proper elixir generation rates (1/2.8s base, 2x in double/overtime)

**Issues Found:**

> [!WARNING]
> **BUG 8: Tiebreaker logic is wrong**
> Lines 41-51: The tiebreaker finds the global minimum HP across ALL towers (both teams). If Blue has a 100 HP princess tower and Red has all towers at full health, the logic correctly destroys Blue's tower. **But** if BOTH teams have a tower at the same minimum HP, both get destroyed simultaneously. In real Clash Royale, the tiebreaker compares the **percentage** HP of the lowest-HP tower per team.

> [!IMPORTANT]
> **BUG 9: Elixir still generates during GameOver frame**
> The `multiplier` for GameOver is 0.0 (line 118), but the [match_manager_system](file:///Users/parthagrawal99/rust_royale/engine/src/systems/match_manager.rs#6-127) early-returns at line 13 before reaching the elixir code. This is fine, but the system still ticks MatchState on the final frame before GameOver is set, potentially giving a tiny elixir boost.

---

### 6. Arena & Data Layer

**Arena** ([arena.rs](file:///Users/parthagrawal99/rust_royale/core/src/arena.rs)):
- Clean 20×32 grid with proper wall placement
- Tower clearance on destruction updates pathfinding in real time
- ✅ No bugs found

**Stats** ([stats.json](file:///Users/parthagrawal99/rust_royale/assets/stats.json)):
- 11 troops defined: Knight, Valkyrie, Baby Dragon, Archer, Skeleton Army, Giant, Musketeer, Golem, Golemite, Mini P.E.K.K.A., Minions
- 3 spells: Fireball, Goblin Barrel, Arrows
- 2 buildings: Princess Tower, King Tower

> [!NOTE]
> **Missing troops for a real game:** Hog Rider, Wizard, Witch, Barbarians, P.E.K.K.A, Electro Wizard, Mega Knight, Lumberjack, Log, Zap, Poison, Rage, Tornado, Balloon, Inferno Tower, Tesla, Bomb Tower, Furnace, etc. A real Clash Royale has 100+ cards.

---

## 🏗️ Architecture Assessment

### What's Good
```
rust_royale/
├── core/       ← Pure data types, zero game logic (PERFECT separation)
├── engine/     ← All ECS systems (GOOD, but needs ordering)
├── app/        ← Production binary
├── sandbox/    ← Test harness binary
└── assets/     ← JSON data files
```

### What Needs Work

| Problem | Details |
|---------|---------|
| **No system ordering** | All 14 systems run unordered in `Update`. Should use `.chain()` or explicit `.before()`/`.after()` |
| **No `FixedUpdate`** | All game logic uses `time.delta_seconds()` which varies per frame. Physics and combat will behave differently at 30fps vs 144fps |
| **No game states** | No `MainMenu`, `Playing`, `Paused`, `GameOver` state machine. The game jumps directly into gameplay |
| **No error handling** | `unwrap()` calls throughout (e.g., [stats.json](file:///Users/parthagrawal99/rust_royale/assets/stats.json) loading, tower data lookups) |
| **No tests** | The `tests/` directory is empty (only [.gitkeep](file:///Users/parthagrawal99/rust_royale/tests/.gitkeep)). Zero unit tests, zero integration tests |
| **App ≡ Sandbox** | [app/src/main.rs](file:///Users/parthagrawal99/rust_royale/app/src/main.rs) and [sandbox/src/main.rs](file:///Users/parthagrawal99/rust_royale/sandbox/src/main.rs) are nearly identical (~95% same code) |

---

## 🛣️ Roadmap: From Prototype to Real Game

### 🔴 Phase 1: Fix Critical Bugs (1-2 days)
Priority fixes that affect gameplay correctness:

- [ ] **1.1** Replace all `despawn()` with `despawn_recursive()` to fix orphaned child entities
- [ ] **1.2** Implement splash damage for Valkyrie (`self_centered`) and Baby Dragon (`target_centered`)
- [ ] **1.3** Fix tiebreaker to compare per-team lowest HP percentage, not global minimum
- [ ] **1.4** Add per-unit projectile speed to [AttackStats](file:///Users/parthagrawal99/rust_royale/core/src/components.rs#118-124) and [stats.json](file:///Users/parthagrawal99/rust_royale/assets/stats.json)
- [ ] **1.5** Implement Goblin Barrel spawner spell logic
- [ ] **1.6** Extract tower death/crown handling into a shared function (eliminate 3x duplication)

---

### 🟠 Phase 2: Engine Correctness (2-3 days)
Make the simulation deterministic and correct:

- [ ] **2.1** Migrate ALL game logic from `Update` to `FixedUpdate` with `FixedTime::new_from_secs(1.0/60.0)`
- [ ] **2.2** Add explicit system ordering:
  ```
  input → spawn → deploy → target → combat → projectiles → spells → movement → collision → death_spawns → ui
  ```
- [ ] **2.3** Replace `Vec<(i32,i32)>` waypoints with `VecDeque` for O(1) front removal
- [ ] **2.4** Add path caching for A* (key: start+goal grid coords)
- [ ] **2.5** Add game state machine (`AppState::Menu`, `Playing`, `Paused`, `GameOver`)
- [ ] **2.6** Fix Red team deck rotation bug (separate selection index per team)
- [ ] **2.7** Validate multi-tile unit footprint on spawn (Golem 2×2 check)

---

### 🟡 Phase 3: Visual Polish (1-2 weeks)
Transform from colored squares to a visual game:

- [ ] **3.1** Replace `Sprite` colored squares with actual sprite sheets / pixel art for each troop
- [ ] **3.2** Add troop animations (idle, walk, attack, death) using Bevy's `AnimationPlayer`
- [ ] **3.3** Build proper UI card bar at screen bottom with:
  - Card art thumbnails
  - Elixir cost badge
  - Selection highlight
  - Next card preview
- [ ] **3.4** Animated elixir bar (purple fill with glow at 10/10)
- [ ] **3.5** Tower health bars (segmented, Clash Royale style)
- [ ] **3.6** Particle effects: deploy dust cloud, death explosion, projectile trails
- [ ] **3.7** Arena background texture (grass, river water animation, bridge planks)
- [ ] **3.8** Crown counter UI with star animations on tower kill
- [ ] **3.9** Match timer display (centered top, enlarges in overtime)
- [ ] **3.10** "DOUBLE ELIXIR" / "OVERTIME" banner announcements

---

### 🟢 Phase 4: Audio (3-5 days)
A game without sound feels lifeless:

- [ ] **4.1** Background music (battle theme, overtime remix)
- [ ] **4.2** Unit deploy SFX (unique per troop)
- [ ] **4.3** Attack SFX (sword swing, arrow fire, spell impact)
- [ ] **4.4** Tower destruction SFX + screen shake
- [ ] **4.5** UI SFX (card select, insufficient elixir "buzzer")
- [ ] **4.6** Elixir full notification sound
- [ ] **4.7** Match start countdown (3, 2, 1, FIGHT!)

---

### 🔵 Phase 5: AI Opponent (1-2 weeks)
Currently there's no way to play a real game — both teams are human-controlled:

- [ ] **5.1** Basic AI: Random card placement at fixed intervals
- [ ] **5.2** Reactive AI: Counter-deploy (if enemy places Giant, AI places Skeleton Army)
- [ ] **5.3** Elixir-aware AI: Don't play 8-cost Golem with only 3 elixir
- [ ] **5.4** Positional AI: Place tanks in front, ranged behind
- [ ] **5.5** Timing AI: Save elixir for counter-pushes, deploy at 2x elixir phase
- [ ] **5.6** Difficulty levels (Easy/Medium/Hard reaction times)

---

### 🟣 Phase 6: Content Expansion (Ongoing)
Adding the breadth that makes Clash Royale addictive:

- [ ] **6.1** Building cards (Tesla, Inferno Tower, Bomb Tower, Furnace)
  - Needs new `is_building: true` troop type with lifetime timer
  - Inferno Tower: ramping damage mechanic
- [ ] **6.2** More spells (Zap, Log, Poison, Rage, Tornado, Freeze)
  - Poison: damage over time component
  - Rage: speed buff component  
  - Tornado: pull mechanic
  - Log: moving hitbox
- [ ] **6.3** More troops (Hog Rider, Wizard, Witch, Barbarians, Mega Knight, Balloon)
  - Witch: periodic skeleton spawning
  - Hog Rider: building-targeting + jump-over-river mechanic
  - Balloon: flying building-targeting
- [ ] **6.4** Card level/upgrade system (`level: 1-14` stat multipliers)
- [ ] **6.5** Deck builder screen (choose 8 from collection)

---

### ⚫ Phase 7: Game Modes & Menus (1-2 weeks)
- [ ] **7.1** Main menu screen (Play, Collection, Settings)
- [ ] **7.2** Match result screen (win/loss animation, crown display)
- [ ] **7.3** Pause functionality
- [ ] **7.4** Settings (volume, speed toggle)
- [ ] **7.5** Training mode against AI bots
- [ ] **7.6** 2v2 mode (4 king towers, shared lanes)

---

### ⚪ Phase 8: Multiplayer (2-4 weeks)
The ultimate goal to make this a real competitive game:

- [ ] **8.1** Deterministic fixed-timestep must be PERFECT (Phase 2.1)
- [ ] **8.2** Input serialization (only sync card plays, not full state)
- [ ] **8.3** GGRS rollback netcode integration
- [ ] **8.4** Matchmaking server (lobby system)
- [ ] **8.5** Spectator mode
- [ ] **8.6** Replay save/load (just replay inputs at fixed timestep)

---

## 🐛 Complete Bug List

| # | Location | Severity | Description |
|---|----------|----------|-------------|
| 1 | [combat.rs:217-293](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#L217-L293) | 🔴 Critical | Splash damage not implemented (Valkyrie/Baby Dragon) |
| 2 | [combat.rs:277](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#L277) | 🟡 Medium | Hardcoded projectile speed (6000) for all units |
| 3 | [combat.rs:337-458](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#L337-L458) | 🟡 Medium | Tower death logic duplicated 3 times across files |
| 4 | [combat.rs:361](file:///Users/parthagrawal99/rust_royale/engine/src/systems/combat.rs#L361) | 🔴 Critical | `despawn()` orphans child text entities (should be `despawn_recursive()`) |
| 5 | [app/main.rs:42-65](file:///Users/parthagrawal99/rust_royale/app/src/main.rs#L42-L65) | 🟡 Medium | No system ordering — race conditions between systems |
| 6 | [spawning.rs:258-333](file:///Users/parthagrawal99/rust_royale/engine/src/systems/spawning.rs#L258-L333) | 🟡 Medium | Spawner spells (Goblin Barrel) create AoE instead of troops |
| 7 | [spawning.rs:278-295](file:///Users/parthagrawal99/rust_royale/engine/src/systems/spawning.rs#L278-L295) | 🟡 Medium | Red team deck rotation uses shared `selected_index` |
| 8 | [match_manager.rs:38-51](file:///Users/parthagrawal99/rust_royale/engine/src/systems/match_manager.rs#L38-L51) | 🟡 Medium | Tiebreaker uses global min HP, not per-team percentage |
| 9 | [movement.rs:182](file:///Users/parthagrawal99/rust_royale/engine/src/systems/movement.rs#L182) | 🟢 Low | `Vec::remove(0)` is O(n), should use VecDeque |
| 10 | All files | 🟡 Medium | No `FixedUpdate` — simulation is framerate-dependent |

---

## 📈 Scoring: How Close to "Real Game"?

| Category | Score | Notes |
|----------|-------|-------|
| **Core Simulation** | 7/10 | Combat, pathfinding, physics all work. Missing splash, spawner spells |
| **Game Rules** | 8/10 | Match flow, crowns, overtime, tiebreaker all implemented. Minor bugs |
| **Card Variety** | 3/10 | 11 troops, 3 spells. Real CR has 100+ cards |
| **Visuals** | 1/10 | Colored squares on wireframe grid. No sprites, no animations |
| **Audio** | 0/10 | Complete silence |
| **UI/UX** | 2/10 | Text-only HUD, no card bar, no menus |
| **AI** | 0/10 | No opponent — both players are human |
| **Multiplayer** | 0/10 | Not started |
| **Polish** | 1/10 | No particles, screen shake, announcements |
| **Overall** | **2.5/10** | Strong engine, but needs everything around it |

---

## 🎯 Recommended Next Steps (Priority Order)

1. **Fix `despawn_recursive()` bug** — 5 minutes, prevents ghost text
2. **Add system ordering** — 30 minutes, prevents race condition bugs  
3. **Migrate to `FixedUpdate`** — 1 hour, makes physics deterministic
4. **Implement splash damage** — 2 hours, makes Valkyrie/BD actually work
5. **Build a basic AI opponent** — makes the game actually playable solo
6. **Replace squares with sprites** — makes it look like a game

> [!TIP]
> The engine foundation is **solid**. The ECS architecture, data-driven stats, and workspace structure are all well-designed. The biggest gap isn't code quality — it's that the game has no **visual identity** and no **opponent to play against**. Focus on AI + sprites before adding more cards.
