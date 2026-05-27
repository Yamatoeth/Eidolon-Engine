# Scenarios

> Predefined simulation configurations for demonstrating specific emergent dynamics.

---

## Overview

Scenarios are **preset world configurations** that initialize the simulation in specific starting conditions. They are not gameplay levels — they are demonstration setups designed to showcase particular emergent behaviors or stress test the simulation engine.

A scenario defines:
- Initial world layout (zones, resource positions)
- Agent count and starting positions
- Global parameter multipliers
- Optional time-based event triggers

---

## Scenario Format

Scenarios are defined as `.ron` files in `assets/scenarios/`:

```rust
// assets/scenarios/equilibrium.ron
ScenarioConfig {
    name: "Stable Equilibrium",
    description: "Resources and agents balanced. Demonstrates stable coexistence.",
    seed: 42,
    world_size: (100.0, 100.0),

    agents: AgentSpawnConfig {
        count: 20,
        distribution: Uniform,  // or Clustered, Zoned
    },

    zones: [
        ZoneConfig { kind: Resource, center: (20.0, 0.0, 20.0), radius: 15.0 },
        ZoneConfig { kind: Resource, center: (80.0, 0.0, 80.0), radius: 15.0 },
        ZoneConfig { kind: Rest,     center: (50.0, 0.0, 20.0), radius: 10.0 },
        ZoneConfig { kind: Rest,     center: (50.0, 0.0, 80.0), radius: 10.0 },
        ZoneConfig { kind: Resource, center: (50.0, 0.0, 50.0), radius: 12.0 },
    ],

    resources: ResourceSpawnConfig {
        nodes_per_resource_zone: 3,
        initial_amount_fraction: 1.0,  // 100% full
        regen_rate: 0.5,
    },

    sim_overrides: SimulationOverrides {
        global_decay_multiplier: 1.0,
        global_regen_multiplier: 1.0,
    },

    events: [],  // no timed events
}
```

---

## Built-in Scenarios

### 1. Stable Equilibrium

**File:** `equilibrium.ron`

**Purpose:** Baseline demonstration of agents finding and maintaining balance between food and rest.

**Configuration:**
- 20 agents
- 3 resource zones, 2 rest zones
- Resources at full capacity, normal regen
- Normal decay rates

**Expected emergent dynamics:**
- Agents distribute across resource zones
- Cyclical eat → rest → explore patterns
- Stable population (no deaths)
- Resource levels oscillate but never fully deplete

**Use when:** Demonstrating the basic utility AI, showing normal operation.

---

### 2. Resource Scarcity (Collapse)

**File:** `scarcity.ron`

**Purpose:** Stress test — demonstrate cascade failure when resources are insufficient.

**Configuration:**
- 30 agents (overpopulated)
- 2 small resource zones, 1 rest zone
- Resources start at 30% capacity
- Low regen rate (0.1/s)
- High decay multiplier (1.8×)

**Expected emergent dynamics:**
- Rapid clustering near resource zones
- Competition → faster depletion
- First agents to starve die near t=60s
- Population collapse cascade
- Surviving agents (if any) cluster at remaining resource

**Use when:** Demonstrating systemic pressure, competition, and cascade failure.

**Timed events:**
```ron
events: [
    // Partial resource replenishment at t=90s to show recovery attempt
    SimEvent { at: 90.0, kind: SetRegenMultiplier(2.0) },
]
```

---

### 3. Controlled Overpopulation

**File:** `overpopulation.ron`

**Purpose:** Demonstrate emergent migration and spatial competition at high density.

**Configuration:**
- Starts with 10 agents
- New agents spawned every 5s (via timed events) up to 50
- 4 resource zones evenly distributed
- Normal regen, moderate decay

**Expected emergent dynamics:**
- Early agents establish territories (soft)
- New agents forced to find unclaimed zones
- Competition increases as population rises
- Resource depletion accelerates after t=60s
- Migration patterns visible in agent trails

**Timed events:**
```ron
events: [
    SimEvent { at: 5.0,  kind: SpawnAgents { count: 5, distribution: Random } },
    SimEvent { at: 10.0, kind: SpawnAgents { count: 5, distribution: Random } },
    // ... continue until cap
    SimEvent { at: 60.0, kind: SetDecayMultiplier(1.3) },  // increase pressure
]
```

---

### 4. Stress Test

**File:** `stress_test.ron`

**Purpose:** Technical benchmark — maximum agent count with instrumentation enabled.

**Configuration:**
- 200 agents
- 8 resource zones
- Minimal rendering detail
- All observability tools active

**Purpose is technical, not behavioral.** Used to:
- Measure FPS and fixed-update timing at high entity count
- Profile spatial grid performance
- Measure observability overhead
- Validate determinism at scale

**Not intended for visual demonstration.**

---

### 5. Single Resource Island

**File:** `island.ron`

**Purpose:** Compact scenario with a single central resource. Demonstrates concentrated competition.

**Configuration:**
- 15 agents
- 1 central resource zone (large)
- 2 distant rest zones
- High decay rate (2×)
- High regen rate (1.5×) — resource replenishes quickly but depletes under heavy use

**Expected emergent dynamics:**
- All agents converge on center
- Resource oscillates rapidly
- Agents must travel to rest zones and return — creating visible "commuting" patterns
- Rich territory for observability demo (many events, clear causal chains)

---

## Adding a Custom Scenario

1. Create `assets/scenarios/my_scenario.ron` using the schema above
2. Register the file name in `assets/scenarios/index.ron`
3. The `ScenarioPlugin` auto-discovers registered scenarios at startup
4. Accessible from the scenario selector in the pause menu

No code changes needed for purely data-driven scenarios. Scenarios requiring custom event logic use the `ScenarioEventKind::Custom(String)` variant with a Rust handler registered in `src/scenarios/handlers.rs`.

---

## Scenario Selector UI

```
┌─ Scenarios ─────────────────────────────────────────────────────┐
│                                                                  │
│  ● Stable Equilibrium                            [LOAD]          │
│    Resources and agents balanced. ~20 agents.                   │
│                                                                  │
│  ○ Resource Scarcity                             [LOAD]          │
│    Collapse demo. 30 agents, low resources.                     │
│                                                                  │
│  ○ Controlled Overpopulation                     [LOAD]          │
│    Migration demo. Population grows over time.                  │
│                                                                  │
│  ○ Stress Test                                   [LOAD]          │
│    200 agents. Performance benchmark.                           │
│                                                                  │
│  ○ Single Resource Island                        [LOAD]          │
│    Concentrated competition demo.                               │
│                                                                  │
│  Seed: [42_________]  [Randomize]                               │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

Scenarios can be loaded at any time. Loading a scenario:
1. Pauses simulation
2. Despawns all current entities
3. Resets `SimulationTime` and `SimRng`
4. Loads new config
5. Re-spawns world
6. Resumes simulation
