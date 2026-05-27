# Simulation

> World structure, spatial system, resource dynamics, and systemic pressures.

---

## World Structure

The simulation world is a **bounded flat plane** with a configurable size (default: 100×100 units). The world has no terrain height at MVP — agents move in the XZ plane, Y axis is visual only.

```
World (100 × 100 units)
┌──────────────────────────────────────────────────┐
│  ╔══════╗        ╔═════════╗      ╔══════╗       │
│  ║ Zone ║        ║Resource ║      ║ Zone ║       │
│  ║(Rest)║        ║  Zone   ║      ║(Rest)║       │
│  ╚══════╝        ╚═════════╝      ╚══════╝       │
│                                                   │
│         [agents]       [agents]                  │
│                                                   │
│  ╔═════════╗      ╔══════╗      ╔══════════╗     │
│  ║Resource ║      ║Neutral║     ║ Resource ║     │
│  ║  Zone   ║      ║  Zone ║     ║  Zone    ║     │
│  ╚═════════╝      ╚══════╝     ╚══════════╝     │
└──────────────────────────────────────────────────┘
```

### Zone Types

| Zone | Description | Agent Effect |
|---|---|---|
| `Resource` | Contains harvestable `ResourceNode` entities | Agents find food/resources here |
| `Rest` | Open flat area, no competition | Fatigue recovery rate ×1.5 |
| `Neutral` | Default passable space | No modifier |
| `Hazard` *(future)* | Dangerous area | Causes energy drain |

Zones are defined in scenario config as `(center: Vec3, radius: f32, kind: ZoneKind)`. They overlap with the entity system — a zone is an entity with `Zone` + `Transform` components.

---

## Agent Lifecycle

```
Spawn (from ScenarioConfig)
    │
    ▼
Idle (initial state)
    │
    ▼
┌─────────────────────────────────────────────────────┐
│                  Active Simulation Loop              │
│                                                     │
│  Needs degrade → AI evaluates → Agent acts          │
│                                                     │
│  States: Idle ↔ MovingToTarget ↔ Eating/Resting     │
│                      ↕                              │
│                  Exploring                          │
└─────────────────────────────────────────────────────┘
    │
    ▼
Death (any Need ≥ 1.0 critical threshold)
    │
    ▼
AgentDied event → Entity despawned after death animation
```

### Needs Degradation

All needs degrade continuously at configurable rates (per simulation second):

```rust
pub struct NeedsDecayRates {
    pub hunger_per_sec: f32,   // default: 0.02  (50s to critical from 0)
    pub fatigue_per_sec: f32,  // default: 0.015 (67s to critical)
    pub energy_per_sec: f32,   // default: 0.01  (100s to critical)
}
```

Decay rates are **multiplied by a global scalar** from `SimulationConfig.global_decay_multiplier` — scenarios can make needs decay faster/slower without editing base rates.

### Need Recovery

| Need | Recovery Action | Rate |
|---|---|---|
| Hunger | Eating at ResourceNode | Drain resource, restore hunger proportionally |
| Fatigue | Resting in Rest zone | +0.03/s while in Rest zone, +0.01/s anywhere |
| Energy | Eating + resting | Small passive recovery when not moving |

---

## Resource System

### ResourceNode

Each `ResourceNode` has:
- `amount: f32` — current harvestable supply
- `max_amount: f32` — cap (default: 100.0)
- `regen_rate: f32` — recovery per second (default: 0.5)

### Consumption

When an agent eats:
1. System checks `DecisionOutput.target` → target `ResourceNode`
2. Calculates consume amount: `min(needs.hunger × consume_rate × dt, resource.amount)`
3. Reduces `resource.amount` by that amount
4. Increases agent's hunger satisfaction proportionally
5. If `resource.amount` → 0: emits `ResourceDepleted` event, marks `is_depleted = true`

### Regeneration

All non-depleted resources regenerate passively each tick:
```rust
resource.amount = (resource.amount + regen_rate × dt).min(max_amount);
```

A depleted node regenerates back to threshold (default: 20% of max) then emits `ResourceReplenished` and becomes harvestable again.

---

## Spatial System

The spatial system provides efficient proximity queries without checking every entity pair every tick.

### Grid Partitioning

The world is divided into a uniform grid. Cell size = `SimulationConfig.spatial_grid_cell_size` (default: 10.0 units).

```rust
// Grid cell coordinate
pub struct GridCell(i32, i32);

// Grid state (Resource)
pub struct SpatialGrid {
    cells: HashMap<GridCell, Vec<Entity>>,
    cell_size: f32,
}

impl SpatialGrid {
    pub fn entities_in_radius(&self, pos: Vec3, radius: f32) -> Vec<Entity>;
    pub fn nearest_of_type<F>(&self, pos: Vec3, radius: f32, filter: F) -> Option<Entity>
    where F: Fn(Entity) -> bool;
}
```

### Update Strategy

`spatial_grid_update_system` runs every `FixedUpdate` Pre phase:
1. Clear all cells
2. Re-insert every entity with a `Collider` component into its cell
3. Also insert into neighboring cells if entity straddles a boundary

Cost: O(n) per tick where n = entity count. Acceptable for expected scale (< 500 entities).

---

## Simulation Determinism

The simulation is **fully deterministic** given the same seed. This is critical for the replay system.

### Rules for Determinism

1. **No `f32` from system clock** — all randomness goes through `SimRng`, seeded from `SimulationConfig.seed`
2. **No parallel iteration with mutable state** — mutation happens in single-threaded apply systems
3. **Fixed timestep** — simulation runs on `FixedUpdate` (default 60hz), not frame rate
4. **Stable entity ordering** — spawn order is deterministic; agent IDs are sequential

### SimRng

```rust
// Single RNG instance, shared resource
#[derive(Resource)]
pub struct SimRng(SmallRng);  // rand::rngs::SmallRng — deterministic, fast

impl SimRng {
    pub fn from_seed(seed: u64) -> Self { ... }
    pub fn next_f32(&mut self) -> f32 { ... }
    pub fn next_in_range(&mut self, min: f32, max: f32) -> f32 { ... }
    pub fn next_vec3_in_bounds(&mut self, bounds: Vec2) -> Vec3 { ... }
}
```

`SimRng` is consumed exclusively in simulation systems. No system outside `src/simulation/` and `src/ai/` may call it.

---

## Systemic Pressures

These are the "pressure valves" that generate emergent macro dynamics:

### Resource Scarcity

- Total resource capacity is finite
- Population growth (if implemented) competes for fixed resource
- Result: boom-bust cycles, local extinction, migration

### Need Cascade

- An agent that can't eat will have fatigue accelerate (energy penalty)
- This compounds — a starving agent becomes exhausted faster
- Result: single resource failure can spiral quickly

### Spatial Competition

- Multiple agents targeting the same `ResourceNode` will drain it faster
- First-come-first-served with no coordination
- Result: cluster → deplete → scatter pattern

### Configurable Global Multipliers

Scenarios can set runtime multipliers on:
- `global_decay_multiplier` — make needs decay faster/slower
- `global_regen_multiplier` — make resources recover faster/slower
- `global_spawn_rate` — add agents over time

These multipliers are the primary lever for scenario design.
