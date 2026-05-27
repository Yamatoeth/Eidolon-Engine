# Architecture

> Layered architecture of emergent-sim: module boundaries, data flow, and design principles.

---

## Design Principles

1. **Strict layer separation** — no upward dependencies (simulation never imports engine)
2. **Event-driven communication** — systems communicate through typed Bevy events, not direct calls
3. **No global mutable state** — world state lives in ECS resources and components
4. **Data-driven configuration** — behaviors parameterized via assets, not hardcoded
5. **Testability at every layer** — simulation layer is fully testable without rendering

---

## Layer Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Scenario Layer                          │
│         (world configs, presets, simulation scripts)        │
├─────────────────────────────────────────────────────────────┤
│                   Observability Layer                       │
│      (inspector, event timeline, replay, overlays)          │
├──────────────────────────┬──────────────────────────────────┤
│      Simulation Layer    │          AI Layer                │
│  (world, agents, rules,  │  (utility scoring, decisions,    │
│   resources, events)     │   behavior trees)                │
├──────────────────────────┴──────────────────────────────────┤
│                      Engine Layer                           │
│         (rendering, input, camera, ECS loop, time)          │
└─────────────────────────────────────────────────────────────┘
```

---

## Layer Definitions

### Engine Layer (`src/engine/`)

Responsible for all technical runtime concerns. Has **no knowledge** of simulation semantics.

Modules:
- `render` — Bevy 3D rendering setup, materials, meshes
- `input` — raw input handling, mapped to engine-level actions
- `camera` — orbit camera, free camera, focus camera
- `time` — fixed timestep management, simulation clock
- `window` — window setup, resolution, vsync

**Allowed imports:** Bevy core, engine-internal modules only.

**Exports:** `EnginePlugin`, timing resources, input events.

---

### Simulation Layer (`src/simulation/`)

Responsible for all world logic. Must be **renderable independently** (headless capable).

Modules:
- `world` — world initialization, zone definitions, global state
- `agent` — agent lifecycle, needs degradation, state transitions
- `resource` — resource nodes, spawn/depletion/regeneration
- `spatial` — spatial grid, proximity queries, chunk management
- `events` — typed simulation events, event queue
- `rules` — global simulation constraints and pressures

**Allowed imports:** Bevy ECS, `ai` layer (for decision results), simulation-internal modules.

**Exports:** `SimulationPlugin`, all component types, all event types.

---

### AI Layer (`src/ai/`)

Responsible for agent decision-making. Consumes simulation state, produces decisions.

Modules:
- `utility` — utility function scoring system
- `actions` — action definitions and scoring functions
- `decision` — decision pipeline (score → select → execute)
- `memory` — optional lightweight agent memory

**Allowed imports:** simulation components (read-only), Bevy ECS.

**Exports:** `AIPlugin`, `DecisionOutput` component.

---

### Observability Layer (`src/observability/`)

Responsible for all debug and inspection tooling. Reads ECS state but **never mutates simulation**.

Modules:
- `inspector` — live entity/component browser (egui)
- `timeline` — event history and visualization
- `replay` — simulation recording and playback
- `overlays` — 3D in-world debug drawing

**Allowed imports:** all layers (read access), egui, Bevy gizmos.

**Exports:** `ObservabilityPlugin`, configurable via `ObservabilityConfig` resource.

---

### Scenario Layer (`src/scenarios/`)

Responsible for simulation configurations. Purely data + setup code.

Modules:
- `loader` — loads scenario definitions from JSON assets
- `presets` — hardcoded scenario presets for quick launch
- `builder` — world builder API for programmatic scenarios

**Allowed imports:** simulation layer, Bevy asset system.

**Exports:** `ScenarioPlugin`, `ScenarioConfig`.

---

## Data Flow

```
Input Events
    │
    ▼
Engine Layer
    │  (camera commands, pause/resume, scenario triggers)
    ▼
Simulation Layer ◄──── Scenario Layer (initial config)
    │  (AgentMoved, ResourceDepleted, NeedChanged...)
    ▼
AI Layer
    │  (reads agent state, writes DecisionOutput)
    ▼
Simulation Layer (applies decisions)
    │  (all simulation events)
    ▼
Observability Layer (passive reads, no writes to sim)
    │
    ▼
Engine Layer (renders updated state)
```

---

## Plugin Architecture

Each layer is a Bevy `Plugin`. The main `App` only assembles plugins:

```rust
// main.rs
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EnginePlugin)
        .add_plugins(SimulationPlugin)
        .add_plugins(AIPlugin)
        .add_plugins(ObservabilityPlugin)
        .add_plugins(ScenarioPlugin)
        .run();
}
```

Each plugin owns its systems, resources, and component registrations. Cross-plugin communication happens exclusively through events and shared component types.

---

## System Execution Order

Systems are organized into explicit `SystemSet`s to guarantee ordering within the fixed-timestep loop:

```
FixedUpdate schedule:
┌─────────────────────┐
│  InputProcessing    │  read raw input → simulation commands
├─────────────────────┤
│  SimulationPre      │  needs degradation, resource regeneration
├─────────────────────┤
│  AIDecision         │  read state → score → write DecisionOutput
├─────────────────────┤
│  SimulationApply    │  apply decisions, move agents, consume resources
├─────────────────────┤
│  EventProcessing    │  process simulation events queue
├─────────────────────┤
│  SimulationPost     │  cleanup, dead entity removal, metrics update
└─────────────────────┘

Update schedule (frame-rate):
┌─────────────────────┐
│  RenderSync         │  sync simulation state → render components
├─────────────────────┤
│  ObservabilityDraw  │  draw overlays, update inspector UI
└─────────────────────┘
```

---

## Dependency Rules (enforced via module structure)

```
engine      → (no simulation imports)
simulation  → (no engine imports)
ai          → simulation (read-only components only)
observability → all (read-only)
scenarios   → simulation
main        → all plugins
```

Violations of these rules are a **blocking issue** in code review.

---

## Configuration Philosophy

- World size, agent count, resource density → `SimulationConfig` resource (loaded from asset)
- AI utility weights → `AIConfig` resource (loaded from asset, hot-reloadable)
- Observability features → `ObservabilityConfig` resource (runtime toggle)
- Scenario parameters → JSON asset files in `assets/scenarios/`

No simulation-relevant constants should be hardcoded in system logic.
